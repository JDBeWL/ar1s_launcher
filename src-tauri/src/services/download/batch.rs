//! 批量下载逻辑（支持断点续传）

use super::file::download_file;
use super::http::get_http_client;
use super::state::DownloadState;
use crate::errors::LauncherError;
use crate::models::{DownloadJob, DownloadProgress, DownloadStatus};
use crate::services::config::load_config;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime;
use tauri::{Emitter, Listener, Window};
use tokio::sync::Mutex;

/// 全局取消标志，用于跨下载会话的取消控制
static CANCEL_FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

/// 获取或初始化全局取消标志
fn get_cancel_flag() -> Arc<AtomicBool> {
    CANCEL_FLAG
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

/// 重置取消标志（在开始新下载时调用）
pub fn reset_cancel_flag() {
    if let Some(flag) = CANCEL_FLAG.get() {
        flag.store(false, Ordering::SeqCst);
    }
}

/// 设置取消标志（在取消下载时调用）
pub fn set_cancel_flag() {
    get_cancel_flag().store(true, Ordering::SeqCst);
}

/// 批量下载所有文件（支持断点续传）
pub async fn download_all_files(
    jobs: Vec<DownloadJob>,
    window: &Window,
    _total_files: u64,
    _mirror: Option<String>,
) -> Result<(), LauncherError> {
    let config = load_config()?;
    let threads = config.download_threads as usize;

    // 使用全局 HTTP 客户端
    let http = get_http_client()?;

    // 获取版本 ID
    let version_id = jobs
        .first()
        .and_then(|j| j.path.parent())
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string());

    // 创建状态文件路径（存储在游戏目录下，避免被其他程序访问）
    let game_dir = std::path::PathBuf::from(&config.game_dir);
    let state_dir = game_dir.join(".download_state");
    std::fs::create_dir_all(&state_dir)?;
    let state_file = state_dir.join(format!("{}.json", version_id));

    // 尝试从状态文件恢复（断点续传）
    let download_state = Arc::new(Mutex::new(
        DownloadState::load_from_file(&state_file).unwrap_or_else(DownloadState::new)
    ));

    // 计算已完成的文件和已下载的字节数
    let (completed_count, resumed_bytes) = {
        let state = download_state.lock().await;
        let completed = state.completed_files.len() as u64;
        // 计算已完成文件的总大小
        let completed_bytes: u64 = jobs
            .iter()
            .filter(|j| state.is_completed(&j.url))
            .map(|j| j.size)
            .sum();
        // 加上部分下载的字节数
        let partial_bytes: u64 = state.partial_downloads.values().sum();
        (completed, completed_bytes + partial_bytes)
    };

    // 过滤已完成的任务
    let filtered_jobs: Vec<DownloadJob> = {
        let state = download_state.lock().await;
        jobs.iter()
            .filter(|job| !state.is_completed(&job.url))
            .cloned()
            .collect()
    };

    // 计算总大小（包括已完成的）
    let total_size: u64 = jobs.iter().map(|j| j.size).sum();

    if filtered_jobs.is_empty() {
        println!("DEBUG: All files already downloaded, skipping");
        emit_completed_progress(window, total_size, total_size);
        return Ok(());
    }

    println!(
        "DEBUG: Resuming download - {} files completed, {} remaining, {} bytes resumed",
        completed_count,
        filtered_jobs.len(),
        resumed_bytes
    );

    // 重置全局取消标志
    reset_cancel_flag();
    let global_cancel = get_cancel_flag();

    // 创建共享状态
    let files_downloaded = Arc::new(AtomicU64::new(completed_count));
    let bytes_downloaded = Arc::new(AtomicU64::new(resumed_bytes));
    let bytes_since_last = Arc::new(AtomicU64::new(0));
    let state = Arc::new(AtomicBool::new(true));
    let was_cancelled = Arc::new(AtomicBool::new(false));
    let error_occurred = Arc::new(tokio::sync::Mutex::new(None::<String>));

    // 监听取消下载事件（使用 listen 而非 once，以支持多次取消尝试）
    let state_clone = state.clone();
    let was_cancelled_clone = was_cancelled.clone();
    let download_state_clone = download_state.clone();
    let state_file_clone = state_file.clone();
    let listener_id = window.listen("cancel-download", move |_| {
        // 检查是否已经取消，避免重复处理
        if state_clone.swap(false, Ordering::SeqCst) {
            was_cancelled_clone.store(true, Ordering::SeqCst);
            // 取消时异步保存状态以便下次续传
            let download_state = download_state_clone.clone();
            let state_file = state_file_clone.clone();
            // 使用 spawn_blocking 来处理可能阻塞的操作
            std::thread::spawn(move || {
                // 尝试获取锁并保存状态
                if let Ok(state) = download_state.try_lock() {
                    let _ = state.save_to_file(&state_file);
                }
            });
        }
    });

    // 创建进度报告器
    let reporter_handle = spawn_progress_reporter(
        files_downloaded.clone(),
        bytes_downloaded.clone(),
        bytes_since_last.clone(),
        state.clone(),
        window.clone(),
        total_size,
    );

    // 定期保存状态（每 30 秒）
    let state_saver_handle = spawn_state_saver(
        download_state.clone(),
        state_file.clone(),
        state.clone(),
    );

    // 执行并发下载
    let semaphore = Arc::new(tokio::sync::Semaphore::new(threads));
    let mut handles = vec![];

    for job in filtered_jobs {
        // 检查本地状态和全局取消标志
        if !state.load(Ordering::SeqCst) || global_cancel.load(Ordering::SeqCst) {
            break;
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let global_cancel_clone = global_cancel.clone();
        let handle = spawn_download_task(
            job,
            http.clone(),
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

    // 等待所有任务完成
    for handle in handles {
        let _ = handle.await;
    }

    // 停止进度报告器和状态保存器
    state.store(false, Ordering::SeqCst);
    reporter_handle.await?;
    state_saver_handle.await?;

    // 取消监听器
    window.unlisten(listener_id);

    // 保存最终状态
    {
        let state = download_state.lock().await;
        if state.dirty {
            if let Err(e) = state.save_to_file(&state_file) {
                println!("WARN: Failed to write final state file: {}", e);
            }
        }
    }

    // 处理取消
    if was_cancelled.load(Ordering::SeqCst) {
        emit_cancelled_progress(window, bytes_downloaded.load(Ordering::SeqCst), total_size);
        return Err(LauncherError::Custom("下载已取消".to_string()));
    }

    // 检查错误
    let error_message = {
        let error_guard = error_occurred.lock().await;
        error_guard.clone()
    };

    if let Some(error_msg) = error_message {
        emit_error_progress(
            window,
            bytes_downloaded.load(Ordering::SeqCst),
            total_size,
            &error_msg,
        );
        return Err(LauncherError::Custom(error_msg));
    }

    // 发送部分失败摘要
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

    // 下载完成，删除状态文件
    if failed_list.is_empty() {
        let _ = std::fs::remove_file(&state_file);
        // 如果状态目录为空，也删除它
        if let Ok(entries) = std::fs::read_dir(&state_dir) {
            if entries.count() == 0 {
                let _ = std::fs::remove_dir(&state_dir);
            }
        }
    }

    // 发送完成事件
    emit_completed_progress(window, bytes_downloaded.load(Ordering::SeqCst), total_size);

    Ok(())
}

/// 启动进度报告器
fn spawn_progress_reporter(
    files_downloaded: Arc<AtomicU64>,
    bytes_downloaded: Arc<AtomicU64>,
    bytes_since_last: Arc<AtomicU64>,
    state: Arc<AtomicBool>,
    window: Window,
    total_size: u64,
) -> tauri::async_runtime::JoinHandle<()> {
    let report_interval = Duration::from_millis(200);

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
                error: None,
            };
            let _ = window.emit("download-progress", &progress);
        }
    })
}

/// 启动状态保存器（定期保存状态以支持断点续传）
fn spawn_state_saver(
    download_state: Arc<Mutex<DownloadState>>,
    state_file: std::path::PathBuf,
    running: Arc<AtomicBool>,
) -> tauri::async_runtime::JoinHandle<()> {
    let save_interval = Duration::from_secs(30);

    async_runtime::spawn(async move {
        while running.load(Ordering::SeqCst) {
            tokio::time::sleep(save_interval).await;
            if !running.load(Ordering::SeqCst) {
                break;
            }

            let state = download_state.lock().await;
            if state.dirty {
                if let Err(e) = state.save_to_file(&state_file) {
                    println!("WARN: Failed to save download state: {}", e);
                } else {
                    println!("DEBUG: Download state saved to {}", state_file.display());
                }
            }
        }
    })
}

/// 启动单个下载任务
fn spawn_download_task(
    job: DownloadJob,
    http: Arc<reqwest::Client>,
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
        // 在开始前再次检查取消状态
        if !state.load(Ordering::SeqCst) || global_cancel.load(Ordering::SeqCst) {
            drop(permit);
            return Ok::<(), LauncherError>(());
        }

        // 记录正在进行的下载
        {
            let mut state = download_state.lock().await;
            state.start_download(job.url.clone(), job.path.clone());
        }

        let mut current_job_error: Option<LauncherError> = None;
        let mut job_succeeded = false;

        const MAX_JOB_RETRIES: usize = 5;
        for retry in 0..MAX_JOB_RETRIES {
            // 在每次重试前检查取消状态
            if !state.load(Ordering::SeqCst) || global_cancel.load(Ordering::SeqCst) {
                break;
            }

            // 在重试时尝试切换到官方源
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
            println!("DEBUG: Downloading file: {} ({})", current_url, attempt_str);

            match download_file(
                http.clone(),
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
                    files_downloaded.fetch_add(1, Ordering::SeqCst);
                    current_job_error = None;
                    job_succeeded = true;
                    break;
                }
                Err(e) => {
                    // 如果是取消导致的错误，不需要重试
                    if e.to_string().contains("cancelled") {
                        break;
                    }
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

        // 更新下载状态
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

/// 发送取消进度事件
fn emit_cancelled_progress(window: &Window, bytes: u64, total: u64) {
    let percent = if total > 0 {
        (bytes as f64 / total as f64 * 100.0).round() as u8
    } else {
        0
    };

    let _ = window.emit(
        "download-progress",
        &DownloadProgress {
            progress: bytes,
            total,
            speed: 0.0,
            status: DownloadStatus::Cancelled,
            bytes_downloaded: bytes,
            total_bytes: total,
            percent,
            error: None,
        },
    );
}

/// 发送错误进度事件
fn emit_error_progress(window: &Window, bytes: u64, total: u64, error_msg: &str) {
    let percent = if total > 0 {
        (bytes as f64 / total as f64 * 100.0).round() as u8
    } else {
        0
    };

    let _ = window.emit(
        "download-progress",
        &DownloadProgress {
            progress: bytes,
            total,
            speed: 0.0,
            status: DownloadStatus::Error,
            bytes_downloaded: bytes,
            total_bytes: total,
            percent,
            error: Some(error_msg.to_string()),
        },
    );
}

/// 发送完成进度事件
fn emit_completed_progress(window: &Window, bytes: u64, total: u64) {
    let _ = window.emit(
        "download-progress",
        &DownloadProgress {
            progress: bytes,
            total,
            speed: 0.0,
            status: DownloadStatus::Completed,
            bytes_downloaded: bytes,
            total_bytes: total,
            percent: 100,
            error: None,
        },
    );
}
