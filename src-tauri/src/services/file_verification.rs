//! 文件校验与修复模块
//!
//! 按 sha1 与大小校验版本文件，损坏时备份 .bak 后重下，批量校验与修复限 64 并发

use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::services::config::load_config;
use crate::utils::file_utils;
use log::{debug, info, warn};
use reqwest::Client;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAX_VERIFY_CONCURRENCY: usize = 64;

#[derive(Debug, Serialize)]
pub struct FileVerificationResult {
    pub file_name: String,
    pub is_valid: bool,
    pub file_size: u64,
    pub expected_size: u64,
    pub hash_match: bool,
}

pub async fn verify_single_file(
    job: &DownloadJob,
    _client: &Client,
) -> Result<FileVerificationResult, LauncherError> {
    let file_name = job
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let file_size = if job.path.exists() {
        std::fs::metadata(&job.path)?.len()
    } else {
        0
    };

    let is_valid = file_utils::verify_file(&job.path, job.hash.as_deref(), job.size)?;

    Ok(FileVerificationResult {
        file_name,
        is_valid,
        file_size,
        expected_size: job.size,
        hash_match: is_valid,
    })
}

pub async fn batch_verify_files(
    jobs: &[DownloadJob],
    client: &Client,
) -> Result<Vec<FileVerificationResult>, LauncherError> {
    use tokio::sync::Semaphore;
    use tokio::task;

    let semaphore = Arc::new(Semaphore::new(MAX_VERIFY_CONCURRENCY));
    let mut tasks = vec![];

    for job in jobs {
        let job_clone = job.clone();
        let client_clone = client.clone();
        let permit = semaphore.clone();

        tasks.push(task::spawn(async move {
            let _permit = permit.acquire().await.unwrap();
            verify_single_file(&job_clone, &client_clone).await
        }));
    }

    let mut results = vec![];
    for task in tasks {
        match task.await {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(e)) => {
                warn!("文件验证失败: {}", e);
            }
            Err(e) => {
                warn!("任务执行失败: {}", e);
            }
        }
    }

    Ok(results)
}

pub async fn repair_corrupted_file(
    job: &DownloadJob,
    client: &Client,
) -> Result<bool, LauncherError> {
    file_utils::verify_and_repair_file(job, client).await
}

pub async fn batch_repair_files(
    jobs: &[DownloadJob],
    client: &Client,
) -> Result<Vec<(String, bool)>, LauncherError> {
    use tokio::sync::Semaphore;
    use tokio::task;

    let semaphore = Arc::new(Semaphore::new(MAX_VERIFY_CONCURRENCY));
    let mut tasks = vec![];

    for job in jobs {
        let job_clone = job.clone();
        let client_clone = client.clone();
        let permit = semaphore.clone();

        tasks.push(task::spawn(async move {
            let _permit = permit.acquire().await.unwrap();
            let file_name = job_clone
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            match repair_corrupted_file(&job_clone, &client_clone).await {
                Ok(success) => (file_name, success),
                Err(_) => (file_name, false),
            }
        }));
    }

    let mut results = vec![];
    for task in tasks {
        match task.await {
            Ok(result) => results.push(result),
            Err(e) => {
                warn!("修复任务失败: {}", e);
            }
        }
    }

    Ok(results)
}

/// 资源索引没落盘说明原版资源这条链还没下载过（核心安装只取 client jar）
fn missing_asset_index(game_dir: &Path, version_json: &serde_json::Value) -> Option<String> {
    let asset_index_id = version_json["assetIndex"]["id"].as_str()?;
    let path = game_dir
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", asset_index_id));

    (!path.exists()).then(|| format!("游戏资源文件未下载: {}", path.display()))
}

/// 校验版本文件的完整性：只做同步 stat，调用方负责放进阻塞线程池
pub fn validate_version_files_sync(version_id: String) -> Result<Vec<String>, LauncherError> {
    let config = load_config()?;
    let game_dir = PathBuf::from(&config.game_dir);
    let version_dir = game_dir.join("versions").join(&version_id);
    let version_json_path = version_dir.join(format!("{}.json", version_id));

    let mut missing_files = Vec::new();

    info!("验证版本文件: {}", version_id);
    info!("版本 JSON 路径: {}", version_json_path.display());

    if !version_json_path.exists() {
        missing_files.push(format!(
            "版本JSON文件不存在: {}",
            version_json_path.display()
        ));
        return Ok(missing_files);
    }

    let version_json_str = fs::read_to_string(&version_json_path)?;
    let version_json: serde_json::Value = serde_json::from_str(&version_json_str)?;

    info!(
        "版本 JSON 内容: id={:?}, inheritsFrom={:?}, jar={:?}",
        version_json["id"].as_str(),
        version_json["inheritsFrom"].as_str(),
        version_json["jar"].as_str()
    );

    let libraries_base_dir = game_dir.join("libraries");

    // 递归查找最终的 JAR 版本（处理多层继承）
    let jar_version = find_jar_version(&version_json, &game_dir)?;
    debug!("JAR 版本: {}", jar_version);

    let main_game_jar_path = game_dir
        .join("versions")
        .join(&jar_version)
        .join(format!("{}.jar", jar_version));

    if !main_game_jar_path.exists() {
        info!("主游戏JAR文件不存在: {}", main_game_jar_path.display());
        missing_files.push(format!(
            "主游戏JAR文件不存在: {}",
            main_game_jar_path.display()
        ));
    }

    let mut versions_to_check = vec![version_json.clone()];
    let mut current_json = version_json.clone();

    while let Some(inherits_from) = current_json["inheritsFrom"].as_str() {
        debug!("检查继承版本: {}", inherits_from);
        let base_version_json_path = game_dir
            .join("versions")
            .join(inherits_from)
            .join(format!("{}.json", inherits_from));

        if !base_version_json_path.exists() {
            info!(
                "基础版本JSON文件不存在: {}",
                base_version_json_path.display()
            );
            missing_files.push(format!(
                "基础版本JSON文件不存在: {}",
                base_version_json_path.display()
            ));
            break;
        }

        let parent_str = fs::read_to_string(&base_version_json_path)?;
        let parent_json: serde_json::Value = serde_json::from_str(&parent_str)?;
        versions_to_check.push(parent_json.clone());
        current_json = parent_json;
    }

    // 继承链上所有版本声明的库和资源索引都要检查
    for ver_json in &versions_to_check {
        let ver_id = ver_json["id"].as_str().unwrap_or("unknown");

        if let Some(missing) = missing_asset_index(&game_dir, ver_json) {
            missing_files.push(missing);
        }

        if let Some(libraries) = ver_json["libraries"].as_array() {
            debug!("检查版本 {} 的 {} 个库", ver_id, libraries.len());
            for file in resolve_library_files(ver_json) {
                let lib_path = libraries_base_dir.join(&file.relative_path);
                if !lib_path.exists() {
                    debug!(
                        "库文件缺失: {} -> {}",
                        file.relative_path,
                        lib_path.display()
                    );
                    missing_files.push(format!("库文件不存在: {}", lib_path.display()));
                }
            }
        } else {
            debug!("版本 {} 没有 libraries 数组", ver_id);
        }
    }

    info!("验证完成，发现 {} 个缺失文件", missing_files.len());
    Ok(missing_files)
}

use crate::utils::library::resolve_library_files;
use crate::utils::minecraft::find_jar_version;

#[cfg(test)]
mod tests {
    use super::missing_asset_index;
    use serde_json::json;
    use std::fs;

    #[test]
    fn test_missing_asset_index_is_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        let version = json!({ "assetIndex": { "id": "17" } });

        assert!(missing_asset_index(dir.path(), &version).is_some());

        let indexes = dir.path().join("assets").join("indexes");
        fs::create_dir_all(&indexes).unwrap();
        fs::write(indexes.join("17.json"), "{}").unwrap();

        assert!(missing_asset_index(dir.path(), &version).is_none());
    }

    #[test]
    fn test_version_without_asset_index_is_ignored() {
        let dir = tempfile::TempDir::new().unwrap();

        assert!(missing_asset_index(dir.path(), &json!({})).is_none());
    }
}
