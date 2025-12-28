//! 版本下载逻辑

use super::batch::download_all_files;
use super::http::get_http_client;
use crate::errors::LauncherError;
use crate::models::{DownloadJob, VersionManifest};
use crate::services::config::load_config;
use std::fs;
use std::path::PathBuf;
use tauri::Window;

/// 处理并下载指定版本
pub async fn process_and_download_version(
    version_id: String,
    mirror: Option<String>,
    window: &Window,
) -> Result<(), LauncherError> {
    let is_mirror = mirror.is_some();
    let base_url = if is_mirror {
        "https://bmclapi2.bangbang93.com"
    } else {
        "https://launchermeta.mojang.com"
    };

    let config = load_config()?;
    let game_dir = PathBuf::from(&config.game_dir);
    let version_dir = game_dir.join("versions").join(&version_id);

    // 创建版本目录
    fs::create_dir_all(&version_dir)?;
    let libraries_base_dir = game_dir.join("libraries");
    let assets_base_dir = game_dir.join("assets");

    // 使用全局 HTTP 客户端
    let client = get_http_client()?;

    // 获取版本清单
    let manifest: VersionManifest = client
        .get(&format!("{}/mc/game/version_manifest.json", base_url))
        .send()
        .await?
        .json()
        .await?;

    let version = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| LauncherError::Custom(format!("版本 {} 不存在", version_id)))?;

    // 获取版本 JSON
    let version_json_url = if is_mirror {
        version
            .url
            .replace("https://launchermeta.mojang.com", base_url)
    } else {
        version.url.clone()
    };

    let text = client.get(&version_json_url).send().await?.text().await?;
    let version_json: serde_json::Value = serde_json::from_str(&text)
        .or_else(|_| serde_json::from_str(text.trim_start_matches('\u{feff}')))
        .map_err(|_| LauncherError::Custom(format!("无法解析版本JSON for {}", version_id)))?;

    // 收集下载任务
    let mut downloads = Vec::new();

    // 添加客户端 JAR
    collect_client_jar(&version_json, &version_dir, &version_id, is_mirror, base_url, &mut downloads)?;

    // 添加资源文件
    collect_assets(
        &client,
        &version_json,
        &assets_base_dir,
        is_mirror,
        base_url,
        &mut downloads,
    )
    .await?;

    // 添加库文件
    collect_libraries(&version_json, &libraries_base_dir, is_mirror, base_url, &mut downloads)?;

    // 执行批量下载
    match download_all_files(downloads.clone(), window, downloads.len() as u64, mirror).await {
        Ok(_) => {
            // 保存版本元数据文件
            let version_json_path = version_dir.join(format!("{}.json", version_id));
            fs::write(version_json_path, text)?;
            Ok(())
        }
        Err(e) => {
            // 下载失败时清理版本文件夹
            println!("下载失败，清理版本文件夹: {}", version_dir.display());
            if version_dir.exists() {
                if let Err(cleanup_err) = fs::remove_dir_all(&version_dir) {
                    println!("清理版本文件夹失败: {}", cleanup_err);
                }
            }
            Err(e)
        }
    }
}

/// 收集客户端 JAR 下载任务
fn collect_client_jar(
    version_json: &serde_json::Value,
    version_dir: &PathBuf,
    version_id: &str,
    is_mirror: bool,
    base_url: &str,
    downloads: &mut Vec<DownloadJob>,
) -> Result<(), LauncherError> {
    let client_info = &version_json["downloads"]["client"];
    let client_url = client_info["url"]
        .as_str()
        .ok_or_else(|| LauncherError::Custom("无法获取客户端下载URL".to_string()))?;
    let client_size = client_info["size"].as_u64().unwrap_or(0);
    let client_hash = client_info["sha1"].as_str().unwrap_or("").to_string();
    let client_jar_path = version_dir.join(format!("{}.jar", version_id));

    downloads.push(DownloadJob {
        url: if is_mirror {
            client_url
                .replace("https://launcher.mojang.com", base_url)
                .replace("https://piston-data.mojang.com", base_url)
        } else {
            client_url.to_string()
        },
        fallback_url: if is_mirror {
            Some(client_url.to_string())
        } else {
            None
        },
        path: client_jar_path,
        size: client_size,
        hash: client_hash,
    });

    Ok(())
}

/// 收集资源文件下载任务
async fn collect_assets(
    client: &reqwest::Client,
    version_json: &serde_json::Value,
    assets_base_dir: &PathBuf,
    is_mirror: bool,
    base_url: &str,
    downloads: &mut Vec<DownloadJob>,
) -> Result<(), LauncherError> {
    let assets_index_id = version_json["assetIndex"]["id"]
        .as_str()
        .ok_or_else(|| LauncherError::Custom("无法获取资源索引ID".to_string()))?;
    let assets_index_url = version_json["assetIndex"]["url"]
        .as_str()
        .ok_or_else(|| LauncherError::Custom("无法获取资源索引URL".to_string()))?;

    let assets_index_url = if is_mirror {
        assets_index_url.replace("https://launchermeta.mojang.com", base_url)
    } else {
        assets_index_url.to_string()
    };

    let assets_index_path = assets_base_dir
        .join("indexes")
        .join(format!("{}.json", assets_index_id));
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
            let hash = obj["hash"]
                .as_str()
                .ok_or_else(|| LauncherError::Custom("资源缺少hash".to_string()))?;
            let size = obj["size"].as_u64().unwrap_or(0);
            let original_url = format!(
                "https://resources.download.minecraft.net/{}/{}",
                &hash[..2],
                hash
            );
            let download_url = if is_mirror {
                format!(
                    "https://bmclapi2.bangbang93.com/assets/{}/{}",
                    &hash[..2],
                    hash
                )
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

    Ok(())
}

/// 收集库文件下载任务
fn collect_libraries(
    version_json: &serde_json::Value,
    libraries_base_dir: &PathBuf,
    is_mirror: bool,
    base_url: &str,
    downloads: &mut Vec<DownloadJob>,
) -> Result<(), LauncherError> {
    fs::create_dir_all(libraries_base_dir)?;

    let Some(libraries) = version_json["libraries"].as_array() else {
        return Ok(());
    };

    for lib in libraries {
        if !should_download_library(lib) {
            continue;
        }

        // 处理普通库
        if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
            if let Some(job) = create_library_job(artifact, libraries_base_dir, is_mirror, base_url) {
                downloads.push(job);
            }
        }

        // 处理 natives 库
        collect_natives_library(lib, libraries_base_dir, is_mirror, base_url, downloads);
    }

    Ok(())
}

/// 检查是否应该下载库
fn should_download_library(lib: &serde_json::Value) -> bool {
    let Some(rules) = lib.get("rules").and_then(|r| r.as_array()) else {
        return true;
    };

    let mut should_download = false;
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

    // LWJGL natives 特殊处理
    let is_lwjgl = lib["name"]
        .as_str()
        .map_or(false, |name| name.contains("lwjgl"));
    let has_natives = lib.get("natives").is_some();

    if is_lwjgl && has_natives {
        return true;
    }

    should_download || !lib.get("rules").is_some()
}

/// 创建库下载任务
fn create_library_job(
    artifact: &serde_json::Value,
    libraries_base_dir: &PathBuf,
    is_mirror: bool,
    base_url: &str,
) -> Option<DownloadJob> {
    let url = artifact["url"].as_str()?;
    let path = artifact["path"].as_str()?;
    let size = artifact["size"].as_u64().unwrap_or(0);
    let hash = artifact["sha1"].as_str().unwrap_or("").to_string();

    let download_url = if is_mirror {
        url.replace(
            "https://libraries.minecraft.net",
            &format!("{}/maven", base_url),
        )
    } else {
        url.to_string()
    };

    Some(DownloadJob {
        url: download_url,
        fallback_url: if is_mirror { Some(url.to_string()) } else { None },
        path: libraries_base_dir.join(path),
        size,
        hash,
    })
}

/// 收集 natives 库下载任务
fn collect_natives_library(
    lib: &serde_json::Value,
    libraries_base_dir: &PathBuf,
    is_mirror: bool,
    base_url: &str,
    downloads: &mut Vec<DownloadJob>,
) {
    let Some(natives) = lib.get("natives") else {
        return;
    };

    let is_lwjgl = lib["name"]
        .as_str()
        .map_or(false, |name| name.contains("lwjgl"));

    let current_os = std::env::consts::OS;
    let os_key = if current_os == "macos" { "osx" } else { current_os };

    let Some(natives_obj) = natives.as_object() else {
        return;
    };

    for (os_name, classifier_value) in natives_obj {
        let Some(os_classifier) = classifier_value.as_str() else {
            continue;
        };

        if os_name != os_key && !is_lwjgl {
            continue;
        }

        // 尝试从 downloads.classifiers 获取
        if let Some(artifact) = lib
            .get("downloads")
            .and_then(|d| d.get("classifiers"))
            .and_then(|c| c.get(os_classifier))
        {
            if let Some(job) = create_library_job(artifact, libraries_base_dir, is_mirror, base_url) {
                downloads.push(job);
                continue;
            }
        }

        // 尝试从 classifiers 获取
        if let Some(artifact) = lib.get("classifiers").and_then(|c| c.get(os_classifier)) {
            if let Some(job) = create_library_job(artifact, libraries_base_dir, is_mirror, base_url) {
                downloads.push(job);
                continue;
            }
        }

        // 回退：根据 maven 坐标构建路径
        if let Some(job) = create_natives_job_from_name(lib, os_classifier, libraries_base_dir, is_mirror, base_url) {
            downloads.push(job);
        }
    }
}

/// 从库名称创建 natives 下载任务
fn create_natives_job_from_name(
    lib: &serde_json::Value,
    os_classifier: &str,
    libraries_base_dir: &PathBuf,
    is_mirror: bool,
    base_url: &str,
) -> Option<DownloadJob> {
    let name = lib["name"].as_str()?;
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }

    let group_id = parts[0].replace('.', "/");
    let artifact_id = parts[1];
    let version = parts[2];
    let classifier = os_classifier.replace(
        "${arch}",
        if cfg!(target_pointer_width = "64") { "64" } else { "32" },
    );

    let natives_path = if artifact_id == "lwjgl" {
        format!(
            "{}/{}-platform/{}/{}-platform-{}-{}.jar",
            group_id, artifact_id, version, artifact_id, version, classifier
        )
    } else {
        format!(
            "{}/{}/{}/{}-{}-{}.jar",
            group_id, artifact_id, version, artifact_id, version, classifier
        )
    };

    let natives_url = format!("https://libraries.minecraft.net/{}", natives_path);
    let download_url = if is_mirror {
        format!("{}/maven/{}", base_url, natives_path)
    } else {
        natives_url.clone()
    };

    Some(DownloadJob {
        url: download_url,
        fallback_url: if is_mirror { Some(natives_url) } else { None },
        path: libraries_base_dir.join(&natives_path),
        size: 0,
        hash: String::new(),
    })
}
