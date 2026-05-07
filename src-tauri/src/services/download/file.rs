//! 单文件下载逻辑（支持断点续传）

use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::utils::file_utils;
use log::{debug, warn};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

pub async fn download_file(
    http: &'static reqwest::Client,
    job: &DownloadJob,
    url: &str,
    state: &Arc<AtomicBool>,
    global_cancel: &Arc<AtomicBool>,
    bytes_downloaded: &Arc<AtomicU64>,
    bytes_since_last: &Arc<AtomicU64>,
) -> Result<(), LauncherError> {
    // 先检查取消状态
    if !state.load(Ordering::Acquire) || global_cancel.load(Ordering::Acquire) {
        return Err(LauncherError::Custom("Download cancelled".to_string()));
    }

    // 1. 检查完整文件是否已存在且有效
    if job.path.exists() {
        match file_utils::verify_and_repair_file(job, &http).await {
            Ok(true) => {
                debug!(
                    "File already exists and is valid, skipping: {}",
                    job.path.display()
                );
                bytes_downloaded.fetch_add(job.size, Ordering::Relaxed);
                return Ok(());
            }
            Ok(false) => {
                debug!(
                    "File exists but is invalid, attempting to download: {}",
                    job.path.display()
                );
            }
            Err(e) => {
                warn!(
                    "File verification failed, attempting to download: {} - {}",
                    job.path.display(),
                    e
                );
            }
        }
    }

    // 2. 尝试从指定 URL 下载（支持断点续传）
    match download_with_resume(http, url, job, state, global_cancel, bytes_downloaded, bytes_since_last).await {
        Ok(_) => Ok(()),
        Err(e) => {
            if e.to_string().contains("cancelled") {
                return Err(e);
            }
            if let Some(fallback_url) = &job.fallback_url {
                if should_try_fallback(&e) {
                    warn!(
                        "Primary URL {} failed ({}), trying fallback: {}",
                        job.url, e, fallback_url
                    );
                    return download_with_resume(
                        http,
                        fallback_url,
                        job,
                        state,
                        global_cancel,
                        bytes_downloaded,
                        bytes_since_last,
                    )
                    .await;
                }
            }
            Err(e)
        }
    }
}

/// 检查是否应该尝试备用 URL
fn should_try_fallback(e: &LauncherError) -> bool {
    let is_http_error = if let LauncherError::Http(err) = e {
        err.status() == Some(reqwest::StatusCode::NOT_FOUND) || err.is_timeout()
    } else {
        false
    };

    let err_str = e.to_string();
    is_http_error
        || err_str.contains("size or hash mismatch")
        || err_str.contains("File size mismatch")
        || err_str.contains("Unexpected Content-Length")
        || err_str.contains("Unexpected Content-Type")
}

async fn download_with_resume(
    client: &'static reqwest::Client,
    url: &str,
    job: &DownloadJob,
    state: &Arc<AtomicBool>,
    global_cancel: &Arc<AtomicBool>,
    bytes_downloaded: &Arc<AtomicU64>,
    bytes_since_last: &Arc<AtomicU64>,
) -> Result<(), LauncherError> {
    let tmp_path = job.path.with_extension("part");
    
    // 检查是否有部分下载的文件
    let existing_size = get_existing_file_size(&tmp_path).await;
    
    // 如果已下载的大小等于或超过预期大小，验证文件
    if existing_size > 0 && job.size > 0 && existing_size >= job.size {
        debug!(
            "Part file complete ({}), verifying: {}",
            existing_size,
            tmp_path.display()
        );
        if file_utils::verify_file(&tmp_path, job.hash.as_deref(), job.size)? {
            // 文件完整，直接移动
            finalize_download(&tmp_path, &job.path).await?;
            bytes_downloaded.fetch_add(job.size, Ordering::Relaxed);
            return Ok(());
        } else {
            // 文件损坏，删除重新下载
            warn!("Part file corrupted, restarting download");
            let _ = tokio::fs::remove_file(&tmp_path).await;
        }
    }

    // 尝试断点续传（直接发送 Range 请求，download_chunk_with_resume 会处理
    // 服务器不支持 Range 返回 200 的情况，无需额外 HEAD 预检请求）
    let resume_from = if existing_size > 0 && job.size > 0 && existing_size < job.size {
        debug!(
            "Resuming download from byte {}: {}",
            existing_size,
            url
        );
        Some(existing_size)
    } else {
        None
    };

    download_chunk_with_resume(
        client,
        url,
        job,
        state,
        global_cancel,
        bytes_downloaded,
        bytes_since_last,
        resume_from,
    )
    .await
}

/// 获取已存在文件的大小
async fn get_existing_file_size(path: &std::path::Path) -> u64 {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.len())
        .unwrap_or(0)
}

async fn download_chunk_with_resume(
    client: &'static reqwest::Client,
    url: &str,
    job: &DownloadJob,
    state: &Arc<AtomicBool>,
    global_cancel: &Arc<AtomicBool>,
    bytes_downloaded: &Arc<AtomicU64>,
    bytes_since_last: &Arc<AtomicU64>,
    resume_from: Option<u64>,
) -> Result<(), LauncherError> {
    let tmp_path = job.path.with_extension("part");
    let mut bytes_added_this_attempt: u64 = 0;
    let start_offset = resume_from.unwrap_or(0);

    let result = async {
        if let Some(parent) = job.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // 根据是否续传选择打开模式
        let mut file = if resume_from.is_some() {
            let mut f = tokio::fs::OpenOptions::new()
                .write(true)
                .open(&tmp_path)
                .await?;
            // 移动到文件末尾
            f.seek(std::io::SeekFrom::End(0)).await?;
            f
        } else {
            tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp_path)
                .await?
        };

        // 构建请求（如果续传，添加 Range 头）
        let mut request = client.get(url);
        if let Some(offset) = resume_from {
            request = request.header("Range", format!("bytes={}-", offset));
            // 已下载的部分计入进度
            bytes_downloaded.fetch_add(offset, Ordering::Relaxed);
            bytes_added_this_attempt += offset;
        }

        let response = request.send().await?;
        
        // 检查响应状态
        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(LauncherError::Custom(format!(
                "HTTP error {} for {}",
                status, url
            )));
        }

        // 如果请求了 Range 但服务器返回 200（而非 206），说明不支持续传
        if resume_from.is_some() && status == reqwest::StatusCode::OK {
            debug!("Server returned 200 instead of 206, restarting download");
            // 回滚已计数的字节
            bytes_downloaded.fetch_sub(start_offset, Ordering::Relaxed);
            bytes_added_this_attempt -= start_offset;
            // 重新打开文件并截断
            drop(file);
            file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp_path)
                .await?;
        }

        // 验证 Content-Type（仅对新下载）
        if resume_from.is_none() {
            validate_content_type(&response, url)?;
        }

        // 验证 Content-Length
        validate_content_length(&response, url, job.size, resume_from)?;

        // 下载数据
        let mut response = response;
        while let Some(chunk) = response.chunk().await? {
            // 检查本地状态和全局取消标志
            if !state.load(Ordering::Acquire) || global_cancel.load(Ordering::Acquire) {
                return Err(LauncherError::Custom("Download cancelled".to_string()));
            }
            file.write_all(&chunk).await?;
            let len = chunk.len() as u64;
            bytes_downloaded.fetch_add(len, Ordering::Relaxed);
            bytes_since_last.fetch_add(len, Ordering::Relaxed);
            bytes_added_this_attempt += len;
        }

        // 确保数据写入磁盘
        file.flush().await?;
        drop(file);

        // 验证文件
        if !file_utils::verify_file(&tmp_path, job.hash.as_deref(), job.size)? {
            // 删除损坏的临时文件
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(LauncherError::Custom(format!(
                "File verification failed for {}: size or hash mismatch (corrupted file deleted).",
                tmp_path.display()
            )));
        }

        // 移动文件到最终位置
        finalize_download(&tmp_path, &job.path).await?;

        Ok::<(), LauncherError>(())
    }
    .await;

    // 如果失败，回滚已计数的字节
    if result.is_err() {
        bytes_downloaded.fetch_sub(bytes_added_this_attempt, Ordering::Relaxed);
    }

    result
}

/// 验证 Content-Type
fn validate_content_type(response: &reqwest::Response, url: &str) -> Result<(), LauncherError> {
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
    Ok(())
}

/// 验证 Content-Length
fn validate_content_length(
    response: &reqwest::Response,
    url: &str,
    expected_size: u64,
    resume_from: Option<u64>,
) -> Result<(), LauncherError> {
    if let Some(len_hdr) = response.headers().get(reqwest::header::CONTENT_LENGTH) {
        if let Ok(len_str) = len_hdr.to_str() {
            if let Ok(remote_len) = len_str.parse::<u64>() {
                // 对于续传，Content-Length 是剩余部分的大小
                let expected_len = if let Some(offset) = resume_from {
                    expected_size.saturating_sub(offset)
                } else {
                    expected_size
                };
                
                if remote_len == 0 && expected_len > 0 {
                    return Err(LauncherError::Custom(format!(
                        "Unexpected Content-Length 0 for {}, expected {}",
                        url, expected_len
                    )));
                }
            }
        }
    }
    Ok(())
}

/// 完成下载，移动文件到最终位置
async fn finalize_download(
    tmp_path: &std::path::Path,
    final_path: &std::path::Path,
) -> Result<(), LauncherError> {
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    if tokio::fs::metadata(final_path).await.is_ok() {
        let _ = tokio::fs::remove_file(final_path).await;
    }
    tokio::fs::rename(tmp_path, final_path).await?;
    Ok(())
}
