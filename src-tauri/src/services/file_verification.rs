use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::utils::file_utils;
use reqwest::Client;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FileVerificationResult {
    pub file_name: String,
    pub is_valid: bool,
    pub file_size: u64,
    pub expected_size: u64,
    pub hash_match: bool,
}

/// 验证单个文件的完整性
pub async fn verify_single_file(
    job: &DownloadJob,
    _client: &Client,
) -> Result<FileVerificationResult, LauncherError> {
    let file_name = job.path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    let file_size = if job.path.exists() {
        std::fs::metadata(&job.path)?.len()
    } else {
        0
    };
    
    let is_valid = file_utils::verify_file(&job.path, &job.hash, job.size)?;
    
    Ok(FileVerificationResult {
        file_name,
        is_valid,
        file_size,
        expected_size: job.size,
        hash_match: is_valid,
    })
}

/// 批量验证文件完整性
pub async fn batch_verify_files(
    jobs: &[DownloadJob],
    client: &Client,
) -> Result<Vec<FileVerificationResult>, LauncherError> {
    use tokio::task;
    
    let mut tasks = vec![];
    
    for job in jobs {
        let job_clone = job.clone();
        let client_clone = client.clone();
        
        tasks.push(task::spawn(async move {
            verify_single_file(&job_clone, &client_clone).await
        }));
    }
    
    let mut results = vec![];
    for task in tasks {
        match task.await {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(e)) => {
                println!("文件验证失败: {}", e);
            }
            Err(e) => {
                println!("任务执行失败: {}", e);
            }
        }
    }
    
    Ok(results)
}

/// 修复损坏的文件
pub async fn repair_corrupted_file(
    job: &DownloadJob,
    client: &Client,
) -> Result<bool, LauncherError> {
    file_utils::verify_and_repair_file(job, client).await
}

/// 批量修复损坏的文件
pub async fn batch_repair_files(
    jobs: &[DownloadJob],
    client: &Client,
) -> Result<Vec<(String, bool)>, LauncherError> {
    use tokio::task;
    
    let mut tasks = vec![];
    
    for job in jobs {
        let job_clone = job.clone();
        let client_clone = client.clone();
        
        tasks.push(task::spawn(async move {
            let file_name = job_clone.path.file_name()
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
                println!("修复任务失败: {}", e);
            }
        }
    }
    
    Ok(results)
}