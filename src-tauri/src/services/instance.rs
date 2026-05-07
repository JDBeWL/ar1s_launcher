use crate::errors::LauncherError;
use crate::models::{DownloadJob, InstanceInfo, LaunchOptions};
use crate::services::{config, download, launcher, loaders::{self, LoaderType}};
use crate::utils::file_utils::{self, validate_instance_name_or_error, validate_instance_name, InstanceNameValidation};
use log::{info, warn};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Window};

#[derive(Clone, Serialize)]
struct InstallProgress {
    progress: u8,
    message: String,
    indeterminate: bool,
}

/// 辅助函数：获取游戏目录和版本目录
fn get_dirs() -> Result<(PathBuf, PathBuf), LauncherError> {
    let config = config::load_config()?;
    let game_dir = PathBuf::from(&config.game_dir);
    let versions_dir = game_dir.join("versions");
    Ok((game_dir, versions_dir))
}

/// 检查实例名称是否可用（验证格式并检查是否已存在）
pub fn check_instance_name_available(name: &str) -> InstanceNameValidation {
    // 首先验证名称格式
    let validation = validate_instance_name(name);
    if !validation.is_valid {
        return validation;
    }
    
    // 然后检查实例是否已存在
    if let Ok((_, versions_dir)) = get_dirs() {
        let instance_dir = versions_dir.join(name);
        if instance_dir.exists() {
            return InstanceNameValidation {
                is_valid: false,
                error_message: Some(format!("名为 '{}' 的实例已存在，请使用其他名称", name)),
            };
        }
    }
    
    InstanceNameValidation {
        is_valid: true,
        error_message: None,
    }
}

/// 创建新实例
pub async fn create_instance(
    new_instance_name: String,
    base_version_id: String,
    loader: Option<LoaderType>,
    window: &Window,
) -> Result<(), LauncherError> {
    // 验证实例名称
    validate_instance_name_or_error(&new_instance_name)?;
    
    let (game_dir, versions_dir) = get_dirs()?;
    let source_dir = versions_dir.join(&base_version_id);
    let dest_dir = versions_dir.join(&new_instance_name);

    let send_progress = |progress: u8, message: &str, indeterminate: bool| {
        let _ = window.emit(
            "instance-install-progress",
            InstallProgress {
                progress,
                message: message.to_string(),
                indeterminate,
            },
        );
    };

    if dest_dir.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 已存在", new_instance_name)));
    }

    send_progress(5, "检查基础版本...", false);

    if !source_dir.exists() {
        send_progress(10, "下载基础版本...", true);
        let config = config::load_config()?;
        download::process_and_download_version(
            base_version_id.clone(),
            config.download_mirror.clone(),
            window,
        ).await?;

        if !source_dir.exists() {
            return Err(LauncherError::Custom(format!("基础版本 '{}' 下载后仍未找到", base_version_id)));
        }
    }

    let cleanup = || {
        warn!("安装失败，正在清理实例目录: {}", dest_dir.display());
        file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
    };

    send_progress(30, "复制基础文件...", false);

    if let Err(e) = file_utils::copy_dir_all(&source_dir, &dest_dir) {
        cleanup();
        return Err(e.into());
    }

    send_progress(40, "配置实例...", false);

    let old_json_path = dest_dir.join(format!("{}.json", base_version_id));
    let new_json_path = dest_dir.join(format!("{}.json", new_instance_name));
    let old_jar_path = dest_dir.join(format!("{}.jar", base_version_id));
    let new_jar_path = dest_dir.join(format!("{}.jar", new_instance_name));

    if let Err(e) = fs::rename(&old_json_path, &new_json_path) {
        cleanup();
        return Err(LauncherError::Custom(format!("重命名 JSON 失败: {}", e)));
    }

    if old_jar_path.exists() {
        if let Err(e) = fs::rename(&old_jar_path, &new_jar_path) {
            cleanup();
            return Err(LauncherError::Custom(format!("重命名 JAR 失败: {}", e)));
        }
    }

    let update_json_id = || -> Result<(), LauncherError> {
        let json_str = fs::read_to_string(&new_json_path)?;
        let mut json: Value = serde_json::from_str(&json_str)
            .map_err(|e| LauncherError::Custom(format!("解析 JSON 失败: {}", e)))?;
        json["id"] = Value::String(new_instance_name.clone());
        fs::write(&new_json_path, serde_json::to_string_pretty(&json)?)?;
        Ok(())
    };

    if let Err(e) = update_json_id() {
        cleanup();
        return Err(e);
    }

    if let Some(ref loader_type) = loader {
        send_progress(60, &format!("安装 {} 加载器...", loader_type.name()), true);
        
        if let Err(e) = loaders::install_loader(loader_type, &new_instance_name, &game_dir).await {
            cleanup();
            return Err(e);
        }

        // 对于 Forge，需要合并配置
        if let LoaderType::Forge { mc_version, loader_version } = loader_type {
            let forge_id_prefix = format!("{}-forge", mc_version);
            let forge_id_exact = format!("{}-forge-{}", mc_version, loader_version);
            
            let found_forge_id = fs::read_dir(&versions_dir)
                .ok()
                .and_then(|entries| {
                    entries.flatten()
                        .filter_map(|e| e.file_name().to_str().map(String::from))
                        .find(|name| name == &forge_id_exact || name.starts_with(&forge_id_prefix))
                });

            if let Some(fid) = found_forge_id {
                let forge_json_path = versions_dir.join(&fid).join(format!("{}.json", fid));
                let base_json_path = versions_dir.join(&base_version_id).join(format!("{}.json", base_version_id));

                if forge_json_path.exists() && base_json_path.exists() {
                    send_progress(70, "合并配置并补全依赖...", true);
                    
                    if let Err(e) = merge_and_complete_instance(
                        &new_instance_name,
                        &new_json_path,
                        &base_json_path,
                        &forge_json_path,
                        &game_dir,
                        window
                    ).await {
                        cleanup();
                        return Err(e);
                    }

                    let forge_dir = versions_dir.join(&fid);
                    if forge_dir.exists() && forge_dir != dest_dir {
                        let _ = fs::remove_dir_all(forge_dir);
                    }
                } else {
                    warn!("未找到 Forge 或 基础版本的 JSON 文件，跳过合并");
                }
            } else {
                warn!("未找到安装后的 Forge 目录");
            }
        }
    }

    send_progress(100, "实例创建完成！", false);

    // 触发游戏目录变更事件，通知前端刷新版本列表
    if let Ok(game_dir_str) = config::get_game_dir() {
        let _ = window.emit("game-dir-changed", &game_dir_str);
    }

    Ok(())
}

/// 获取实例列表（使用 spawn_blocking 避免阻塞异步运行时）
pub async fn get_instances() -> Result<Vec<InstanceInfo>, LauncherError> {
    let (_, versions_dir) = get_dirs()?;
    
    // 将 CPU 密集型的文件系统操作和 JSON 解析移到阻塞线程池
    let instances = tokio::task::spawn_blocking(move || {
        get_instances_sync(&versions_dir)
    }).await.map_err(|e| LauncherError::Custom(format!("获取实例列表失败: {}", e)))??;
    
    Ok(instances)
}

/// 从版本 JSON 的 arguments 中提取 FML 参数
///
/// Forge 版本 JSON 的 arguments.game 中包含：
///   "--fml.forgeVersion", "36.2.39",
///   "--fml.mcVersion", "1.16.5",
///   "--fml.forgeGroup", "net.minecraftforge",
///   "--fml.mcpVersion", "20210115.111550"
///
/// 也支持 NeoForge 的 "--fml.neoForgeVersion" 格式
fn parse_fml_arguments(json_value: &Value) -> (Option<String>, Option<String>, Option<String>) {
    let args = match json_value.get("arguments").and_then(|a| a.get("game")).and_then(|g| g.as_array()) {
        Some(arr) => arr,
        None => {
            let raw_args = match json_value["minecraftArguments"].as_str() {
                Some(s) => s,
                None => return (None, None, None),
            };
            let tokens: Vec<&str> = raw_args.split_whitespace().collect();
            let mut forge_ver: Option<String> = None;
            let mut mc_ver: Option<String> = None;
            let mut is_neoforge = false;

            for (i, token) in tokens.iter().enumerate() {
                match *token {
                    "--fml.forgeVersion" => {
                        if i + 1 < tokens.len() {
                            forge_ver = Some(tokens[i + 1].to_string());
                        }
                    }
                    "--fml.neoForgeVersion" => {
                        if i + 1 < tokens.len() {
                            forge_ver = Some(tokens[i + 1].to_string());
                            is_neoforge = true;
                        }
                    }
                    "--fml.mcVersion" => {
                        if i + 1 < tokens.len() {
                            mc_ver = Some(tokens[i + 1].to_string());
                        }
                    }
                    _ => {}
                }
            }

            if forge_ver.is_some() || mc_ver.is_some() {
                let lt = if is_neoforge { "NeoForge" } else { "Forge" };
                return (Some(lt.to_string()), forge_ver, mc_ver);
            }
            return (None, None, None);
        }
    };

    let mut forge_ver: Option<String> = None;
    let mut mc_ver: Option<String> = None;
    let mut is_neoforge = false;

    for (i, arg) in args.iter().enumerate() {
        let token = match arg.as_str() {
            Some(s) => s,
            None => continue,
        };

        match token {
            "--fml.forgeVersion" => {
                if let Some(next) = args.get(i + 1).and_then(|v| v.as_str()) {
                    forge_ver = Some(next.to_string());
                }
            }
            "--fml.neoForgeVersion" => {
                if let Some(next) = args.get(i + 1).and_then(|v| v.as_str()) {
                    forge_ver = Some(next.to_string());
                    is_neoforge = true;
                }
            }
            "--fml.mcVersion" => {
                if let Some(next) = args.get(i + 1).and_then(|v| v.as_str()) {
                    mc_ver = Some(next.to_string());
                }
            }
            _ => {}
        }
    }

    if forge_ver.is_some() || mc_ver.is_some() {
        let lt = if is_neoforge { "NeoForge" } else { "Forge" };
        return (Some(lt.to_string()), forge_ver, mc_ver);
    }

    (None, None, None)
}

/// 从版本 JSON 的 patches 数组中提取加载器版本和游戏版本
///
/// Fabric/Quilt 的版本 JSON 使用 patches 结构：
///   "patches": [
///     { "id": "game", "version": "1.20.4", "priority": 0 },
///     { "id": "fabric", "version": "0.15.11" },
///     { "id": "quilt", "version": "0.19.2" }
///   ]
fn parse_patches(json_value: &Value) -> (Option<String>, Option<String>, Option<String>) {
    let patches = match json_value["patches"].as_array() {
        Some(arr) => arr,
        None => return (None, None, None),
    };

    let mut game_version: Option<String> = None;
    let mut loader_type: Option<String> = None;
    let mut loader_version: Option<String> = None;

    for patch in patches {
        let id = match patch["id"].as_str() {
            Some(s) => s,
            None => continue,
        };
        let version = patch["version"].as_str().map(String::from);

        match id {
            "game" => {
                game_version = version;
            }
            "fabric" => {
                loader_type = Some("Fabric".to_string());
                loader_version = version;
            }
            "quilt" => {
                loader_type = Some("Quilt".to_string());
                loader_version = version;
            }
            "forge" => {
                loader_type = Some("Forge".to_string());
                loader_version = version;
            }
            "neoforge" => {
                loader_type = Some("NeoForge".to_string());
                loader_version = version;
            }
            _ => {}
        }
    }

    if loader_type.is_some() || game_version.is_some() {
        return (loader_type, loader_version, game_version);
    }

    (None, None, None)
}

/// 从版本 JSON 的 libraries 数组中提取加载器类型和版本
///
/// 通过扫描 Maven 坐标识别加载器：
/// - Forge: `net.minecraftforge:forge:1.16.5-36.2.39`
/// - Fabric: `net.fabricmc:fabric-loader:0.15.11`
/// - Quilt: `org.quiltmc:quilt-loader:0.19.2`
/// - NeoForge: `net.neoforged:neoforge:20.1.2`
fn parse_loader_from_libraries(json_value: &Value) -> (Option<String>, Option<String>) {
    let libs = match json_value["libraries"].as_array() {
        Some(arr) => arr,
        None => return (None, None),
    };

    for lib in libs {
        let name = match lib["name"].as_str() {
            Some(n) => n,
            None => continue,
        };

        let parts: Vec<&str> = name.split(':').collect();
        if parts.len() < 3 {
            continue;
        }

        let group = parts[0];
        let artifact = parts[1];
        let version = parts[2];

        if group == "net.minecraftforge" && artifact == "forge" {
            let loader_ver = version.split('-').last().unwrap_or(version).to_string();
            return (Some("Forge".to_string()), Some(loader_ver));
        }

        if group == "net.minecraftforge" && artifact == "fmlloader" {
            let loader_ver = version.split('-').last().unwrap_or(version).to_string();
            return (Some("Forge".to_string()), Some(loader_ver));
        }

        if group == "net.fabricmc" && artifact == "fabric-loader" {
            return (Some("Fabric".to_string()), Some(version.to_string()));
        }

        if group == "org.quiltmc" && artifact == "quilt-loader" {
            return (Some("Quilt".to_string()), Some(version.to_string()));
        }

        if group == "net.neoforged" && (artifact == "neoforge" || artifact == "fml") {
            let loader_ver = version.split('-').last().unwrap_or(version).to_string();
            return (Some("NeoForge".to_string()), Some(loader_ver));
        }
    }

    (None, None)
}

/// 读取 instance.json 获取整合包元数据
fn read_instance_metadata(instance_dir: &Path) -> Option<(String, Option<String>)> {
    let meta_path = instance_dir.join("instance.json");
    if !meta_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&meta_path).ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;

    let mc_version = json["minecraft"].as_str().map(String::from)?;
    let loader = json["loader"].as_str().map(String::from);

    Some((mc_version, loader))
}

/// 通过继承链查找真实的 Minecraft 版本
///
/// 优先级：
/// 1. 如果 `inheritsFrom` 本身就是合法的 MC 版本号（如 "1.16.5"），直接使用
/// 2. 否则尝试读取父版本 JSON 继续递归（处理多层继承）
/// 3. 都失败时回退到 version_id
fn find_real_minecraft_version(
    version_id: &str,
    json_value: &Value,
    versions_dir: &Path
) -> Option<String> {
    if let Some(inherits) = json_value["inheritsFrom"].as_str() {
        if crate::utils::minecraft::parse_mc_version(inherits).is_some() {
            return Some(inherits.to_string());
        }
        let parent_json_path = versions_dir.join(inherits).join(format!("{}.json", inherits));
        if parent_json_path.exists() {
            if let Ok(parent_content) = fs::read_to_string(&parent_json_path) {
                if let Ok(parent_json) = serde_json::from_str::<Value>(&parent_content) {
                    return find_real_minecraft_version(inherits, &parent_json, versions_dir);
                }
            }
        }
        Some(inherits.to_string())
    } else {
        Some(version_id.to_string())
    }
}

/// 同步获取实例列表（在阻塞线程池中执行）
fn get_instances_sync(versions_dir: &Path) -> Result<Vec<InstanceInfo>, LauncherError> {
    let mut instances = Vec::new();

    if !versions_dir.exists() {
        return Ok(instances);
    }

    if let Ok(entries) = fs::read_dir(versions_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let path = entry.path();
                    let json_path = path.join(format!("{}.json", name));

                    if json_path.exists() {
                        let json_content = fs::read_to_string(&json_path).ok();
                        let json_value = json_content
                            .as_ref()
                            .and_then(|c| serde_json::from_str::<Value>(c).ok());

                        let version_id = json_value
                            .as_ref()
                            .and_then(|v| v["id"].as_str().map(String::from))
                            .unwrap_or_else(|| name.clone());

                        let mod_count = count_mods(&path);

                        let main_class = json_value
                            .as_ref()
                            .and_then(|v| v["mainClass"].as_str())
                            .unwrap_or("");

                        let mut loader_type: Option<String> = None;
                        let mut loader_version: Option<String> = None;
                        let mut game_version: Option<String> = None;

                        if let Some(ref json) = json_value {
                            if let (Some(lt), fv, mv) = parse_fml_arguments(json) {
                                loader_type = Some(lt);
                                loader_version = fv;
                                game_version = mv;
                            }
                        }

                        if let Some(ref json) = json_value {
                            let (plt, plv, pgv) = parse_patches(json);
                            if plt.is_some() {
                                if loader_type.is_none() {
                                    loader_type = plt;
                                }
                                if loader_version.is_none() {
                                    loader_version = plv;
                                }
                                if game_version.is_none() {
                                    game_version = pgv;
                                }
                            }
                        }

                        if let Some((meta_mc, meta_loader)) = read_instance_metadata(&path) {
                            if game_version.is_none() {
                                game_version = Some(meta_mc);
                            }
                            if loader_type.is_none() {
                                if let Some(lt) = meta_loader {
                                    loader_type = Some(match lt.as_str() {
                                        "forge" => "Forge".to_string(),
                                        "fabric" => "Fabric".to_string(),
                                        "quilt" => "Quilt".to_string(),
                                        "neoforge" => "NeoForge".to_string(),
                                        other => {
                                            let mut c = other.chars();
                                            match c.next() {
                                                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                                                None => other.to_string(),
                                            }
                                        }
                                    });
                                }
                            }
                        }

                        if let Some(ref json) = json_value {
                            if let (Some(lt), Some(lv)) = parse_loader_from_libraries(json) {
                                if loader_type.is_none() {
                                    loader_type = Some(lt);
                                }
                                if loader_version.is_none() {
                                    loader_version = Some(lv);
                                }
                            }
                        }

                        if loader_type.is_none() {
                            if main_class.contains("forge") || main_class.contains("LaunchWrapper") || main_class.contains("modlauncher") {
                                loader_type = Some("Forge".to_string());
                            } else if main_class.contains("fabricmc") || (main_class.contains("knot") && main_class.to_lowercase().contains("fabric")) {
                                loader_type = Some("Fabric".to_string());
                            } else if main_class.contains("quiltmc") || (main_class.contains("knot") && main_class.to_lowercase().contains("quilt")) {
                                loader_type = Some("Quilt".to_string());
                            } else if main_class.contains("neoforge") {
                                loader_type = Some("NeoForge".to_string());
                            }
                        }

                        if game_version.is_none() {
                            if let Some(json) = json_value.as_ref() {
                                game_version = find_real_minecraft_version(&version_id, json, versions_dir);
                            } else {
                                game_version = Some(version_id.clone());
                            }
                        }

                        if loader_type.is_none() {
                            if json_value.as_ref().and_then(|v| v["inheritsFrom"].as_str()).is_some() {
                                loader_type = Some("Unknown".to_string());
                            } else if mod_count.unwrap_or(0) > 0 {
                                loader_type = Some("Modded".to_string());
                            } else {
                                loader_type = Some("None".to_string());
                            }
                        }

                        let created = entry.metadata()
                            .and_then(|m| m.created())
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs().to_string());

                        let config_last_played = config::get_instance_last_played(&name);
                        let json_last_played = json_value
                            .as_ref()
                            .and_then(|v| v["lastPlayed"].as_i64());
                        let last_played = match (config_last_played, json_last_played) {
                            (Some(c), Some(j)) => Some(c.max(j)),
                            (Some(c), None) => Some(c),
                            (None, Some(j)) => Some(j),
                            (None, None) => None,
                        };

                        instances.push(InstanceInfo {
                            id: name.clone(),
                            name: name.clone(),
                            version: version_id,
                            path: path.to_string_lossy().to_string(),
                            created_time: created,
                            loader_type,
                            loader_version,
                            game_version,
                            last_played,
                            mod_count,
                        });
                    }
                }
            }
        }
    }
    Ok(instances)
}

fn count_mods(instance_path: &Path) -> Option<u32> {
    let mods_dir = instance_path.join("mods");
    if !mods_dir.is_dir() {
        return None;
    }
    let count = fs::read_dir(&mods_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "jar")
                .unwrap_or(false)
        })
        .count() as u32;
    if count == 0 { None } else { Some(count) }
}

/// 删除实例
pub async fn delete_instance(instance_name: String) -> Result<(), LauncherError> {
    let (_, versions_dir) = get_dirs()?;
    let instance_dir = versions_dir.join(&instance_name);

    if !instance_dir.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 不存在", instance_name)));
    }

    fs::remove_dir_all(&instance_dir)
        .map_err(|e| LauncherError::Custom(format!("删除实例失败: {}", e)))?;
    
    // 删除上次启动时间记录
    let _ = config::remove_instance_last_played(&instance_name);
    
    info!("实例 {} 已删除", instance_name);
    Ok(())
}

/// 重命名实例
pub async fn rename_instance(old_name: String, new_name: String) -> Result<(), LauncherError> {
    // 验证新实例名称
    validate_instance_name_or_error(&new_name)?;
    
    let (_, versions_dir) = get_dirs()?;
    let old_dir = versions_dir.join(&old_name);
    let new_dir = versions_dir.join(&new_name);

    if !old_dir.exists() {
        return Err(LauncherError::Custom(format!("原实例 '{}' 不存在", old_name)));
    }
    if new_dir.exists() {
        return Err(LauncherError::Custom(format!("目标实例名 '{}' 已存在", new_name)));
    }

    fs::rename(&old_dir, &new_dir)
        .map_err(|e| LauncherError::Custom(format!("重命名目录失败: {}", e)))?;

    // 重命名内部文件
    let _ = fs::rename(
        new_dir.join(format!("{}.json", old_name)),
        new_dir.join(format!("{}.json", new_name)),
    );
    let _ = fs::rename(
        new_dir.join(format!("{}.jar", old_name)),
        new_dir.join(format!("{}.jar", new_name)),
    );

    // 更新 JSON ID
    let json_path = new_dir.join(format!("{}.json", new_name));
    if json_path.exists() {
        let content = fs::read_to_string(&json_path)?;
        if let Ok(mut json) = serde_json::from_str::<Value>(&content) {
            json["id"] = Value::String(new_name.clone());
            fs::write(&json_path, serde_json::to_string_pretty(&json)?)?;
        }
    }

    // 重命名上次启动时间记录
    let _ = config::rename_instance_last_played(&old_name, &new_name);

    Ok(())
}

/// 打开实例文件夹
pub async fn open_instance_folder(instance_name: String) -> Result<(), LauncherError> {
    let (_, versions_dir) = get_dirs()?;
    let instance_dir = versions_dir.join(&instance_name);

    if !instance_dir.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 不存在", instance_name)));
    }

    opener::open(&instance_dir)
        .map_err(|e| LauncherError::Custom(format!("无法打开文件夹: {}", e)))?;

    Ok(())
}

/// 启动实例
pub async fn launch_instance(instance_name: String, override_java_path: Option<String>, window: Window) -> Result<(), LauncherError> {
    let config = config::load_config()?;
    let (_, versions_dir) = get_dirs()?;
    let instance_dir = versions_dir.join(&instance_name);

    let json_path = instance_dir.join(format!("{}.json", instance_name));
    if !json_path.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 的配置文件不存在", instance_name)));
    }

    let json_content = fs::read_to_string(&json_path)?;
    let json: Value = serde_json::from_str(&json_content)
        .map_err(|e| LauncherError::Custom(format!("实例 '{}' 的配置文件格式无效: {}", instance_name, e)))?;

    if !json.get("id").and_then(|v| v.as_str()).is_some() {
        return Err(LauncherError::Custom(format!("实例 '{}' 的配置文件缺少 'id' 字段", instance_name)));
    }
    if !json.get("mainClass").and_then(|v| v.as_str()).is_some() {
        return Err(LauncherError::Custom(format!("实例 '{}' 的配置文件缺少 'mainClass' 字段", instance_name)));
    }

    let config_last_played = config.instance_last_played.get(&instance_name).copied();
    if let Some(ts) = config_last_played {
        if let Some(json_played) = json.get("lastPlayed").and_then(|v| v.as_i64()) {
            if json_played != ts {
                warn!("实例 '{}' 的 lastPlayed 不一致 (config: {}, json: {}), 使用 config 值", instance_name, ts, json_played);
            }
        }
    }

    // 注意: update_instance_last_played 已在 launcher::launch_minecraft 中调用，此处无需重复

    let (auth_type, access_token, uuid) = match config.auth_type {
        crate::models::AuthType::Microsoft => {
            let at = "microsoft".to_string();
            let mut token = config.ms_access_token.clone();
            let uid = config.uuid.clone();
            let expires_at = config.ms_expires_at.unwrap_or(0);

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            if now >= expires_at - 60 {
                info!("Microsoft token 已过期或即将过期，正在刷新...");
                if let Some(refresh_token) = config.ms_refresh_token.clone() {
                    match crate::services::microsoft_auth::refresh_and_authenticate(&refresh_token).await {
                        Ok(auth_result) => {
                            let mut cfg = (*config::load_config()?).clone();
                            cfg.ms_access_token = Some(auth_result.access_token.clone());
                            cfg.ms_refresh_token = Some(auth_result.refresh_token.clone());
                            cfg.ms_expires_at = Some(auth_result.expires_at);
                            cfg.username = Some(auth_result.username.clone());
                            cfg.uuid = Some(auth_result.uuid.clone());
                            config::save_config(&cfg)?;

                            token = Some(auth_result.access_token);
                            info!("Microsoft token 刷新成功");
                        }
                        Err(e) => {
                            warn!("Microsoft token 刷新失败: {}, 将使用旧 token 启动", e);
                        }
                    }
                } else {
                    warn!("未找到 refresh_token，无法刷新 Microsoft token");
                }
            }

            (Some(at), token, uid)
        }
        crate::models::AuthType::Offline => (None, None, None),
    };

    let launch_options = LaunchOptions {
        version: instance_name,
        username: config.username.clone().unwrap_or_else(|| "Player".to_string()),
        memory: Some(config.max_memory),
        window_width: config.window_width,
        window_height: config.window_height,
        fullscreen: Some(config.fullscreen),
        auth_type,
        access_token,
        uuid,
        override_java_path,
    };

    launcher::launch_minecraft(launch_options, window).await
}

// --- 下面是合并 JSON 和收集下载任务的私有辅助函数 ---

async fn merge_and_complete_instance(
    instance_id: &str,
    target_json_path: &Path,
    base_json_path: &Path,
    forge_json_path: &Path,
    game_dir: &Path,
    window: &Window,
) -> Result<(), LauncherError> {
    let base_content = fs::read_to_string(base_json_path)?;
    let forge_content = fs::read_to_string(forge_json_path)?;
    
    let base_json: Value = serde_json::from_str(&base_content).map_err(|e| LauncherError::Custom(e.to_string()))?;
    let forge_json: Value = serde_json::from_str(&forge_content).map_err(|e| LauncherError::Custom(e.to_string()))?;

    let mut merged = forge_json.clone();
    merged["id"] = Value::String(instance_id.to_string());

    if merged["mainClass"].is_null() {
        merged["mainClass"] = base_json["mainClass"].clone();
    }

    if merged["arguments"].is_null() {
        if let Some(forge_args) = forge_json["minecraftArguments"].as_str() {
            let args_array: Vec<Value> = forge_args.split_whitespace().map(|s| Value::String(s.to_string())).collect();
            merged["arguments"] = serde_json::json!({ "game": args_array });
        } else if !base_json["arguments"].is_null() {
            merged["arguments"] = base_json["arguments"].clone();
        } else if let Some(base_args) = base_json["minecraftArguments"].as_str() {
            let args_array: Vec<Value> = base_args.split_whitespace().map(|s| Value::String(s.to_string())).collect();
            merged["arguments"] = serde_json::json!({ "game": args_array });
        }
    }

    let mut final_libs = Vec::new();
    let mut seen_libs = HashSet::new();

    if let Some(libs) = forge_json["libraries"].as_array() {
        for lib in libs {
            if let Some(name) = lib["name"].as_str() {
                seen_libs.insert(name.to_string());
            }
            final_libs.push(lib.clone());
        }
    }

    if let Some(libs) = base_json["libraries"].as_array() {
        for lib in libs {
            if let Some(name) = lib["name"].as_str() {
                if !seen_libs.contains(name) {
                    final_libs.push(lib.clone());
                }
            } else {
                final_libs.push(lib.clone());
            }
        }
    }
    merged["libraries"] = Value::Array(final_libs);

    if let Some(obj) = base_json.as_object() {
        for (k, v) in obj {
            if merged[k].is_null() {
                merged[k] = v.clone();
            }
        }
    }

    fs::write(target_json_path, serde_json::to_string_pretty(&merged)?)?;

    let jobs = collect_download_jobs(&merged, game_dir, instance_id)?;
    
    if !jobs.is_empty() {
        let (index_jobs, other_jobs): (Vec<_>, Vec<_>) = jobs.into_iter().partition(|j| {
            j.path.to_string_lossy().contains("indexes")
        });

        if !index_jobs.is_empty() {
            download::download_all_files(index_jobs.clone(), window).await?;
        }

        let mut all_jobs = other_jobs;
        for job in index_jobs {
            if job.path.exists() {
                let content = fs::read_to_string(&job.path)?;
                if let Ok(idx_json) = serde_json::from_str::<Value>(&content) {
                    if let Some(objects) = idx_json["objects"].as_object() {
                        let assets_objects_dir = game_dir.join("assets").join("objects");
                        for obj in objects.values() {
                            if let Some(hash) = obj["hash"].as_str() {
                                let size = obj["size"].as_u64().unwrap_or(0);
                                let prefix = &hash[..2];
                                let path = assets_objects_dir.join(prefix).join(hash);
                                let url = format!("https://resources.download.minecraft.net/{}/{}", prefix, hash);
                                
                                all_jobs.push(DownloadJob {
                                    url,
                                    fallback_url: None,
                                    path,
                                    size,
                                    hash: Some(hash.to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }

        if !all_jobs.is_empty() {
            download::download_all_files(all_jobs, window).await?;
        }
    }

    Ok(())
}

fn collect_download_jobs(
    json: &Value,
    game_dir: &Path,
    instance_id: &str
) -> Result<Vec<DownloadJob>, LauncherError> {
    let mut jobs = Vec::new();
    let libraries_dir = game_dir.join("libraries");
    let assets_indexes_dir = game_dir.join("assets").join("indexes");
    let versions_dir = game_dir.join("versions");

    if let Some(client) = json.get("downloads").and_then(|d| d.get("client")) {
        if let (Some(url), Some(sha1), Some(size)) = (
            client["url"].as_str(),
            client["sha1"].as_str(),
            client["size"].as_u64()
        ) {
            let path = versions_dir.join(instance_id).join(format!("{}.jar", instance_id));
            jobs.push(DownloadJob {
                url: url.to_string(),
                fallback_url: None,
                path,
                size,
                hash: Some(sha1.to_string()),
            });
        }
    }

    if let Some(asset_idx) = json.get("assetIndex") {
        if let (Some(id), Some(url), Some(sha1), Some(size)) = (
            asset_idx["id"].as_str(),
            asset_idx["url"].as_str(),
            asset_idx["sha1"].as_str(),
            asset_idx["size"].as_u64()
        ) {
            let path = assets_indexes_dir.join(format!("{}.json", id));
            jobs.push(DownloadJob {
                url: url.to_string(),
                fallback_url: None,
                path,
                size,
                hash: Some(sha1.to_string()),
            });
        }
    }

    if let Some(libs) = json["libraries"].as_array() {
        for lib in libs {
            let allowed = lib["rules"].as_array().map_or(true, |rules| {
                let current_os = std::env::consts::OS;
                let target_os = if current_os == "macos" { "osx" } else { current_os };
                let mut allow = false;
                for rule in rules {
                    let action = rule["action"].as_str().unwrap_or("allow");
                    let os_match = rule["os"]["name"].as_str().map_or(true, |o| o == target_os);
                    if os_match { allow = action == "allow"; }
                }
                allow
            });

            if !allowed { continue; }

            if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                if let (Some(url), Some(path), Some(sha1), Some(size)) = (
                    artifact["url"].as_str(),
                    artifact["path"].as_str(),
                    artifact["sha1"].as_str(),
                    artifact["size"].as_u64()
                ) {
                    jobs.push(DownloadJob {
                        url: url.to_string(),
                        fallback_url: None,
                        path: libraries_dir.join(path),
                        size,
                        hash: Some(sha1.to_string()),
                    });
                }
            }

            if let Some(classifiers) = lib.get("downloads").and_then(|d| d.get("classifiers")) {
                if let Some(obj) = classifiers.as_object() {
                    let current_os = std::env::consts::OS;
                    for (key, artifact) in obj {
                        if key.contains(current_os) || (current_os == "macos" && key.contains("osx")) {
                             if let (Some(url), Some(path), Some(sha1), Some(size)) = (
                                artifact["url"].as_str(),
                                artifact["path"].as_str(),
                                artifact["sha1"].as_str(),
                                artifact["size"].as_u64()
                            ) {
                                jobs.push(DownloadJob {
                                    url: url.to_string(),
                                    fallback_url: None,
                                    path: libraries_dir.join(path),
                                    size,
                                    hash: Some(sha1.to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(jobs)
}