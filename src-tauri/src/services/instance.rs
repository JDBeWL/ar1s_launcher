use crate::errors::LauncherError;
use crate::models::{DownloadJob, ForgeVersion, InstanceInfo, LaunchOptions};
use crate::services::{config, download, forge, launcher};
use crate::utils::file_utils;
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
    let game_dir = PathBuf::from(config.game_dir);
    let versions_dir = game_dir.join("versions");
    Ok((game_dir, versions_dir))
}

/// 创建新实例
pub async fn create_instance(
    new_instance_name: String,
    base_version_id: String,
    forge_version: Option<ForgeVersion>,
    window: &Window,
) -> Result<(), LauncherError> {
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
            config.download_mirror,
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

    if let Some(forge_ver) = forge_version {
        send_progress(60, "安装 Forge 加载器...", true);
        if let Err(e) = forge::install_forge(dest_dir.clone(), forge_ver.clone()).await {
            cleanup();
            return Err(e);
        }

        let forge_id_prefix = format!("{}-forge", forge_ver.mcversion);
        let forge_id_exact = format!("{}-forge-{}", forge_ver.mcversion, forge_ver.version);
        
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

    send_progress(100, "实例创建完成！", false);
    Ok(())
}

/// 获取实例列表
pub async fn get_instances() -> Result<Vec<InstanceInfo>, LauncherError> {
    let (_, versions_dir) = get_dirs()?;
    let mut instances = Vec::new();

    if !versions_dir.exists() {
        return Ok(instances);
    }

    if let Ok(entries) = fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let path = entry.path();
                    let json_path = path.join(format!("{}.json", name));

                    if json_path.exists() {
                        let version_id = fs::read_to_string(&json_path)
                            .ok()
                            .and_then(|c| serde_json::from_str::<Value>(&c).ok())
                            .and_then(|v| v["id"].as_str().map(String::from))
                            .unwrap_or_else(|| name.clone());

                        let created = entry.metadata()
                            .and_then(|m| m.created())
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs().to_string());

                        instances.push(InstanceInfo {
                            id: name.clone(),
                            name,
                            version: version_id,
                            path: path.to_string_lossy().to_string(),
                            created_time: created,
                        });
                    }
                }
            }
        }
    }
    Ok(instances)
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
    
    info!("实例 {} 已删除", instance_name);
    Ok(())
}

/// 重命名实例
pub async fn rename_instance(old_name: String, new_name: String) -> Result<(), LauncherError> {
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
            json["id"] = Value::String(new_name);
            fs::write(&json_path, serde_json::to_string_pretty(&json)?)?;
        }
    }

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
pub async fn launch_instance(instance_name: String, window: Window) -> Result<(), LauncherError> {
    let config = config::load_config()?;
    let (_, versions_dir) = get_dirs()?;
    let instance_dir = versions_dir.join(&instance_name);

    if !instance_dir.join(format!("{}.json", instance_name)).exists() {
        return Err(LauncherError::Custom(format!("实例 '{}' 的配置文件不存在", instance_name)));
    }

    let launch_options = LaunchOptions {
        version: instance_name,
        username: config.username.unwrap_or_else(|| "Player".to_string()),
        memory: Some(config.max_memory),
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
            download::download_all_files(index_jobs.clone(), window, 0, None).await?;
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
                                    hash: hash.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        if !all_jobs.is_empty() {
            download::download_all_files(all_jobs, window, 0, None).await?;
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
                hash: sha1.to_string(),
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
                hash: sha1.to_string(),
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
                        hash: sha1.to_string(),
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
                                    hash: sha1.to_string(),
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