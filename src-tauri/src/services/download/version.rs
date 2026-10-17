//! 版本资源整合与下载
//!
//! 跟随 inheritsFrom 递归补基础版本，汇总成下载任务并按镜像改写地址

use super::batch::download_all_files;
use super::http::get_http_client;
use crate::errors::LauncherError;
use crate::models::{DownloadJob, VersionManifest};
use crate::services::config::{self, load_config};
use log::{info, warn};
use std::fs;
use std::path::PathBuf;
use tauri::{Emitter, Window};

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

    // 记录目录本次调用前是否已存在，供失败清理判断
    let dir_existed_before = version_dir.exists();

    fs::create_dir_all(&version_dir)?;
    let libraries_base_dir = game_dir.join("libraries");
    let assets_base_dir = game_dir.join("assets");

    let client = get_http_client();

    // 整合包/mod 加载器版本：本地 JSON 存在且有 inheritsFrom
    let local_version_json_path = version_dir.join(format!("{}.json", version_id));
    let (actual_version_id, version_json, text) = if local_version_json_path.exists() {
        let local_text = fs::read_to_string(&local_version_json_path)?;
        let local_json: serde_json::Value = serde_json::from_str(&local_text)
            .map_err(|e| LauncherError::Custom(format!("解析本地版本JSON失败: {}", e)))?;

        if let Some(inherits_from) = local_json["inheritsFrom"].as_str() {
            info!("版本 {} 继承自 {}", version_id, inherits_from);
            Box::pin(process_and_download_version(
                inherits_from.to_string(),
                mirror.clone(),
                window,
            ))
            .await?;

            // 基础版本已下载，整合包的库文件另行处理
            return download_modpack_libraries(
                &local_json,
                &libraries_base_dir,
                is_mirror,
                base_url,
                window,
            )
            .await;
        }

        (version_id.clone(), local_json, local_text)
    } else {
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

        let version_json_url = if is_mirror {
            version
                .url
                .replace("https://launchermeta.mojang.com", base_url)
                .replace("https://piston-meta.mojang.com", base_url)
        } else {
            version.url.clone()
        };

        let text = client.get(&version_json_url).send().await?.text().await?;
        let version_json: serde_json::Value = serde_json::from_str(&text)
            .or_else(|_| serde_json::from_str(text.trim_start_matches('\u{feff}')))
            .map_err(|_| LauncherError::Custom(format!("无法解析版本JSON for {}", version_id)))?;

        (version_id.clone(), version_json, text)
    };

    let mut downloads = Vec::new();

    collect_client_jar(
        &version_json,
        &version_dir,
        &actual_version_id,
        is_mirror,
        base_url,
        &mut downloads,
    )?;

    collect_assets(
        &client,
        &version_json,
        &assets_base_dir,
        is_mirror,
        base_url,
        &mut downloads,
    )
    .await?;

    collect_libraries(
        &version_json,
        &libraries_base_dir,
        is_mirror,
        base_url,
        &mut downloads,
    )?;

    match download_all_files(downloads, window).await {
        Ok(_) => {
            let version_json_path = version_dir.join(format!("{}.json", actual_version_id));
            fs::write(version_json_path, text)?;

            // 通知前端刷新版本列表
            if let Ok(game_dir_str) = config::get_game_dir() {
                let _ = window.emit("game-dir-changed", &game_dir_str);
            }

            Ok(())
        }
        Err(e) => {
            // 仅在本次创建的全新目录时才清理，避免删除用户已有的版本数据（mod、存档等）
            if !dir_existed_before && version_dir.exists() {
                warn!("下载失败，清理新建的版本文件夹: {}", version_dir.display());
                if let Err(cleanup_err) = fs::remove_dir_all(&version_dir) {
                    warn!("清理版本文件夹失败: {}", cleanup_err);
                }
            }
            Err(e)
        }
    }
}

async fn download_modpack_libraries(
    version_json: &serde_json::Value,
    libraries_base_dir: &PathBuf,
    is_mirror: bool,
    base_url: &str,
    window: &Window,
) -> Result<(), LauncherError> {
    let mut downloads = Vec::new();

    collect_libraries(
        version_json,
        libraries_base_dir,
        is_mirror,
        base_url,
        &mut downloads,
    )?;

    if downloads.is_empty() {
        return Ok(());
    }

    info!("下载整合包库文件: {} 个", downloads.len());

    download_all_files(downloads, window).await
}

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
    let client_hash = client_info["sha1"].as_str().map(|h| h.to_string());
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
        assets_index_url
            .replace("https://launchermeta.mojang.com", base_url)
            .replace("https://piston-meta.mojang.com", base_url)
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
                hash: Some(hash.to_string()),
            });
        }
    }

    Ok(())
}

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

        if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
            if let Some(job) = create_library_job(artifact, libraries_base_dir, is_mirror, base_url)
            {
                downloads.push(job);
            }
        } else {
            // 无 downloads.artifact（Forge 库常见），从 name 构建任务
            if let Some(job) =
                create_library_job_from_name(lib, libraries_base_dir, is_mirror, base_url)
            {
                downloads.push(job);
            }
        }

        collect_natives_library(lib, libraries_base_dir, is_mirror, base_url, downloads);
    }

    Ok(())
}

use crate::utils::minecraft::maven_name_to_path;

fn create_library_job_from_name(
    lib: &serde_json::Value,
    libraries_base_dir: &PathBuf,
    is_mirror: bool,
    base_url: &str,
) -> Option<DownloadJob> {
    let name = lib["name"].as_str()?;
    let maven_path = maven_name_to_path(name, None, "jar")?;

    let target_path = libraries_base_dir.join(&maven_path);

    if target_path.exists() {
        return None;
    }

    let lib_url = lib.get("url").and_then(|u| u.as_str());

    let download_url = if is_mirror {
        format!("{}/maven/{}", base_url, maven_path)
    } else if let Some(url) = lib_url {
        let base = if url.ends_with('/') {
            url.to_string()
        } else {
            format!("{}/", url)
        };
        format!("{}{}", base, maven_path)
    } else {
        format!("https://repo1.maven.org/maven2/{}", maven_path)
    };

    let fallback_url = if is_mirror {
        if let Some(url) = lib_url {
            let base = if url.ends_with('/') {
                url.to_string()
            } else {
                format!("{}/", url)
            };
            Some(format!("{}{}", base, maven_path))
        } else {
            Some(format!("https://repo1.maven.org/maven2/{}", maven_path))
        }
    } else {
        Some(format!(
            "https://bmclapi2.bangbang93.com/maven/{}",
            maven_path
        ))
    };

    Some(DownloadJob {
        url: download_url,
        fallback_url,
        path: target_path,
        size: 0,
        hash: None,
    })
}

fn should_download_library(lib: &serde_json::Value) -> bool {
    // LWJGL natives 无视 rules 评估，始终下载
    if lib["name"]
        .as_str()
        .map_or(false, |name| name.contains("lwjgl"))
        && lib.get("natives").is_some()
    {
        return true;
    }

    crate::utils::minecraft::evaluate_rules(lib.get("rules"))
}

fn create_library_job(
    artifact: &serde_json::Value,
    libraries_base_dir: &PathBuf,
    is_mirror: bool,
    base_url: &str,
) -> Option<DownloadJob> {
    let url = artifact["url"].as_str()?;
    let path = artifact["path"].as_str()?;
    let size = artifact["size"].as_u64().unwrap_or(0);
    let hash = artifact["sha1"].as_str().map(|h| h.to_string());

    let download_url = if is_mirror {
        url.replace(
            "https://libraries.minecraft.net",
            &format!("{}/libraries", base_url),
        )
        .replace(
            "https://maven.minecraftforge.net",
            &format!("{}/maven", base_url),
        )
        .replace(
            "https://maven.neoforged.net/releases",
            &format!("{}/maven", base_url),
        )
    } else {
        url.to_string()
    };

    Some(DownloadJob {
        url: download_url,
        fallback_url: if is_mirror {
            Some(url.to_string())
        } else {
            None
        },
        path: libraries_base_dir.join(path),
        size,
        hash,
    })
}

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
    let os_key = if current_os == "macos" {
        "osx"
    } else {
        current_os
    };

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

        if let Some(artifact) = lib
            .get("downloads")
            .and_then(|d| d.get("classifiers"))
            .and_then(|c| c.get(os_classifier))
        {
            if let Some(job) = create_library_job(artifact, libraries_base_dir, is_mirror, base_url)
            {
                downloads.push(job);
                continue;
            }
        }

        if let Some(artifact) = lib.get("classifiers").and_then(|c| c.get(os_classifier)) {
            if let Some(job) = create_library_job(artifact, libraries_base_dir, is_mirror, base_url)
            {
                downloads.push(job);
                continue;
            }
        }

        if let Some(job) = create_natives_job_from_name(
            lib,
            os_classifier,
            libraries_base_dir,
            is_mirror,
            base_url,
        ) {
            downloads.push(job);
        }
    }
}

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
        if cfg!(target_pointer_width = "64") {
            "64"
        } else {
            "32"
        },
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
        format!("{}/libraries/{}", base_url, natives_path)
    } else {
        natives_url.clone()
    };

    Some(DownloadJob {
        url: download_url,
        fallback_url: if is_mirror { Some(natives_url) } else { None },
        path: libraries_base_dir.join(&natives_path),
        size: 0,
        hash: None,
    })
}
