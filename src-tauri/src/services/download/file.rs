//! 单个文件下载
//!
//! 写 .part 临时文件按 Range 续传，校验大小与 sha1，失败时切 fallback_url

use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::utils::file_utils;
use log::{debug, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

pub async fn download_file(
    http: &reqwest::Client,
    job: &DownloadJob,
    url: &str,
    state: &Arc<AtomicBool>,
    global_cancel: &Arc<AtomicBool>,
    bytes_downloaded: &Arc<AtomicU64>,
    bytes_since_last: &Arc<AtomicU64>,
) -> Result<(), LauncherError> {
    if !state.load(Ordering::Acquire) || global_cancel.load(Ordering::Acquire) {
        return Err(LauncherError::Cancelled);
    }

    if job.path.exists() {
        match file_utils::verify_and_repair_file(job, http).await {
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

    match download_with_resume(
        http,
        url,
        job,
        state,
        global_cancel,
        bytes_downloaded,
        bytes_since_last,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            if e.is_cancelled() {
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
    client: &reqwest::Client,
    url: &str,
    job: &DownloadJob,
    state: &Arc<AtomicBool>,
    global_cancel: &Arc<AtomicBool>,
    bytes_downloaded: &Arc<AtomicU64>,
    bytes_since_last: &Arc<AtomicU64>,
) -> Result<(), LauncherError> {
    let tmp_path = job.path.with_extension("part");

    let existing_size = get_existing_file_size(&tmp_path).await;

    if existing_size > 0 && job.size > 0 && existing_size >= job.size {
        debug!(
            "Part file complete ({}), verifying: {}",
            existing_size,
            tmp_path.display()
        );
        if file_utils::verify_file(&tmp_path, job.hash.as_deref(), job.size)? {
            finalize_download(&tmp_path, &job.path).await?;
            bytes_downloaded.fetch_add(job.size, Ordering::Relaxed);
            return Ok(());
        } else {
            warn!("Part file corrupted, restarting download");
            let _ = tokio::fs::remove_file(&tmp_path).await;
        }
    }

    // 不做 HEAD 预检，直接发 Range；返回 200（不支持 Range）由 download_chunk_with_resume 兜底
    let resume_from = if existing_size > 0 && job.size > 0 && existing_size < job.size {
        debug!("Resuming download from byte {}: {}", existing_size, url);
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

async fn get_existing_file_size(path: &std::path::Path) -> u64 {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.len())
        .unwrap_or(0)
}

async fn download_chunk_with_resume(
    client: &reqwest::Client,
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

        let mut file = if resume_from.is_some() {
            let mut f = tokio::fs::OpenOptions::new()
                .write(true)
                .open(&tmp_path)
                .await?;
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

        let mut request = client.get(url);
        if let Some(offset) = resume_from {
            request = request.header("Range", format!("bytes={}-", offset));
            bytes_downloaded.fetch_add(offset, Ordering::Relaxed);
            bytes_added_this_attempt += offset;
        }

        let response = request.send().await?;

        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(LauncherError::Custom(format!(
                "HTTP error {} for {}",
                status, url
            )));
        }

        if resume_from.is_some() && status == reqwest::StatusCode::OK {
            debug!("Server returned 200 instead of 206, restarting download");
            bytes_downloaded.fetch_sub(start_offset, Ordering::Relaxed);
            bytes_added_this_attempt -= start_offset;
            drop(file);
            file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp_path)
                .await?;
        }

        if resume_from.is_none() {
            validate_content_type(&response, url)?;
        }

        validate_content_length(&response, url, job.size, resume_from)?;

        let mut response = response;
        while let Some(chunk) = response.chunk().await? {
            if !state.load(Ordering::Acquire) || global_cancel.load(Ordering::Acquire) {
                return Err(LauncherError::Cancelled);
            }
            file.write_all(&chunk).await?;
            let len = chunk.len() as u64;
            bytes_downloaded.fetch_add(len, Ordering::Relaxed);
            bytes_since_last.fetch_add(len, Ordering::Relaxed);
            bytes_added_this_attempt += len;
        }

        file.flush().await?;
        drop(file);

        if !file_utils::verify_file(&tmp_path, job.hash.as_deref(), job.size)? {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(LauncherError::Custom(format!(
                "File verification failed for {}: size or hash mismatch (corrupted file deleted).",
                tmp_path.display()
            )));
        }

        finalize_download(&tmp_path, &job.path).await?;

        Ok::<(), LauncherError>(())
    }
    .await;

    if result.is_err() {
        bytes_downloaded.fetch_sub(bytes_added_this_attempt, Ordering::Relaxed);
    }

    result
}

fn validate_content_type(response: &reqwest::Response, url: &str) -> Result<(), LauncherError> {
    if let Some(ct) = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        let ct_lower = ct.to_ascii_lowercase();
        if ct_lower.starts_with("text/") || ct_lower.contains("json") || ct_lower.contains("html") {
            return Err(LauncherError::Custom(format!(
                "Unexpected Content-Type {} for {}",
                ct, url
            )));
        }
    }
    Ok(())
}

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
