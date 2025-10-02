use std::fs;
use std::io::Write;
use std::path::PathBuf;


use crate::errors::LauncherError;
use crate::models::*;
use crate::services::config::load_config;
use crate::services::download::download_all_files as download_all_files_impl;

/// 初始化日志系统（控制器私有）
fn init_logging() -> Result<PathBuf, LauncherError> {
    let config = load_config()?;
    let minecraft_dir = PathBuf::from(&config.game_dir);
    let log_dir = minecraft_dir.join("logs");
    fs::create_dir_all(&log_dir)
        .map_err(|e| LauncherError::Custom(format!("无法创建日志目录: {}", e)))?;
    Ok(log_dir)
}

async fn fetch_versions(client: &reqwest::Client, url: &str) -> Result<VersionManifest, LauncherError> {
    let logs_dir = std::env::current_dir()?.join("logs");
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }
    let log_file = logs_dir.join("network_debug.log");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| LauncherError::Custom(format!("无法创建日志文件 {}: {}", log_file.display(), e)))?;

    writeln!(log, "[DEBUG] 准备发送请求到: {}", url)?;

    let request = client.get(url);
    let default_headers = reqwest::header::HeaderMap::new();
    writeln!(log, "[DEBUG] 请求头: {:?}", default_headers)?;

    let response = request.send().await.map_err(|e| {
        let _ = writeln!(log, "[ERROR] 请求失败: {}", e);
        e
    })?;

    writeln!(log, "[DEBUG] 响应状态码: {}", response.status())?;
    writeln!(log, "[DEBUG] 响应头: {:?}", response.headers())?;

    let content_type = response.headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    writeln!(log, "[DEBUG] Content-Type: {}", content_type)?;

    let bytes = response.bytes().await.map_err(|e| {
        let _ = writeln!(log, "[ERROR] 读取响应体失败: {}", e);
        e
    })?;

    let text = String::from_utf8_lossy(&bytes).into_owned();
    let text = text.trim_start_matches('\u{feff}').to_string();

    log::debug!("Received response (first 100 chars): {:?}", 
        text.chars().take(100).collect::<String>());

    let log_dir = PathBuf::from("logs");
    if !log_dir.exists() {
        fs::create_dir(&log_dir)?;
    }
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("version_fetch.log"))?;
    writeln!(log, "Raw response from {}:\n{}", url, text)?;

    let manifest = serde_json::from_str::<VersionManifest>(&text)
        .map_err(|e| {
            writeln!(log, "JSON parse error: {}", e).ok();
            LauncherError::Json(e)
        })?;

    writeln!(log, "Parsed manifest with {} versions", manifest.versions.len())?;
    Ok(manifest)
}

/// 获取 Minecraft 版本列表
#[tauri::command]
pub async fn get_versions() -> Result<VersionManifest, LauncherError> {
    let _ = init_logging()?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| LauncherError::Custom(format!("创建HTTP客户端失败: {}", e)))?;

    let urls = [
        "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json",
        "https://launchermeta.mojang.com/mc/game/version_manifest.json"
    ];

    let log_file = {
        let log_dir = init_logging()?;
        log_dir.join("version_fetch.log")
    };
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| LauncherError::Custom(format!("无法创建日志文件 {}: {}", log_file.display(), e)))?;

    writeln!(log, "[{}] 开始获取版本列表", chrono::Local::now())?;

    for (i, url) in urls.iter().enumerate() {
        writeln!(log, "尝试第{}个源: {}", i + 1, url)?;
        match fetch_versions(&client, url).await {
            Ok(manifest) => {
                writeln!(log, "成功获取版本列表，共{}个版本", manifest.versions.len())?;
                return Ok(manifest);
            },
            Err(e) => {
                writeln!(log, "获取失败: {}", e)?;
                continue;
            }
        }
    }
    Err(LauncherError::Custom("所有源都尝试失败，请检查网络连接".to_string()))
}

/// 下载 Minecraft 版本
#[tauri::command]
pub async fn download_version(
    version_id: String,
    mirror: Option<String>,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    let is_mirror = mirror.as_deref() == Some("bmcl");
    let base_url = if is_mirror {
        "https://bmclapi2.bangbang93.com"
    } else {
        "https://launchermeta.mojang.com"
    };

    let config = load_config()?;
    let game_dir = PathBuf::from(&config.game_dir);
    let version_dir = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&version_dir)?;
    let (libraries_base_dir, assets_base_dir) = (
        game_dir.join("libraries"),
        game_dir.join("assets")
    );

    let client = reqwest::Client::new();
    let manifest: VersionManifest = client
        .get(&format!("{}/mc/game/version_manifest.json", base_url))
        .send().await?.json().await?;

    let version = manifest.versions.iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| LauncherError::Custom(format!("版本 {} 不存在", version_id)))?;

    let version_json_url = if is_mirror {
        version.url.replace("https://launchermeta.mojang.com", base_url)
    } else {
        version.url.clone()
    };

    let text = client.get(&version_json_url).send().await?.text().await?;
    let version_json: serde_json::Value = serde_json::from_str(&text)
        .or_else(|_| serde_json::from_str(text.trim_start_matches('\u{feff}')))
        .map_err(|_| LauncherError::Custom(format!("无法解析版本JSON for {}", version_id)))?;

    // 收集下载任务
    let mut downloads = Vec::new();

    // 客户端 JAR
    let client_info = &version_json["downloads"]["client"];
    let client_url = client_info["url"].as_str().ok_or_else(|| LauncherError::Custom("无法获取客户端下载URL".to_string()))?;
    let client_size = client_info["size"].as_u64().unwrap_or(0);
    let client_hash = client_info["sha1"].as_str().unwrap_or("").to_string();
    let client_jar_path = version_dir.join(format!("{}.jar", version_id));
    downloads.push(DownloadJob {
        url: if is_mirror {
            client_url.replace("https://launcher.mojang.com", base_url)
                      .replace("https://piston-data.mojang.com", base_url)
        } else {
            client_url.to_string()
        },
        fallback_url: if is_mirror { Some(client_url.to_string()) } else { None },
        path: client_jar_path,
        size: client_size,
        hash: client_hash,
    });

    // 资源文件索引
    let assets_index_id = version_json["assetIndex"]["id"].as_str().ok_or_else(|| LauncherError::Custom("无法获取资源索引ID".to_string()))?;
    let assets_index_url = version_json["assetIndex"]["url"].as_str().ok_or_else(|| LauncherError::Custom("无法获取资源索引URL".to_string()))?;
    let assets_index_url = if is_mirror {
        assets_index_url.replace("https://launchermeta.mojang.com", base_url)
    } else {
        assets_index_url.to_string()
    };

    let assets_index_path = assets_base_dir.join("indexes").join(format!("{}.json", assets_index_id));
    fs::create_dir_all(assets_index_path.parent().unwrap())?;

    if !assets_index_path.exists() {
        let response = client.get(&assets_index_url).send().await?;
        let bytes = response.bytes().await?;
        fs::write(&assets_index_path, &bytes)?;
    }

    let index_content = fs::read_to_string(&assets_index_path)?;
    let index: serde_json::Value = serde_json::from_str(&index_content)?;

    if let Some(objects) = index["objects"].as_object() {
        for (_path, obj) in objects {
            let hash = obj["hash"].as_str().ok_or_else(|| LauncherError::Custom("资源缺少hash".to_string()))?;
            let size = obj["size"].as_u64().unwrap_or(0);
            let original_url = format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash);
            let download_url = if is_mirror {
                format!("https://bmclapi2.bangbang93.com/assets/{}/{}", &hash[..2], hash)
            } else {
                original_url.clone()
            };
            let file_path = assets_base_dir.join("objects").join(&hash[..2]).join(hash);
            downloads.push(DownloadJob {
                url: download_url,
                fallback_url: if is_mirror { Some(original_url) } else { None },
                path: file_path,
                size,
                hash: hash.to_string(),
            });
        }
    }

    // 库文件
    fs::create_dir_all(&libraries_base_dir)?;
    if let Some(libraries) = version_json["libraries"].as_array() {
        for lib in libraries {
            let mut should_download = true;
            if let Some(rules) = lib.get("rules").and_then(|r| r.as_array()) {
                should_download = false;
                for rule in rules {
                    let action = rule["action"].as_str().unwrap_or("");
                    if let Some(os) = rule.get("os") {
                        if let Some(name) = os["name"].as_str() {
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

            let is_lwjgl = lib["name"].as_str().map_or(false, |name| name.contains("lwjgl"));
            let has_natives = lib.get("natives").is_some();

            if is_lwjgl && has_natives {
                should_download = true;
            }

            if !should_download {
                continue;
            }

            if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                if let (Some(url), Some(path)) = (artifact["url"].as_str(), artifact["path"].as_str()) {
                    let size = artifact["size"].as_u64().unwrap_or(0);
                    let hash = artifact["sha1"].as_str().unwrap_or("").to_string();
                    let download_url = if is_mirror {
                        url.replace("https://libraries.minecraft.net", base_url)
                    } else {
                        url.to_string()
                    };
                    let file_path = libraries_base_dir.join(path);
                    downloads.push(DownloadJob {
                        url: download_url,
                        fallback_url: if is_mirror { Some(url.to_string()) } else { None },
                        path: file_path,
                        size,
                        hash,
                    });
                }
            }

            if let Some(natives) = lib.get("natives") {
                let is_lwjgl = lib["name"].as_str().map_or(false, |name| name.contains("lwjgl"));
                for (os_name, classifier_value) in natives.as_object().unwrap() {
                    let os_classifier = classifier_value.as_str().unwrap();
                    if os_name == std::env::consts::OS || is_lwjgl {
                        if let Some(classifiers) = lib.get("downloads").and_then(|d| d.get("classifiers")) {
                            if let Some(artifact) = classifiers.get(os_classifier) {
                                if let (Some(url), Some(path)) = (artifact["url"].as_str(), artifact["path"].as_str()) {
                                    let size = artifact["size"].as_u64().unwrap_or(0);
                                    let hash = artifact["sha1"].as_str().unwrap_or("").to_string();
                                    let download_url = if is_mirror {
                                        url.replace("https://libraries.minecraft.net", base_url)
                                    } else {
                                        url.to_string()
                                    };
                                    let file_path = libraries_base_dir.join(path);
                                    downloads.push(DownloadJob {
                                        url: download_url,
                                        fallback_url: if is_mirror { Some(url.to_string()) } else { None },
                                        path: file_path,
                                        size,
                                        hash,
                                    });
                                    continue;
                                }
                            }
                        }

                        if let Some(classifiers) = lib.get("classifiers") {
                            if let Some(artifact) = classifiers.get(os_classifier) {
                                if let (Some(url), Some(path)) = (artifact["url"].as_str(), artifact["path"].as_str()) {
                                    let size = artifact["size"].as_u64().unwrap_or(0);
                                    let hash = artifact["sha1"].as_str().unwrap_or("").to_string();
                                    let download_url = if is_mirror {
                                        url.replace("https://libraries.minecraft.net", base_url)
                                    } else {
                                        url.to_string()
                                    };
                                    let file_path = libraries_base_dir.join(path);
                                    downloads.push(DownloadJob {
                                        url: download_url,
                                        fallback_url: if is_mirror { Some(url.to_string()) } else { None },
                                        path: file_path,
                                        size,
                                        hash,
                                    });
                                    continue;
                                }
                            }
                        }

                        let name = lib["name"].as_str().unwrap_or("");
                        let parts: Vec<&str> = name.split(":").collect();
                        if parts.len() >= 3 {
                            let group_id = parts[0].replace(".", "/");
                            let artifact_id = parts[1];
                            let version = parts[2];
                            let classifier = os_classifier.replace("${arch}", if cfg!(target_pointer_width = "64") { "64" } else { "32" });
                            let natives_path = if artifact_id == "lwjgl" {
                                format!("{}/{}-platform/{}/{}-platform-{}-{}.jar",
                                       group_id, artifact_id, version, artifact_id, version, classifier)
                            } else if artifact_id == "lwjgl-platform" {
                                format!("{}/{}/{}/{}-{}-{}.jar",
                                       group_id, artifact_id, version, artifact_id, version, classifier)
                            } else {
                                format!("{}/{}/{}/{}-{}-{}.jar",
                                       group_id, artifact_id, version, artifact_id, version, classifier)
                            };
                            let natives_url = format!("https://libraries.minecraft.net/{}", natives_path);
                            let download_url = if is_mirror {
                                natives_url.replace("https://libraries.minecraft.net", base_url)
                            } else {
                                natives_url.clone()
                            };
                            let file_path = libraries_base_dir.join(&natives_path);
                            downloads.push(DownloadJob {
                                url: download_url,
                                fallback_url: if is_mirror { Some(natives_url) } else { None },
                                path: file_path,
                                size: 0,
                                hash: "".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // 执行批量下载（调用服务层）
    download_all_files_impl(downloads.clone(), &window, downloads.len() as u64, mirror).await?;

    // 保存版本元数据文件
    let version_json_path = version_dir.join(format!("{}.json", version_id));
    fs::write(version_json_path, text)?;

    Ok(())
}