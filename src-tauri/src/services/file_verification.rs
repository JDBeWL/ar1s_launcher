use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::services::config::load_config;
use crate::utils::file_utils;
use reqwest::Client;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

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
                println!("修复任务失败: {}", e);
            }
        }
    }

    Ok(results)
}

pub async fn validate_version_files(version_id: String) -> Result<Vec<String>, LauncherError> {
    let config = load_config()?;
    let game_dir = PathBuf::from(&config.game_dir);
    let version_dir = game_dir.join("versions").join(&version_id);
    let version_json_path = version_dir.join(format!("{}.json", &version_id));

    let mut missing_files = Vec::new();

    if !version_json_path.exists() {
        missing_files.push(format!(
            "版本JSON文件不存在: {}",
            version_json_path.display()
        ));
        return Ok(missing_files);
    }

    let version_json_str = fs::read_to_string(&version_json_path)?;
    let version_json: serde_json::Value = serde_json::from_str(&version_json_str)?;

    let libraries_base_dir = game_dir.join("libraries");

    let main_game_jar_path = version_dir.join(format!("{}.jar", &version_id));
    if !main_game_jar_path.exists() {
        missing_files.push(format!(
            "主游戏JAR文件不存在: {}",
            main_game_jar_path.display()
        ));
    }

    if let Some(libraries) = version_json["libraries"].as_array() {
        for lib in libraries {
            if let Some(natives) = lib.get("natives") {
                let current_os = std::env::consts::OS;
                let os_key = match current_os {
                    "windows" => "windows",
                    "linux" => "linux",
                    "macos" => "osx",
                    _ => "unknown",
                };

                if let Some(os_classifier) = natives.get(os_key) {
                    if let Some(classifier_str) = os_classifier.as_str() {
                        // 处理 ${arch} 占位符替换，与 download.rs 和 launcher.rs 保持一致
                        let arch = if std::env::consts::ARCH.contains("64") {
                            "64"
                        } else {
                            "32"
                        };
                        let classifier = classifier_str.replace("${arch}", arch);

                        if let Some(artifact) = lib
                            .get("downloads")
                            .and_then(|d| d.get("classifiers"))
                            .and_then(|c| c.get(&classifier))
                        {
                            let lib_path =
                                libraries_base_dir.join(artifact["path"].as_str().unwrap_or(""));
                            if !lib_path.exists() {
                                missing_files
                                    .push(format!("Natives库文件不存在: {}", lib_path.display()));
                            }
                        }
                    }
                }
            } else {
                if let Some(rules) = lib.get("rules").and_then(|r| r.as_array()) {
                    let mut allowed = true;
                    for rule in rules {
                        if let Some(os) = rule.get("os") {
                            if let Some(name) = os["name"].as_str() {
                                if name == std::env::consts::OS {
                                    allowed = rule["action"].as_str() == Some("allow");
                                } else {
                                    allowed = rule["action"].as_str() != Some("allow");
                                }
                            }
                        }
                    }
                    if !allowed {
                        continue;
                    }
                }
                if let Some(path) = lib
                    .get("downloads")
                    .and_then(|d| d.get("artifact"))
                    .and_then(|a| a.get("path"))
                    .and_then(|p| p.as_str())
                {
                    let lib_path = libraries_base_dir.join(path);
                    if !lib_path.exists() {
                        missing_files.push(format!("库文件不存在: {}", lib_path.display()));
                    }
                }
            }
        }
    }

    Ok(missing_files)
}
