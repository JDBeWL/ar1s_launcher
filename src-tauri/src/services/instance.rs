use crate::errors::LauncherError;
use crate::models::{ForgeVersion, DownloadJob};
use crate::services::{config, download, forge};
use crate::utils::file_utils;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use serde::Serialize;
use tauri::Emitter;

#[derive(Clone, Serialize)]
struct InstallProgress {
    progress: u8,
    message: String,
    indeterminate: bool,
}

// 使用共享模块中的函数

pub async fn create_instance(
    new_instance_name: String,
    base_version_id: String,
    forge_version: Option<ForgeVersion>,
    window: &tauri::Window,
) -> Result<(), LauncherError> {
    let config = config::load_config()?;
    let game_dir = PathBuf::from(config.game_dir);
    let versions_dir = game_dir.join("versions");

    let source_dir = versions_dir.join(&base_version_id);
    let dest_dir = versions_dir.join(&new_instance_name);

    // 发送进度更新
    let send_progress = |progress: u8, message: &str, indeterminate: bool| {
        let _ = window.emit("instance-install-progress", InstallProgress {
            progress,
            message: message.to_string(),
            indeterminate,
        });
    };

    // 1. 检查实例是否已存在
    if dest_dir.exists() {
        return Err(LauncherError::Custom(format!(
            "名为 '{}' 的实例已存在.",
            new_instance_name
        )));
    }

    send_progress(5, "检查基础版本...", false);

    // 2. 检查基础版本是否存在，如果不存在则下载
    if !source_dir.exists() {
        send_progress(10, "下载基础版本...", true);
        if let Err(e) = download::process_and_download_version(base_version_id.clone(), config.download_mirror.clone(), window).await {
            // 基础版本下载失败，此时实例目录尚未创建，不需要清理
            return Err(e);
        }
    }
    
    if !source_dir.exists() {
        // 基础版本下载失败，此时实例目录尚未创建，不需要清理
        return Err(LauncherError::Custom(format!(
            "基础版本 '{}' 下载失败或未找到.",
            base_version_id
        )));
    }

    send_progress(30, "复制基础文件...", false);
    
    // 3. 复制整个目录
    if let Err(e) = file_utils::copy_dir_all(&source_dir, &dest_dir) {
        file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
        return Err(e.into());
    }

    send_progress(40, "重命名配置文件...", false);
    
    // 4. 重命名 .json 和 .jar 文件
    let old_json_path = dest_dir.join(format!("{}.json", base_version_id));
    let new_json_path = dest_dir.join(format!("{}.json", new_instance_name));
    if let Err(e) = fs::rename(&old_json_path, &new_json_path) {
        file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
        return Err(e.into());
    }

    let old_jar_path = dest_dir.join(format!("{}.jar", base_version_id));
    if old_jar_path.exists() {
        let new_jar_path = dest_dir.join(format!("{}.jar", new_instance_name));
        if let Err(e) = fs::rename(&old_jar_path, &new_jar_path) {
            file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e.into());
        }
    }

    send_progress(50, "更新配置文件...", false);
    
    // 5. 修改新 JSON 文件中的 'id' 字段
    let json_str = match fs::read_to_string(&new_json_path) {
        Ok(s) => s,
        Err(e) => {
            file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e.into());
        }
    };
    
    let mut json: Value = match serde_json::from_str(&json_str) {
        Ok(j) => j,
        Err(e) => {
            file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e.into());
        }
    };

    if let Some(id_field) = json.get_mut("id") {
        *id_field = Value::String(new_instance_name.clone());
    }

    let modified_json_str = match serde_json::to_string_pretty(&json) {
        Ok(s) => s,
        Err(e) => {
            file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e.into());
        }
    };
    
    if let Err(e) = fs::write(&new_json_path, modified_json_str) {
        file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
        return Err(e.into());
    }

    // 6. 如果请求了 Forge，则安装 Forge
    if let Some(forge_version) = forge_version {
        send_progress(60, "安装Forge加载器...", true);
        if let Err(e) = forge::install_forge(dest_dir.clone(), forge_version.clone()).await {
            file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e);
        }

        // Forge 安装程序运行后，通常会在 game_dir/versions 下创建一个版本条目，
        // 如 `{mcversion}-forge-{forge_version}`。
        // 尝试将 Forge 版本的 JSON 与基础（原版）JSON 合并，
        // 遵循 UI 中描述的规则：优先使用 Forge 字段，
        // 按 group:artifact:version 去重库文件，id 设置为实例名称。

        // 尝试查找实际的 forge 创建的目录。常见模式是
        // "{mcversion}-forge-{build}"，但某些安装程序可能会写入稍微不同的 id。
        // 我们将扫描 versions 目录中以前缀 "{mcversion}-forge" 开头的条目。
        let mut found_forge_id: Option<String> = None;
        if let Ok(entries) = fs::read_dir(game_dir.join("versions")) {
            let prefix = format!("{}-forge", forge_version.mcversion);
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == format!("{}-forge-{}", forge_version.mcversion, forge_version.version) {
                        // 精确匹配预期
                        found_forge_id = Some(name.to_string());
                        break;
                    }
                }
            }
            if found_forge_id.is_none() {
                // 回退：第一个以前缀开头的目录
                if let Ok(entries2) = fs::read_dir(game_dir.join("versions")) {
                    for entry in entries2.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with(&prefix) {
                                found_forge_id = Some(name.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        let forge_json_path = if let Some(ref fid) = found_forge_id {
            game_dir.join("versions").join(fid).join(format!("{}.json", fid))
        } else {
            game_dir.join("versions").join(format!("{}-forge-{}", forge_version.mcversion, forge_version.version)).join(format!("{}-forge-{}.json", forge_version.mcversion, forge_version.version))
        };
        let base_json_path = game_dir.join("versions").join(&base_version_id).join(format!("{}.json", &base_version_id));
        let instance_json_path = dest_dir.join(format!("{}.json", new_instance_name));

        if forge_json_path.exists() && base_json_path.exists() {
            let base_str = match fs::read_to_string(&base_json_path) {
                Ok(s) => s,
                Err(e) => {
                    file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };
            let forge_str = match fs::read_to_string(&forge_json_path) {
                Ok(s) => s,
                Err(e) => {
                    file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };
            let base_json: Value = match serde_json::from_str(&base_str) {
                Ok(j) => j,
                Err(e) => {
                    file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };
            let forge_json: Value = match serde_json::from_str(&forge_str) {
                Ok(j) => j,
                Err(e) => {
                    file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };

            // 从 forge_json 开始合并，然后从 base_json 填充缺失的字段
            let mut merged = forge_json.clone();

            // 确保 id 设置为实例名称
            merged["id"] = Value::String(new_instance_name.clone());

            // mainClass：优先使用 Forge（如果已存在于合并中），否则从基础版本获取
            if !merged.get("mainClass").is_some() {
                if let Some(mc) = base_json.get("mainClass") {
                    merged["mainClass"] = mc.clone();
                }
            }

            // arguments：优先使用 Forge。如果 Forge 使用旧的 `minecraftArguments`，则转换为 `arguments.game` 数组
            if merged.get("arguments").is_none() {
                if let Some(mc_args) = forge_json.get("minecraftArguments").and_then(|v| v.as_str()) {
                    // 按空格分割（与启动器相同的行为）
                    let parts: Vec<Value> = mc_args
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    let mut args_obj = serde_json::Map::new();
                    args_obj.insert("game".to_string(), Value::Array(parts));
                    merged["arguments"] = Value::Object(args_obj);
                } else if let Some(base_args) = base_json.get("arguments") {
                    merged["arguments"] = base_args.clone();
                } else if let Some(base_mc_args) = base_json.get("minecraftArguments").and_then(|v| v.as_str()) {
                    let parts: Vec<Value> = base_mc_args
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    let mut args_obj = serde_json::Map::new();
                    args_obj.insert("game".to_string(), Value::Array(parts));
                    merged["arguments"] = Value::Object(args_obj);
                }
            }

            // libraries：以 Forge 为主，然后附加缺失的基础库（按名称去重）
            let mut final_libs: Vec<Value> = Vec::new();
            let mut seen = std::collections::HashSet::new();

            if let Some(forge_libs) = forge_json.get("libraries").and_then(|v| v.as_array()) {
                for lib in forge_libs {
                    // 尝试获取唯一键：library.name (group:artifact:version)
                    if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                        seen.insert(name.to_string());
                    }
                    final_libs.push(lib.clone());
                }
            }

            if let Some(base_libs) = base_json.get("libraries").and_then(|v| v.as_array()) {
                for lib in base_libs {
                    let mut add = true;
                    if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                        if seen.contains(name) {
                            add = false;
                        }
                    }
                    if add {
                        final_libs.push(lib.clone());
                    }
                }
            }

            if !final_libs.is_empty() {
                merged["libraries"] = Value::Array(final_libs);
            }

            // 从 base_json 填充其他缺失的顶级字段（logging、assetIndex 等）
            if let Some(obj) = base_json.as_object() {
                for (k, v) in obj.iter() {
                    if !merged.get(k).is_some() {
                        merged[k] = v.clone();
                    }
                }
            }

            // 最后，将合并后的 JSON 写入实例 JSON 路径
            let merged_str = match serde_json::to_string_pretty(&merged) {
                Ok(s) => s,
                Err(e) => {
                    file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };
            if let Err(e) = fs::write(&instance_json_path, merged_str) {
                file_utils::cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                return Err(e.into());
            }

            // 写入合并的 JSON 后，确保下载所需的文件（客户端、库、原生库、资源）
            // 从合并的清单构建下载任务并运行 download_all_files
            // 辅助闭包用于收集下载任务
            let collect_download_jobs = |merged_json: &Value| -> Result<Vec<DownloadJob>, LauncherError> {
                let mut jobs: Vec<DownloadJob> = Vec::new();
                let libraries_base_dir = game_dir.join("libraries");
                let assets_base_dir = game_dir.join("assets");

                // 1) 客户端 jar
                if let Some(client) = merged_json.get("downloads").and_then(|d| d.get("client")) {
                    if let Some(url) = client.get("url").and_then(|u| u.as_str()) {
                        let size = client.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                        let hash = client.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string();
                        let path = dest_dir.join(format!("{}.jar", new_instance_name));
                        jobs.push(DownloadJob { url: url.to_string(), fallback_url: None, path, size, hash });
                    }
                }

                // 2) 资源索引 -> 对象
                if let Some(asset_idx) = merged_json.get("assetIndex") {
                    if let Some(idx_id) = asset_idx.get("id").and_then(|v| v.as_str()) {
                        if let Some(idx_url) = asset_idx.get("url").and_then(|v| v.as_str()) {
                            let index_path = assets_base_dir.join("indexes").join(format!("{}.json", idx_id));
                            jobs.push(DownloadJob {
                                url: idx_url.to_string(),
                                fallback_url: None,
                                path: index_path.clone(),
                                size: asset_idx.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                                hash: asset_idx.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string(),
                            });

                            // 如果索引已存在，我们仍然希望在调用 download_all_files 时将单个对象加入队列，
                            // 但构建对象列表需要索引文件存在。我们将在下载后通过读取索引来处理对象下载。
                        }
                    }
                }

                // 3) 库 + 原生库
                if let Some(libs) = merged_json.get("libraries").and_then(|v| v.as_array()) {
                    for lib in libs {
                        // 规则评估（从 download.rs 复制逻辑）
                        let mut should_download = true;
                        if let Some(rules) = lib.get("rules").and_then(|r| r.as_array()) {
                            should_download = false;
                            for rule in rules {
                                let action = rule.get("action").and_then(|a| a.as_str()).unwrap_or("");
                                if let Some(os) = rule.get("os") {
                                    if let Some(name) = os.get("name").and_then(|n| n.as_str()) {
                                        let current_os = std::env::consts::OS;
                                        if name == current_os {
                                            should_download = action == "allow";
                                        }
                                    }
                                } else {
                                    should_download = action == "allow";
                                }
                            }
                        }

                        if !should_download {
                            continue;
                        }

                        // 构件
                        if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                            let path = artifact.get("path").and_then(|p| p.as_str()).map(|s| s.to_string());
                            let url = artifact.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
                            let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                            let hash = artifact.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string();
                            if let Some(path_str) = path {
                                let file_path = libraries_base_dir.join(path_str.clone());
                                let download_url = if let Some(u) = url { u } else { format!("https://libraries.minecraft.net/{}", path_str) };
                                jobs.push(DownloadJob { url: download_url, fallback_url: None, path: file_path, size, hash });
                            }
                        }

                        // 原生库/分类器
                        if let Some(natives) = lib.get("natives") {
                            if let Some(natives_map) = natives.as_object() {
                                let current_os = std::env::consts::OS;
                                for (os_name, classifier_val) in natives_map.iter() {
                                    let classifier = classifier_val.as_str().unwrap_or("");
                                    if os_name == current_os || lib.get("name").and_then(|n| n.as_str()).map_or(false, |s| s.contains("lwjgl")) {
                                        // 优先使用 downloads.classifiers
                                        if let Some(classifiers) = lib.get("downloads").and_then(|d| d.get("classifiers")) {
                                            if let Some(artifact) = classifiers.get(classifier) {
                                                if let (Some(path), Some(url)) = (artifact.get("path").and_then(|p| p.as_str()), artifact.get("url").and_then(|u| u.as_str())) {
                                                    let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                                    let hash = artifact.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string();
                                                    let file_path = libraries_base_dir.join(path);
                                                    jobs.push(DownloadJob { url: url.to_string(), fallback_url: None, path: file_path, size, hash });
                                                    continue;
                                                }
                                            }
                                        }

                                        // 回退：尝试顶级 "classifiers" 下的分类器
                                        if let Some(classifiers) = lib.get("classifiers") {
                                            if let Some(artifact) = classifiers.get(classifier) {
                                                if let (Some(path), Some(url)) = (artifact.get("path").and_then(|p| p.as_str()), artifact.get("url").and_then(|u| u.as_str())) {
                                                    let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                                    let hash = artifact.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string();
                                                    let file_path = libraries_base_dir.join(path);
                                                    jobs.push(DownloadJob { url: url.to_string(), fallback_url: None, path: file_path, size, hash });
                                                    continue;
                                                }
                                            }
                                        }

                                        // 最后手段：从名称派生路径
                                        if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                                            let parts: Vec<&str> = name.split(":").collect();
                                            if parts.len() >= 3 {
                                                let group_id = parts[0].replace(".", "/");
                                                let artifact_id = parts[1];
                                                let version = parts[2];
                                                let classifier_replaced = classifier.replace("${arch}", if cfg!(target_pointer_width = "64") { "64" } else { "32" });
                                                let natives_path = format!("{}/{}/{}/{}-{}-{}.jar", group_id, artifact_id, version, artifact_id, version, classifier_replaced);
                                                let natives_url = format!("https://libraries.minecraft.net/{}", natives_path);
                                                let file_path = libraries_base_dir.join(&natives_path);
                                                jobs.push(DownloadJob { url: natives_url, fallback_url: None, path: file_path, size: 0, hash: "".to_string() });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(jobs)
            };

            // 收集并运行下载任务
            if let Ok(mut jobs) = collect_download_jobs(&merged) {
                // 如果存在资源索引任务，我们需要先下载索引，然后将对象扩展为任务
                // 查找资源索引任务
                let mut index_jobs: Vec<DownloadJob> = Vec::new();
                jobs.retain(|j| {
                    if j.path.to_string_lossy().contains("indexes") {
                        index_jobs.push(j.clone());
                        false
                    } else {
                        true
                    }
                });

                // 首先下载索引文件
                if !index_jobs.is_empty() {
                    let _ = download::download_all_files(index_jobs.clone(), window, index_jobs.len() as u64, config.download_mirror.clone()).await;
                    // 对于每个索引，解析并将对象加入队列
                    for idx in index_jobs.iter() {
                        if idx.path.exists() {
                            if let Ok(idx_content) = fs::read_to_string(&idx.path) {
                                if let Ok(idx_json) = serde_json::from_str::<Value>(&idx_content) {
                                    if let Some(objects) = idx_json.get("objects").and_then(|o| o.as_object()) {
                                        for (_p, obj) in objects.iter() {
                                            if let Some(hash) = obj.get("hash").and_then(|h| h.as_str()) {
                                                let size = obj.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                                let download_url = format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash);
                                                let file_path = game_dir.join("assets").join("objects").join(&hash[..2]).join(hash);
                                                jobs.push(DownloadJob { url: download_url, fallback_url: None, path: file_path, size, hash: hash.to_string() });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                    // 最后运行剩余任务的下载（库、客户端、资源对象）
                if !jobs.is_empty() {
                    send_progress(70, "校验并下载缺失游戏文件...", true);
                    let _ = download::download_all_files(jobs.clone(), window, jobs.len() as u64, config.download_mirror.clone()).await;
                }
            }

            // 清理临时的 forge 版本目录，因为合并后不再需要
            if let Some(ref fid) = found_forge_id {
                println!("Forge: 合并完成，清理 Forge 版本文件夹: {}", fid);
                let version_dir_to_clean = game_dir.join("versions").join(fid);
                
                if version_dir_to_clean.exists() {
                    println!("Forge: 清理版本文件夹: {}", version_dir_to_clean.display());
                    if let Err(e) = fs::remove_dir_all(&version_dir_to_clean) {
                        println!("Forge: 清理版本文件夹失败: {}，但安装继续", e);
                    } else {
                        println!("Forge: 版本文件夹清理完成");
                    }
                }
            }
        } else {
            // 如果没有找到 forge 版本，则跳过合并
            println!("Forge json not found at {}. Skipping merge.", forge_json_path.display());
        }
    }

    send_progress(100, "安装完成！", false);
    
    Ok(())
}

// 获取实例列表
pub async fn get_instances() -> Result<Vec<crate::controllers::instance_controller::InstanceInfo>, LauncherError> {
    let config = config::load_config()?;
    let game_dir = PathBuf::from(config.game_dir);
    let versions_dir = game_dir.join("versions");
    
    let mut instances = Vec::new();
    
    if versions_dir.exists() {
        if let Ok(entries) = fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let instance_name = entry.file_name().to_string_lossy().to_string();
                        let instance_path = entry.path();
                        
                        // 检查是否存在对应的 JSON 文件
                        let json_path = instance_path.join(format!("{}.json", instance_name));
                        if json_path.exists() {
                            if let Ok(json_content) = fs::read_to_string(&json_path) {
                                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&json_content) {
                                    let version = json_value.get("id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&instance_name)
                                        .to_string();
                                    
                                    let created_time = entry.metadata()
                                        .ok()
                                        .and_then(|meta| meta.created().ok())
                                        .map(|time| {
                                            time.duration_since(std::time::UNIX_EPOCH)
                                                .map(|dur| dur.as_secs().to_string())
                                                .unwrap_or_default()
                                        });
                                    
                                    instances.push(crate::controllers::instance_controller::InstanceInfo {
                                        id: instance_name.clone(),
                                        name: instance_name,
                                        version,
                                        path: instance_path.to_string_lossy().to_string(),
                                        created_time,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(instances)
}

// 删除实例
pub async fn delete_instance(instance_name: String) -> Result<(), LauncherError> {
    let config = config::load_config()?;
    let game_dir = PathBuf::from(config.game_dir);
    let instance_dir = game_dir.join("versions").join(&instance_name);
    
    if !instance_dir.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 不存在", instance_name)));
    }
    
    // 删除实例目录
    fs::remove_dir_all(&instance_dir)
        .map_err(|e| LauncherError::Custom(format!("删除实例失败: {}", e)))?;
    
    Ok(())
}

// 重命名实例
pub async fn rename_instance(old_name: String, new_name: String) -> Result<(), LauncherError> {
    let config = config::load_config()?;
    let game_dir = PathBuf::from(config.game_dir);
    let versions_dir = game_dir.join("versions");
    
    let old_dir = versions_dir.join(&old_name);
    let new_dir = versions_dir.join(&new_name);
    
    if !old_dir.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 不存在", old_name)));
    }
    
    if new_dir.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 已存在", new_name)));
    }
    
    // 重命名目录
    fs::rename(&old_dir, &new_dir)
        .map_err(|e| LauncherError::Custom(format!("重命名实例失败: {}", e)))?;
    
    // 重命名 JSON 文件
    let old_json = new_dir.join(format!("{}.json", old_name));
    let new_json = new_dir.join(format!("{}.json", new_name));
    
    if old_json.exists() {
        fs::rename(&old_json, &new_json)
            .map_err(|e| LauncherError::Custom(format!("重命名 JSON 文件失败: {}", e)))?;
        
        // 更新 JSON 文件中的 id 字段
        if let Ok(json_content) = fs::read_to_string(&new_json) {
            if let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(&json_content) {
                if let Some(id_field) = json_value.get_mut("id") {
                    *id_field = serde_json::Value::String(new_name.clone());
                }
                
                if let Ok(updated_json) = serde_json::to_string_pretty(&json_value) {
                    fs::write(&new_json, updated_json)
                        .map_err(|e| LauncherError::Custom(format!("更新 JSON 文件失败: {}", e)))?;
                }
            }
        }
    }
    
    // 重命名 JAR 文件（如果存在）
    let old_jar = new_dir.join(format!("{}.jar", old_name));
    let new_jar = new_dir.join(format!("{}.jar", new_name));
    
    if old_jar.exists() {
        fs::rename(&old_jar, &new_jar)
            .map_err(|e| LauncherError::Custom(format!("重命名 JAR 文件失败: {}", e)))?;
    }
    
    Ok(())
}

// 打开实例文件夹
pub async fn open_instance_folder(instance_name: String) -> Result<(), LauncherError> {
    let config = config::load_config()?;
    let game_dir = PathBuf::from(config.game_dir);
    let instance_dir = game_dir.join("versions").join(&instance_name);
    
    if !instance_dir.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 不存在", instance_name)));
    }
    
    // 使用系统默认程序打开文件夹
    opener::open(&instance_dir)
        .map_err(|e| LauncherError::Custom(format!("打开文件夹失败: {}", e)))?;
    
    Ok(())
}

// 启动实例
pub async fn launch_instance(instance_name: String, window: tauri::Window) -> Result<(), LauncherError> {
    let config = config::load_config()?;
    let game_dir = PathBuf::from(config.game_dir);
    let instance_dir = game_dir.join("versions").join(&instance_name);
    
    if !instance_dir.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 不存在", instance_name)));
    }
    
    // 检查实例的 JSON 文件是否存在
    let instance_json_path = instance_dir.join(format!("{}.json", instance_name));
    if !instance_json_path.exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 的配置文件不存在", instance_name)));
    }
    
    // 获取用户名，如果未设置则使用默认值
    let username = config.username.unwrap_or_else(|| "Player".to_string());
    
    // 创建启动选项
    let launch_options = crate::models::LaunchOptions {
        version: instance_name.clone(),
        username,
        memory: Some(config.max_memory),
    };
    
    // 调用启动器服务启动游戏
    crate::services::launcher::launch_minecraft(launch_options, window).await
}