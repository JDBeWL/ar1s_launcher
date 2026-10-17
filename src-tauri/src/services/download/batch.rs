//! 批量下载调度
//!
//! 按配置线程数限并发，跳过 .download_state 里已完成项，退避重试并响应取消

use super::file::download_file;
use super::http::get_http_client;
use super::state::DownloadState;
use crate::errors::LauncherError;
use crate::models::{DownloadJob, DownloadProgress, DownloadStatus};
use crate::services::config::load_config;
use log::{debug, info, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::async_runtime;
use tauri::{Emitter, Listener, Window};
use tokio::sync::Mutex;

/// 取消标志跨下载会话保留，不会随单次下载结束销毁
static CANCEL_FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

fn get_cancel_flag() -> Arc<AtomicBool> {
    CANCEL_FLAG
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

/// 重置取消标志（在开始新下载时调用）
pub fn reset_cancel_flag() {
    if let Some(flag) = CANCEL_FLAG.get() {
        flag.store(false, Ordering::Release);
    }
}

pub fn set_cancel_flag() {
    get_cancel_flag().store(true, Ordering::Release);
}

pub async fn download_all_files(
    jobs: Vec<DownloadJob>,
    window: &Window,
) -> Result<(), LauncherError> {
    let config = load_config()?;
    let threads = config.download_threads as usize;

    let http = get_http_client();

    let version_id = jobs
        .first()
        .and_then(|j| j.path.parent())
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string());

    // 状态文件放在游戏目录下，避免被其他程序访问
    let game_dir = std::path::PathBuf::from(&config.game_dir);
    let state_dir = game_dir.join(".download_state");
    std::fs::create_dir_all(&state_dir)?;
    let state_file = state_dir.join(format!("{}.json", version_id));

    let download_state = Arc::new(Mutex::new(
        DownloadState::load_from_file(&state_file).unwrap_or_else(DownloadState::new),
    ));

    let filtered_jobs: Vec<DownloadJob> = {
        let mut state = download_state.lock().await;
        jobs.iter()
            .filter(|job| {
                if state.is_completed(&job.url) {
                    if job.path.exists() {
                        false
                    } else {
                        warn!(
                            "File marked as completed but missing on disk: {}, re-downloading",
                            job.path.display()
                        );
                        state.completed_files.remove(&job.url);
                        state.mark_dirty();
                        true
                    }
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    };

    // 必须在过滤之后统计，才能反映真实状态
    let (completed_count, resumed_bytes) = {
        let state = download_state.lock().await;
        let completed = state.completed_files.len() as u64;
        let completed_bytes: u64 = jobs
            .iter()
            .filter(|j| state.is_completed(&j.url))
            .map(|j| j.size)
            .sum();
        let partial_bytes: u64 = state.partial_downloads.values().sum();
        (completed, completed_bytes + partial_bytes)
    };

    let total_size: u64 = jobs.iter().map(|j| j.size).sum();

    if filtered_jobs.is_empty() {
        debug!("All files already downloaded, skipping");
        emit_completed_progress(window, total_size, total_size);
        return Ok(());
    }

    info!(
        "Resuming download - {} files completed, {} remaining, {} bytes resumed",
        completed_count,
        filtered_jobs.len(),
        resumed_bytes
    );

    reset_cancel_flag();
    let global_cancel = get_cancel_flag();

    let files_downloaded = Arc::new(AtomicU64::new(completed_count));
    let bytes_downloaded = Arc::new(AtomicU64::new(resumed_bytes));
    let bytes_since_last = Arc::new(AtomicU64::new(0));
    let state = Arc::new(AtomicBool::new(true));
    let was_cancelled = Arc::new(AtomicBool::new(false));
    let error_occurred = Arc::new(tokio::sync::Mutex::new(None::<String>));

    // 用 listen 而非 once，以支持多次取消尝试
    let state_clone = state.clone();
    let was_cancelled_clone = was_cancelled.clone();
    let download_state_clone = download_state.clone();
    let state_file_clone = state_file.clone();
    let listener_id = window.listen("cancel-download", move |_| {
        if state_clone.swap(false, Ordering::AcqRel) {
            was_cancelled_clone.store(true, Ordering::Release);
            // 取消时立即保存状态，供下次续传
            let download_state = download_state_clone.clone();
            let state_file = state_file_clone.clone();
            std::thread::spawn(move || {
                if let Ok(mut state) = download_state.try_lock() {
                    let _ = state.save_to_file(&state_file);
                }
            });
        }
    });

    let reporter_handle = spawn_progress_reporter(
        bytes_downloaded.clone(),
        bytes_since_last.clone(),
        state.clone(),
        window.clone(),
        total_size,
    );

    let state_saver_handle =
        spawn_state_saver(download_state.clone(), state_file.clone(), state.clone());

    let semaphore = Arc::new(tokio::sync::Semaphore::new(threads));
    let mut handles = vec![];

    for job in filtered_jobs {
        if !state.load(Ordering::Acquire) || global_cancel.load(Ordering::Acquire) {
            break;
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let global_cancel_clone = global_cancel.clone();
        let handle = spawn_download_task(
            job,
            http,
            state.clone(),
            global_cancel_clone,
            files_downloaded.clone(),
            bytes_downloaded.clone(),
            bytes_since_last.clone(),
            error_occurred.clone(),
            download_state.clone(),
            permit,
        );
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    state.store(false, Ordering::Release);
    reporter_handle.await?;
    state_saver_handle.await?;

    window.unlisten(listener_id);

    {
        let mut state = download_state.lock().await;
        if state.dirty {
            if let Err(e) = state.save_to_file(&state_file) {
                warn!("Failed to write final state file: {}", e);
            }
        }
    }

    if was_cancelled.load(Ordering::Acquire) {
        emit_cancelled_progress(window, bytes_downloaded.load(Ordering::Relaxed), total_size);
        return Err(LauncherError::Custom("下载已取消".to_string()));
    }

    let error_message = {
        let error_guard = error_occurred.lock().await;
        error_guard.clone()
    };

    if let Some(error_msg) = error_message {
        emit_error_progress(
            window,
            bytes_downloaded.load(Ordering::Relaxed),
            total_size,
            &error_msg,
        );
        return Err(LauncherError::Custom(error_msg));
    }

    let failed_list: Vec<String> = {
        let state = download_state.lock().await;
        state.failed_files.iter().cloned().collect()
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

    if failed_list.is_empty() {
        let _ = std::fs::remove_file(&state_file);
        if let Ok(entries) = std::fs::read_dir(&state_dir) {
            if entries.count() == 0 {
                let _ = std::fs::remove_dir(&state_dir);
            }
        }
    }

    emit_completed_progress(window, bytes_downloaded.load(Ordering::Relaxed), total_size);

    Ok(())
}

fn spawn_progress_reporter(
    bytes_downloaded: Arc<AtomicU64>,
    bytes_since_last: Arc<AtomicU64>,
    state: Arc<AtomicBool>,
    window: Window,
    total_size: u64,
) -> tauri::async_runtime::JoinHandle<()> {
    let report_interval = Duration::from_millis(200);

    async_runtime::spawn(async move {
        while state.load(Ordering::Acquire) {
            tokio::time::sleep(report_interval).await;
            if !state.load(Ordering::Acquire) {
                break;
            }

            let current_bytes = bytes_downloaded.load(Ordering::Relaxed);
            let bytes_since = bytes_since_last.swap(0, Ordering::Relaxed);

            if bytes_since == 0 {
                continue;
            }

            let speed = (bytes_since as f64 / 1024.0) / report_interval.as_secs_f64();
            let progress_percent = if total_size > 0 {
                (current_bytes as f64 / total_size as f64 * 100.0).round() as u8
            } else {
                0
            };

            let progress = DownloadProgress {
                bytes_downloaded: current_bytes,
                total_bytes: total_size,
                speed,
                status: DownloadStatus::Downloading,
                percent: progress_percent,
                error: None,
            };
            let _ = window.emit("download-progress", &progress);
        }
    })
}

fn spawn_state_saver(
    download_state: Arc<Mutex<DownloadState>>,
    state_file: std::path::PathBuf,
    running: Arc<AtomicBool>,
) -> tauri::async_runtime::JoinHandle<()> {
    let save_interval = Duration::from_secs(30);

    async_runtime::spawn(async move {
        while running.load(Ordering::Acquire) {
            tokio::time::sleep(save_interval).await;
            if !running.load(Ordering::Acquire) {
                break;
            }

            let mut state = download_state.lock().await;
            if state.dirty {
                if let Err(e) = state.save_to_file(&state_file) {
                    warn!("Failed to save download state: {}", e);
                } else {
                    debug!("Download state saved to {}", state_file.display());
                }
            }
        }
    })
}

fn spawn_download_task(
    job: DownloadJob,
    http: &'static reqwest::Client,
    state: Arc<AtomicBool>,
    global_cancel: Arc<AtomicBool>,
    files_downloaded: Arc<AtomicU64>,
    bytes_downloaded: Arc<AtomicU64>,
    bytes_since_last: Arc<AtomicU64>,
    error_occurred: Arc<tokio::sync::Mutex<Option<String>>>,
    download_state: Arc<Mutex<DownloadState>>,
    permit: tokio::sync::OwnedSemaphorePermit,
) -> tauri::async_runtime::JoinHandle<Result<(), LauncherError>> {
    async_runtime::spawn(async move {
        if !state.load(Ordering::Acquire) || global_cancel.load(Ordering::Acquire) {
            drop(permit);
            return Ok::<(), LauncherError>(());
        }

        {
            let mut state = download_state.lock().await;
            state.start_download(job.url.clone(), job.path.clone());
        }

        let mut current_job_error: Option<LauncherError> = None;
        let mut job_succeeded = false;

        const MAX_JOB_RETRIES: usize = 5;
        for retry in 0..MAX_JOB_RETRIES {
            if !state.load(Ordering::Acquire) || global_cancel.load(Ordering::Acquire) {
                break;
            }

            // 重试 2 次后改用官方源 fallback_url
            let current_url = if retry >= 2 && job.url.contains("bmclapi2.bangbang93.com") {
                job.fallback_url.as_deref().unwrap_or(&job.url)
            } else {
                &job.url
            };

            let attempt_str = if retry == 0 {
                "attempt 1".to_string()
            } else {
                format!("retry {}/{}", retry, MAX_JOB_RETRIES - 1)
            };
            debug!("Downloading file: {} ({})", current_url, attempt_str);

            match download_file(
                http,
                &job,
                current_url,
                &state,
                &global_cancel,
                &bytes_downloaded,
                &bytes_since_last,
            )
            .await
            {
                Ok(_) => {
                    files_downloaded.fetch_add(1, Ordering::Relaxed);
                    current_job_error = None;
                    job_succeeded = true;
                    break;
                }
                Err(e) => {
                    if e.to_string().contains("cancelled") {
                        break;
                    }
                    warn!("Download failed: {} ({}) - {}", current_url, attempt_str, e);
                    current_job_error = Some(e);
                    if retry < MAX_JOB_RETRIES - 1 {
                        let backoff = Duration::from_secs(1 << retry);
                        debug!("Waiting {:?} before next attempt", backoff);
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }

        {
            let mut state = download_state.lock().await;
            if job_succeeded {
                state.mark_completed(job.url.clone());
            } else {
                state.mark_failed(job.url.clone());
                if let Some(e) = current_job_error {
                    let mut error_guard = error_occurred.lock().await;
                    if error_guard.is_none() {
                        *error_guard = Some(e.to_string());
                    }
                }
            }
            state.finish_download(&job.url);
        }

        drop(permit);
        Ok::<(), LauncherError>(())
    })
}

fn emit_cancelled_progress(window: &Window, bytes: u64, total: u64) {
    let percent = if total > 0 {
        (bytes as f64 / total as f64 * 100.0).round() as u8
    } else {
        0
    };

    let _ = window.emit(
        "download-progress",
        &DownloadProgress {
            bytes_downloaded: bytes,
            total_bytes: total,
            speed: 0.0,
            status: DownloadStatus::Cancelled,
            percent,
            error: None,
        },
    );
}

fn emit_error_progress(window: &Window, bytes: u64, total: u64, error_msg: &str) {
    let percent = if total > 0 {
        (bytes as f64 / total as f64 * 100.0).round() as u8
    } else {
        0
    };

    let _ = window.emit(
        "download-progress",
        &DownloadProgress {
            bytes_downloaded: bytes,
            total_bytes: total,
            speed: 0.0,
            status: DownloadStatus::Error,
            percent,
            error: Some(error_msg.to_string()),
        },
    );
}

fn emit_completed_progress(window: &Window, bytes: u64, total: u64) {
    let _ = window.emit(
        "download-progress",
        &DownloadProgress {
            bytes_downloaded: bytes,
            total_bytes: total,
            speed: 0.0,
            status: DownloadStatus::Completed,
            percent: 100,
            error: None,
        },
    );
}
