use crate::errors::LauncherError;
use crate::models::{DownloadJob, VersionManifest};
use crate::services::config::load_config;
use crate::utils::file_utils;
use reqwest;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime;
use tauri::{Emitter, Listener, Window};
use tokio::io::AsyncWriteExt;

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
    fs::create_dir_all(&version_dir)?;
    let (libraries_base_dir, assets_base_dir) =
        (game_dir.join("libraries"), game_dir.join("assets"));

    let client = reqwest::Client::new();
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

    // 资源文件索引
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

            let is_lwjgl = lib["name"]
                .as_str()
                .map_or(false, |name| name.contains("lwjgl"));
            let has_natives = lib.get("natives").is_some();

            if is_lwjgl && has_natives {
                should_download = true;
            }

            if !should_download {
                continue;
            }

            if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                if let (Some(url), Some(path)) =
                    (artifact["url"].as_str(), artifact["path"].as_str())
                {
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
                    let file_path = libraries_base_dir.join(path);
                    downloads.push(DownloadJob {
                        url: download_url,
                        fallback_url: if is_mirror {
                            Some(url.to_string())
                        } else {
                            None
                        },
                        path: file_path,
                        size,
                        hash,
                    });
                }
            }

            if let Some(natives) = lib.get("natives") {
                let is_lwjgl = lib["name"]
                    .as_str()
                    .map_or(false, |name| name.contains("lwjgl"));
                for (os_name, classifier_value) in natives.as_object().unwrap() {
                    let os_classifier = classifier_value.as_str().unwrap();
                    if os_name == std::env::consts::OS || is_lwjgl {
                        if let Some(classifiers) =
                            lib.get("downloads").and_then(|d| d.get("classifiers"))
                        {
                            if let Some(artifact) = classifiers.get(os_classifier) {
                                if let (Some(url), Some(path)) =
                                    (artifact["url"].as_str(), artifact["path"].as_str())
                                {
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
                                    let file_path = libraries_base_dir.join(path);
                                    downloads.push(DownloadJob {
                                        url: download_url,
                                        fallback_url: if is_mirror {
                                            Some(url.to_string())
                                        } else {
                                            None
                                        },
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
                                if let (Some(url), Some(path)) =
                                    (artifact["url"].as_str(), artifact["path"].as_str())
                                {
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
                                    let file_path = libraries_base_dir.join(path);
                                    downloads.push(DownloadJob {
                                        url: download_url,
                                        fallback_url: if is_mirror {
                                            Some(url.to_string())
                                        } else {
                                            None
                                        },
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
                                    group_id,
                                    artifact_id,
                                    version,
                                    artifact_id,
                                    version,
                                    classifier
                                )
                            } else if artifact_id == "lwjgl-platform" {
                                format!(
                                    "{}/{}/{}/{}-{}-{}.jar",
                                    group_id,
                                    artifact_id,
                                    version,
                                    artifact_id,
                                    version,
                                    classifier
                                )
                            } else {
                                format!(
                                    "{}/{}/{}/{}-{}-{}.jar",
                                    group_id,
                                    artifact_id,
                                    version,
                                    artifact_id,
                                    version,
                                    classifier
                                )
                            };
                            let natives_url =
                                format!("https://libraries.minecraft.net/{}", natives_path);
                            let download_url = if is_mirror {
                                if artifact_id == "lwjgl" || artifact_id == "lwjgl-platform" {
                                    format!("{}/maven/{}", base_url, natives_path)
                                } else {
                                    natives_url.replace(
                                        "https://libraries.minecraft.net",
                                        &format!("{}/maven", base_url),
                                    )
                                }
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

    // 执行批量下载
    download_all_files(downloads.clone(), window, downloads.len() as u64, mirror).await?;

    // 保存版本元数据文件
    let version_json_path = version_dir.join(format!("{}.json", version_id));
    fs::write(version_json_path, text)?;

    Ok(())
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadState {
    completed_files: Vec<String>,
    failed_files: Vec<String>,
    active_downloads: HashMap<String, std::path::PathBuf>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Downloading,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub progress: u64, // 已下载字节数
    pub total: u64,    // 总字节数
    pub speed: f64,    // 下载速度(KB/s)
    pub status: DownloadStatus,
    pub bytes_downloaded: u64, // 已下载字节数
    pub total_bytes: u64,      // 总字节数
    pub percent: u8,           // 完成百分比(0-100)
}

pub async fn download_all_files(
    jobs: Vec<DownloadJob>,
    window: &Window,
    _total_files: u64,
    _mirror: Option<String>,
) -> Result<(), LauncherError> {
    let config = load_config()?;
    let threads = config.download_threads as usize;

    // 全局复用 HTTP 客户端，减少“starting new connection”
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Ar1s-Launcher/1.0",
        ),
    );
    // 禁用压缩，避免长度不一致；同时显式声明只接受 identity
    default_headers.insert(
        reqwest::header::ACCEPT_ENCODING,
        reqwest::header::HeaderValue::from_static("identity"),
    );

    let http = std::sync::Arc::new(
        reqwest::Client::builder()
            .default_headers(default_headers)
            .no_gzip()
            .no_brotli()
            .no_deflate()
            // 为默认8线程优化连接复用；若线程数不同也按比例生效
            .pool_max_idle_per_host(threads.max(1) * 4)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| LauncherError::Custom(format!("创建HTTP客户端失败: {}", e)))?,
    );

    // 获取版本ID（从第一个下载任务的路径推断）
    let version_id = jobs
        .first()
        .and_then(|j| j.path.parent())
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string());

    // 创建版本特定的状态文件
    let state_file = std::env::temp_dir().join(format!("ar1s_download_state_{}.json", version_id));

    // 在每次下载开始时删除旧的状态文件，以确保全新下载
    if state_file.exists() {
        let _ = std::fs::remove_file(&state_file);
    }

    let download_state = Arc::new(Mutex::new(DownloadState {
        completed_files: Vec::new(),
        failed_files: Vec::new(),
        active_downloads: HashMap::new(),
    }));

    // 创建过滤后的任务列表（不移动原始jobs）
    let filtered_jobs: Vec<DownloadJob> = {
        let state = download_state.lock().await;
        jobs.iter()
            .filter(|job| !state.completed_files.contains(&job.url))
            .cloned()
            .collect()
    };

    // 更新总文件数为实际需要下载的数量
    let _actual_total = jobs.len() as u64;

    let completed_count_from_state = download_state.lock().await.completed_files.len() as u64;
    let total_size_precomputed: u64 = filtered_jobs.iter().map(|j| j.size).sum::<u64>();

    // 创建共享状态
    // TODO: 这里的状态应该改为一个结构体，而不是使用原子类型，以便更好地跟踪状态
    let files_downloaded = Arc::new(AtomicU64::new(completed_count_from_state));
    let bytes_downloaded = Arc::new(AtomicU64::new(0));
    let bytes_since_last = Arc::new(AtomicU64::new(0));
    let state = Arc::new(AtomicBool::new(true)); // true = running, false = cancelled/stopped
    let was_cancelled = Arc::new(AtomicBool::new(false));
    let error_occurred = Arc::new(tokio::sync::Mutex::new(None::<String>));

    // 监听取消下载事件
    let state_clone = state.clone();
    let was_cancelled_clone = was_cancelled.clone();
    window.once("cancel-download", move |_| {
        state_clone.store(false, Ordering::SeqCst);
        was_cancelled_clone.store(true, Ordering::SeqCst);
    });

    // 创建进度报告器
    let reporter_handle = {
        let files_downloaded = files_downloaded.clone();
        let bytes_downloaded = bytes_downloaded.clone();
        let bytes_since_last = bytes_since_last.clone();
        let state = state.clone();
        let window = window.clone();
        let report_interval = Duration::from_millis(200); // 更频繁的更新
        let total_size = total_size_precomputed;

        async_runtime::spawn(async move {
            while state.load(Ordering::SeqCst) {
                tokio::time::sleep(report_interval).await;
                if !state.load(Ordering::SeqCst) {
                    break;
                }

                let _downloaded_count = files_downloaded.load(Ordering::SeqCst);
                let current_bytes = bytes_downloaded.load(Ordering::SeqCst);
                let bytes_since = bytes_since_last.swap(0, Ordering::SeqCst);
                let speed = (bytes_since as f64 / 1024.0) / report_interval.as_secs_f64();
                let progress_percent = if total_size > 0 {
                    (current_bytes as f64 / total_size as f64 * 100.0).round() as u8
                } else {
                    0
                };

                let progress = DownloadProgress {
                    progress: current_bytes,
                    total: total_size,
                    speed,
                    status: DownloadStatus::Downloading,
                    bytes_downloaded: current_bytes,
                    total_bytes: total_size,
                    percent: progress_percent,
                };
                let _ = window.emit("download-progress", &progress);
            }
        })
    };

    // 创建线程池
    let semaphore = Arc::new(tokio::sync::Semaphore::new(threads));
    let mut handles = vec![];

    // 在循环前克隆共享状态
    let state_file_clone = state_file.clone();

    for job in filtered_jobs {
        if !state.load(Ordering::SeqCst) {
            break;
        }

        // 记录正在进行的下载
        {
            let mut state = download_state.lock().await;
            state
                .active_downloads
                .insert(job.url.clone(), job.path.clone());
            std::fs::write(&state_file_clone, serde_json::to_string(&*state)?)?;
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let state = state.clone();
        let files_downloaded = files_downloaded.clone();
        let bytes_downloaded = bytes_downloaded.clone();
        let bytes_since_last = bytes_since_last.clone();
        let error_occurred = error_occurred.clone();
        let job_state_file = state_file.clone();
        let job_download_state = download_state.clone();

        let http = http.clone();
        handles.push(async_runtime::spawn(async move {
            let mut current_job_error: Option<LauncherError> = None;
            let mut job_succeeded = false;

            // The download logic now handles file verification and resuming.
            const MAX_JOB_RETRIES: usize = 5;
            for retry in 0..MAX_JOB_RETRIES {
                // 在重试时尝试切换到官方源
                let current_url = if retry >= 2 && job.url.contains("bmclapi2.bangbang93.com") {
                    // 第3次重试开始，如果当前是镜像源，切换到官方源
                    if let Some(fallback_url) = &job.fallback_url {
                        println!("DEBUG: Switching to official source for: {}", job.url);
                        fallback_url.as_str()
                    } else {
                        &job.url
                    }
                } else {
                    &job.url
                };
                if !state.load(Ordering::SeqCst) {
                    break;
                }

                let attempt_str = if retry == 0 {
                    "attempt 1".to_string()
                } else {
                    format!("retry {}/{}", retry, MAX_JOB_RETRIES - 1)
                };
                println!("DEBUG: Downloading file: {} ({})", current_url, attempt_str);

                match download_file(
                    http.clone(),
                    &job,
                    current_url,
                    &state,
                    &bytes_downloaded,
                    &bytes_since_last,
                )
                .await
                {
                    Ok(_) => {
                        files_downloaded.fetch_add(1, Ordering::SeqCst);
                        current_job_error = None;
                        job_succeeded = true;
                        break;
                    }
                    Err(e) => {
                        println!(
                            "ERROR: Download failed: {} ({}) - {}",
                            current_url, attempt_str, e
                        );
                        current_job_error = Some(e);
                        if retry < MAX_JOB_RETRIES - 1 {
                            let backoff = Duration::from_secs(1 << retry);
                            println!("DEBUG: Waiting {:?} before next attempt", backoff);
                            tokio::time::sleep(backoff).await;
                        }
                    }
                }
            }

            // 克隆共享状态，以便在下载完成时更新
            // TODO: 这里应该使用一个结构体来跟踪状态，而不是使用原子类型，以便更好地跟踪状态
            let state_file_clone = job_state_file;
            let download_state_clone = job_download_state;

            if job_succeeded {
                // 记录成功下载
                let mut state = download_state_clone.lock().await;
                state.completed_files.push(job.url.clone());
                state.active_downloads.remove(&job.url);
                std::fs::write(&state_file_clone, serde_json::to_string(&*state)?)?;
            } else {
                // 下载失败
                if let Some(e) = current_job_error {
                    // 不取消全局，记录失败
                    let mut error_guard = error_occurred.lock().await;
                    if error_guard.is_none() {
                        *error_guard = Some(e.to_string());
                    }

                    // 记录失败下载
                    let mut state = download_state_clone.lock().await;
                    state.failed_files.push(job.url.clone());
                    state.active_downloads.remove(&job.url);
                    std::fs::write(&state_file_clone, serde_json::to_string(&*state)?)?;
                }
            }
            drop(permit);
            Ok::<(), LauncherError>(())
        }));
    }

    // 等待所有线程完成
    for handle in handles {
        let _ = handle.await;
    }

    // 取消下载
    state.store(false, Ordering::SeqCst);
    reporter_handle.await?;

    if was_cancelled.load(Ordering::SeqCst) {
        let final_bytes = bytes_downloaded.load(Ordering::SeqCst);
        let total_bytes = total_size_precomputed;
        let final_percent = if total_bytes > 0 {
            (final_bytes as f64 / total_bytes as f64 * 100.0).round() as u8
        } else {
            0
        };

        let _ = window.emit(
            "download-progress",
            &DownloadProgress {
                progress: final_bytes,
                total: total_bytes,
                speed: 0.0,
                status: DownloadStatus::Cancelled,
                bytes_downloaded: final_bytes,
                total_bytes,
                percent: final_percent,
            },
        );
        return Err(LauncherError::Custom("下载已取消".to_string()));
    }

    // 如果有部分失败，发出摘要事件，但不报错，允许整体完成
    let failed_list: Vec<String> = {
        let state = download_state.lock().await;
        state.failed_files.clone()
    };
    if !failed_list.is_empty() {
        let _ = window.emit(
            "download-summary",
            &serde_json::json!({
                "status": "partial",
                "failed_count": failed_list.len(),
                "failed": failed_list,
            }),
        );
    }

    // 下载完成 - 确保只发送一次完成事件
    let _ = window.emit(
        "download-progress",
        &DownloadProgress {
            progress: bytes_downloaded.load(Ordering::SeqCst),
            total: total_size_precomputed,
            speed: 0.0,
            status: DownloadStatus::Completed,
            bytes_downloaded: bytes_downloaded.load(Ordering::SeqCst),
            total_bytes: total_size_precomputed,
            percent: 100,
        },
    );

    Ok(())
}

async fn download_file(
    http: std::sync::Arc<reqwest::Client>,
    job: &DownloadJob,
    url: &str,
    state: &Arc<AtomicBool>,
    bytes_downloaded: &Arc<AtomicU64>,
    bytes_since_last: &Arc<AtomicU64>,
) -> Result<(), LauncherError> {
    // 1. 验证文件完整性，如果文件有效则跳过下载
    if job.path.exists() {
        if file_utils::verify_file(&job.path, &job.hash, job.size)? {
            println!(
                "DEBUG: File already exists and is valid, skipping: {}",
                job.path.display()
            );
            bytes_downloaded.fetch_add(job.size, Ordering::SeqCst);
            return Ok(());
        }
        println!(
            "DEBUG: File exists but is invalid, removing and re-downloading: {}",
            job.path.display()
        );
        tokio::fs::remove_file(&job.path).await?; // 清理损坏的文件
    }

    // 2. 尝试从指定 URL 下载（复用共享客户端）
    match download_chunk(
        http.clone(),
        url,
        job,
        state,
        bytes_downloaded,
        bytes_since_last,
    )
    .await
    {
        Ok(_) => Ok(()), // 主 URL 下载成功
        Err(e) => {
            // 3. 如果主 URL 失败，并且是特定错误，则尝试备用 URL
            if let Some(fallback_url) = &job.fallback_url {
                let is_http_error = if let LauncherError::Http(err) = &e {
                    err.status() == Some(reqwest::StatusCode::NOT_FOUND) || err.is_timeout()
                } else {
                    false
                };
                let err_str = e.to_string();
                let should_fallback = is_http_error
                    || err_str.contains("size or hash mismatch")
                    || err_str.contains("File size mismatch")
                    || err_str.contains("Unexpected Content-Length")
                    || err_str.contains("Unexpected Content-Type");

                if should_fallback {
                    println!(
                        "DEBUG: Primary URL {} failed ({}), trying fallback: {}",
                        job.url, e, fallback_url
                    );
                    return download_chunk(
                        http.clone(),
                        fallback_url,
                        job,
                        state,
                        bytes_downloaded,
                        bytes_since_last,
                    )
                    .await;
                }
            }
            // 4. 如果没有备用 URL 或错误不符合重试条件，则返回原始错误
            Err(e)
        }
    }
}

async fn download_chunk(
    client: std::sync::Arc<reqwest::Client>,
    url: &str,
    job: &DownloadJob,
    state: &Arc<AtomicBool>,
    bytes_downloaded: &Arc<AtomicU64>,
    bytes_since_last: &Arc<AtomicU64>,
) -> Result<(), LauncherError> {
    let tmp_path = job.path.with_extension("part");
    let mut bytes_added_this_attempt: u64 = 0;

    let result = async {
        if let Some(parent) = job.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .await?;

        let mut response = client.get(url).send().await?.error_for_status()?;

        if let Some(len_hdr) = response.headers().get(reqwest::header::CONTENT_LENGTH) {
            if let Ok(len_str) = len_hdr.to_str() {
                if let Ok(remote_len) = len_str.parse::<u64>() {
                    if remote_len == 0 && job.size > 0 {
                        return Err(LauncherError::Custom(format!(
                            "Unexpected Content-Length 0 for {}, expected {}",
                            url, job.size
                        )));
                    }
                }
            }
        }
        if let Some(ct) = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
        {
            let ct_lower = ct.to_ascii_lowercase();
            if ct_lower.starts_with("text/")
                || ct_lower.contains("json")
                || ct_lower.contains("html")
            {
                return Err(LauncherError::Custom(format!(
                    "Unexpected Content-Type {} for {}",
                    ct, url
                )));
            }
        }

        while let Some(chunk) = response.chunk().await? {
            if !state.load(Ordering::SeqCst) {
                return Err(LauncherError::Custom("Download cancelled".to_string()));
            }
            file.write_all(&chunk).await?;
            let len = chunk.len() as u64;
            bytes_downloaded.fetch_add(len, Ordering::Relaxed);
            bytes_since_last.fetch_add(len, Ordering::Relaxed);
            bytes_added_this_attempt += len;
        }

        if !file_utils::verify_file(&tmp_path, &job.hash, job.size)? {
            return Err(LauncherError::Custom(format!(
                "File verification failed for {}: size or hash mismatch.",
                tmp_path.display()
            )));
        }

        if let Some(parent) = job.path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        if tokio::fs::metadata(&job.path).await.is_ok() {
            let _ = tokio::fs::remove_file(&job.path).await;
        }
        tokio::fs::rename(&tmp_path, &job.path).await?;

        Ok::<(), LauncherError>(())
    }
    .await;

    if result.is_err() {
        bytes_downloaded.fetch_sub(bytes_added_this_attempt, Ordering::Relaxed);
    }

    result
}



async fn fetch_versions(
    client: &reqwest::Client,
    url: &str,
) -> Result<VersionManifest, LauncherError> {
    let config = load_config()?;
    let log_dir = PathBuf::from(config.game_dir).join("logs");
    fs::create_dir_all(&log_dir)?;

    let log_file = log_dir.join("network_debug.log");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| {
            LauncherError::Custom(format!("无法创建日志文件 {}: {}", log_file.display(), e))
        })?;

    writeln!(log, "[DEBUG] 准备发送请求到: {}", url)?;

    let response = client.get(url).send().await?;

    writeln!(log, "[DEBUG] 响应状态码: {}", response.status())?;

    let text = response.text().await?;
    let text = text.trim_start_matches('\u{feff}').to_string();

    let manifest = serde_json::from_str::<VersionManifest>(&text).map_err(|e| {
        writeln!(log, "JSON parse error: {}", e).ok();
        LauncherError::Json(e)
    })?;

    writeln!(
        log,
        "Parsed manifest with {} versions",
        manifest.versions.len()
    )?;
    Ok(manifest)
}

/// 获取 Minecraft 版本列表
pub async fn get_versions() -> Result<VersionManifest, LauncherError> {
    // 确保目录存在
    let config = load_config()?;
    let log_dir = PathBuf::from(config.game_dir).join("logs");
    fs::create_dir_all(&log_dir)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| LauncherError::Custom(format!("创建HTTP客户端失败: {}", e)))?;

    let urls = [
        "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json",
        "https://launchermeta.mojang.com/mc/game/version_manifest.json",
    ];

    let log_file = log_dir.join("version_fetch.log");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| {
            LauncherError::Custom(format!("无法创建日志文件 {}: {}", log_file.display(), e))
        })?;

    writeln!(
        log,
        "[{}] 开始获取版本列表",
        chrono::Local::now().to_rfc3339()
    )?;

    for (i, url) in urls.iter().enumerate() {
        writeln!(log, "尝试第{}个源: {}", i + 1, url)?;
        match fetch_versions(&client, url).await {
            Ok(manifest) => {
                writeln!(log, "成功获取版本列表，共{}个版本", manifest.versions.len())?;
                return Ok(manifest);
            }
            Err(e) => {
                writeln!(log, "获取失败: {}", e)?;
                continue;
            }
        }
    }
    Err(LauncherError::Custom(
        "所有源都尝试失败，请检查网络连接".to_string(),
    ))
}
