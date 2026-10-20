//! 单个文件下载
//!
//! 写 .part 临时文件按 Range 续传，边写边算 sha1（写完后不再整盘重读一遍），
//! 校验失败时切换到 fallback_url

use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::utils::file_utils::{self, FileHasher};
use crate::utils::verify_cache;
use log::{debug, warn};
use std::path::{Path, PathBuf};
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
        match verify_existing(job).await {
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
        Ok(()) => Ok(()),
        Err(e) => {
            if e.is_cancelled() {
                return Err(e);
            }
            if let Some(fallback_url) = &job.fallback_url
                && should_try_fallback(&e)
            {
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
            Err(e)
        }
    }
}

/// 校验会有整文件读盘，扔到阻塞线程池，别占着 async worker
async fn verify_existing(job: &DownloadJob) -> Result<bool, LauncherError> {
    let job = job.clone();
    tokio::task::spawn_blocking(move || {
        file_utils::verify_file(&job.path, job.hash.as_deref(), job.size)
    })
    .await
    .map_err(|e| LauncherError::Custom(format!("校验文件失败: {}", e)))?
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
    let tmp_path = part_path(&job.path);
    adopt_legacy_part(&job.path, &tmp_path);

    let existing_size = tokio::fs::metadata(&tmp_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    if existing_size > 0 && job.size > 0 && existing_size >= job.size {
        debug!(
            "Part file complete ({}), verifying: {}",
            existing_size,
            tmp_path.display()
        );
        let tmp_for_verify = tmp_path.clone();
        let hash = job.hash.clone();
        let size = job.size;
        let valid = tokio::task::spawn_blocking(move || {
            file_utils::verify_file(&tmp_for_verify, hash.as_deref(), size)
        })
        .await
        .map_err(|e| LauncherError::Custom(format!("校验临时文件失败: {}", e)))??;

        if valid {
            finalize_download(&tmp_path, &job.path).await?;
            remember_fingerprint(&job.path, job.hash.as_deref());
            bytes_downloaded.fetch_add(job.size, Ordering::Relaxed);
            return Ok(());
        }
        warn!("Part file corrupted, restarting download");
        let _ = tokio::fs::remove_file(&tmp_path).await;
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

/// 保持 `<文件名>.part` 的形式：`with_extension` 会把 `a.jar` 变成 `a.part`，
/// 同目录下两个 jar 会撞名
fn part_path(final_path: &Path) -> PathBuf {
    let mut name = final_path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(".part");
    final_path.with_file_name(name)
}

/// 老版本用 `with_extension("part")`，升级前中断的下载要把旧临时文件认领过来，
/// 否则一个下到一半的大 jar 会从头再下一遍
fn adopt_legacy_part(final_path: &Path, part: &Path) {
    if part.exists() {
        return;
    }
    let legacy = final_path.with_extension("part");
    if legacy != part && legacy.exists() {
        debug!(
            "沿用旧版临时文件: {} -> {}",
            legacy.display(),
            part.display()
        );
        let _ = std::fs::rename(&legacy, part);
    }
}

#[allow(clippy::too_many_arguments)]
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
    let tmp_path = part_path(&job.path);
    let mut bytes_added_this_attempt: u64 = 0;
    let start_offset = resume_from.unwrap_or(0);

    let result = async {
        if let Some(parent) = job.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let (mut file, mut hasher, resume) = open_sink(&tmp_path, resume_from).await?;
        let mut resume = resume;

        let mut request = client.get(url);
        if let Some(offset) = resume {
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

        if resume.is_some() && status == reqwest::StatusCode::OK {
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
            hasher = FileHasher::new();
            resume = None;
        }

        if resume.is_none() {
            validate_content_type(&response, url)?;
        }

        validate_content_length(&response, url, job.size, resume)?;

        let mut response = response;
        while let Some(chunk) = response.chunk().await? {
            if !state.load(Ordering::Acquire) || global_cancel.load(Ordering::Acquire) {
                return Err(LauncherError::Cancelled);
            }
            file.write_all(&chunk).await?;
            // 顺手进哈希：省掉写完之后再把整个文件读一遍
            hasher.update(&chunk);
            let len = chunk.len() as u64;
            bytes_downloaded.fetch_add(len, Ordering::Relaxed);
            bytes_since_last.fetch_add(len, Ordering::Relaxed);
            bytes_added_this_attempt += len;
        }

        file.flush().await?;
        drop(file);

        verify_downloaded(&tmp_path, job, hasher).await?;

        finalize_download(&tmp_path, &job.path).await?;
        remember_fingerprint(&job.path, job.hash.as_deref());

        Ok::<(), LauncherError>(())
    }
    .await;

    if result.is_err() {
        bytes_downloaded.fetch_sub(bytes_added_this_attempt, Ordering::Relaxed);
    }

    result
}

/// 校验刚下完的数据：大小 + 哈希，失败删掉 .part 让下次从头来
async fn verify_downloaded(
    tmp_path: &Path,
    job: &DownloadJob,
    hasher: FileHasher,
) -> Result<(), LauncherError> {
    let actual_size = tokio::fs::metadata(tmp_path).await?.len();

    let mismatch = if job.size > 0 && actual_size != job.size {
        Some(format!("expected {} bytes, got {}", job.size, actual_size))
    } else {
        match job.hash.as_deref() {
            Some(expected) => {
                let actual = hasher.finish_hex();
                (!actual.eq_ignore_ascii_case(expected.trim()))
                    .then(|| format!("expected sha1 {}, got {}", expected, actual))
            }
            None => None,
        }
    };

    if let Some(reason) = mismatch {
        let _ = tokio::fs::remove_file(tmp_path).await;
        warn!("下载校验失败: {} ({})", tmp_path.display(), reason);
        return Err(LauncherError::Custom(format!(
            "File verification failed for {}: {} (corrupted file deleted).",
            tmp_path.display(),
            reason
        )));
    }

    Ok(())
}

/// 记下「这份文件已校验通过」，下次安装命中指纹就跳过读盘
fn remember_fingerprint(path: &Path, expected_hash: Option<&str>) {
    if let Ok(meta) = std::fs::metadata(path) {
        verify_cache::remember(path, &meta, expected_hash);
    }
}

/// 续传时把 `.part` 里已有的前缀补算进哈希；首次下载直接给个空哈希器
async fn open_sink(
    tmp_path: &Path,
    resume_from: Option<u64>,
) -> Result<(tokio::fs::File, FileHasher, Option<u64>), LauncherError> {
    match resume_from {
        Some(offset) => {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .open(tmp_path)
                .await?;
            file.seek(std::io::SeekFrom::End(0)).await?;
            let hasher = hash_prefix(tmp_path, offset).await?;
            Ok((file, hasher, Some(offset)))
        }
        None => {
            let file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(tmp_path)
                .await?;
            Ok((file, FileHasher::new(), None))
        }
    }
}

/// 把 `.part` 前 `bytes` 个字节重新喂进哈希
///
/// 只有真正中断过的文件才会走这里，读盘量等于上次已下载的量；不补算的话
/// 续传完成后的 sha1 必然对不上，等于白下。
async fn hash_prefix(path: &Path, bytes: u64) -> Result<FileHasher, LauncherError> {
    use std::io::Read;

    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<FileHasher, LauncherError> {
        let mut file = std::fs::File::open(&path)?;
        let mut hasher = FileHasher::new();
        let mut remaining = bytes;
        let mut buf = vec![0u8; 1024 * 1024];
        while remaining > 0 {
            let want = remaining.min(buf.len() as u64) as usize;
            let read = file.read(&mut buf[..want])?;
            if read == 0 {
                break;
            }
            hasher.update(&buf[..read]);
            remaining -= read as u64;
        }
        Ok(hasher)
    })
    .await
    .map_err(|e| LauncherError::Custom(format!("计算续传前缀哈希失败: {}", e)))?
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
    if let Some(len_hdr) = response.headers().get(reqwest::header::CONTENT_LENGTH)
        && let Ok(len_str) = len_hdr.to_str()
        && let Ok(remote_len) = len_str.parse::<u64>()
    {
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
    Ok(())
}

async fn finalize_download(tmp_path: &Path, final_path: &Path) -> Result<(), LauncherError> {
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    if tokio::fs::metadata(final_path).await.is_ok() {
        let _ = tokio::fs::remove_file(final_path).await;
    }
    tokio::fs::rename(tmp_path, final_path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::part_path;
    use crate::utils::file_utils::FileHasher;
    use std::path::Path;

    #[test]
    fn test_part_path_appends_suffix_instead_of_replacing_extension() {
        assert_eq!(
            part_path(Path::new("/x/assets/ab/abcdef")),
            Path::new("/x/assets/ab/abcdef.part")
        );
        // 同一目录下的两个 jar 不能算出同一个临时文件
        assert_ne!(
            part_path(Path::new("/x/a.jar")),
            part_path(Path::new("/x/b.jar"))
        );
        assert_eq!(part_path(Path::new("/x/a.jar")), Path::new("/x/a.jar.part"));
    }

    #[test]
    fn test_streamed_hash_matches_single_shot_hash() {
        let data: Vec<u8> = (0..70_000u32).map(|i| (i % 251) as u8).collect();

        let mut hasher = FileHasher::new();
        for chunk in data.chunks(1024) {
            hasher.update(chunk);
        }

        assert_eq!(
            hasher.finish_hex(),
            crate::utils::file_utils::sha1_hex(&data)
        );
    }
}
