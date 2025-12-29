use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::services::config::load_config;
use crate::utils::file_utils;
use log::{debug, info};
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
    
    info!("版本 JSON 内容: id={:?}, inheritsFrom={:?}, jar={:?}",
        version_json["id"].as_str(),
        version_json["inheritsFrom"].as_str(),
        version_json["jar"].as_str()
    );

    let libraries_base_dir = game_dir.join("libraries");

    // 递归查找最终的 JAR 版本（处理多层继承）
    let jar_version = find_jar_version(&version_json, &game_dir)?;
    debug!("JAR 版本: {}", jar_version);

    // 主游戏 JAR 文件路径
    let main_game_jar_path = game_dir
        .join("versions")
        .join(&jar_version)
        .join(format!("{}.jar", &jar_version));
    
    if !main_game_jar_path.exists() {
        info!("主游戏JAR文件不存在: {}", main_game_jar_path.display());
        missing_files.push(format!(
            "主游戏JAR文件不存在: {}",
            main_game_jar_path.display()
        ));
    }

    // 递归验证整个继承链的版本 JSON 文件，并检查所有库
    let mut versions_to_check = vec![version_json.clone()];
    let mut current_json = version_json.clone();
    
    while let Some(inherits_from) = current_json["inheritsFrom"].as_str() {
        debug!("检查继承版本: {}", inherits_from);
        let base_version_json_path = game_dir
            .join("versions")
            .join(inherits_from)
            .join(format!("{}.json", inherits_from));
        
        if !base_version_json_path.exists() {
            info!("基础版本JSON文件不存在: {}", base_version_json_path.display());
            missing_files.push(format!(
                "基础版本JSON文件不存在: {}",
                base_version_json_path.display()
            ));
            break;
        }
        
        // 读取父版本 JSON 继续检查
        let parent_str = fs::read_to_string(&base_version_json_path)?;
        let parent_json: serde_json::Value = serde_json::from_str(&parent_str)?;
        versions_to_check.push(parent_json.clone());
        current_json = parent_json;
    }

    // 检查所有版本（包括继承链）中声明的库
    for ver_json in &versions_to_check {
        let ver_id = ver_json["id"].as_str().unwrap_or("unknown");
        if let Some(libraries) = ver_json["libraries"].as_array() {
            debug!("检查版本 {} 的 {} 个库", ver_id, libraries.len());
            for lib in libraries {
                check_library(lib, &libraries_base_dir, &mut missing_files);
            }
        } else {
            debug!("版本 {} 没有 libraries 数组", ver_id);
        }
    }

    info!("验证完成，发现 {} 个缺失文件", missing_files.len());
    Ok(missing_files)
}

/// 检查单个库文件是否存在
fn check_library(lib: &serde_json::Value, libraries_base_dir: &PathBuf, missing_files: &mut Vec<String>) {
    let lib_name = lib.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
    
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
        // 检查 rules
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
                return;
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
                debug!("库文件缺失: {} -> {}", lib_name, lib_path.display());
                missing_files.push(format!("库文件不存在: {}", lib_path.display()));
            }
        } else {
            // 没有 downloads.artifact.path，尝试从 name 构建路径
            if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                if let Some(path) = maven_name_to_path(name) {
                    let lib_path = libraries_base_dir.join(&path);
                    if !lib_path.exists() {
                        debug!("库文件缺失 (从name构建): {} -> {}", name, lib_path.display());
                        missing_files.push(format!("库文件不存在: {}", lib_path.display()));
                    }
                }
            }
        }
    }
}

/// 将 Maven 坐标转换为文件路径
fn maven_name_to_path(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = if parts.len() > 3 { Some(parts[3]) } else { None };
    
    let filename = if let Some(c) = classifier {
        format!("{}-{}-{}.jar", artifact, version, c)
    } else {
        format!("{}-{}.jar", artifact, version)
    };
    
    Some(format!("{}/{}/{}/{}", group, artifact, version, filename))
}

/// 递归查找最终的 JAR 版本（处理多层继承链）
fn find_jar_version(version_json: &serde_json::Value, game_dir: &PathBuf) -> Result<String, LauncherError> {
    let current_id = version_json["id"].as_str().unwrap_or("unknown");
    debug!("查找 JAR 版本, 当前 JSON id: {}, jar: {:?}, inheritsFrom: {:?}",
        current_id,
        version_json["jar"].as_str(),
        version_json["inheritsFrom"].as_str()
    );
    
    // 优先使用 jar 字段
    if let Some(jar) = version_json["jar"].as_str() {
        debug!("使用 jar 字段: {}", jar);
        return Ok(jar.to_string());
    }
    
    // 如果有 inheritsFrom，递归查找
    if let Some(inherits_from) = version_json["inheritsFrom"].as_str() {
        debug!("递归查找 inheritsFrom: {}", inherits_from);
        let parent_json_path = game_dir
            .join("versions")
            .join(inherits_from)
            .join(format!("{}.json", inherits_from));
        
        if parent_json_path.exists() {
            let parent_str = fs::read_to_string(&parent_json_path)?;
            let parent_json: serde_json::Value = serde_json::from_str(&parent_str)?;
            return find_jar_version(&parent_json, game_dir);
        } else {
            info!("父版本 JSON 不存在: {} (从 {} 继承)", parent_json_path.display(), current_id);
            // 如果父版本 JSON 不存在，假设 inheritsFrom 就是最终版本（原版 MC）
            return Ok(inherits_from.to_string());
        }
    }
    
    // 没有 jar 也没有 inheritsFrom，使用版本 ID（这是原版 MC）
    if let Some(id) = version_json["id"].as_str() {
        debug!("使用版本 ID 作为 JAR 版本: {}", id);
        return Ok(id.to_string());
    }
    
    Err(LauncherError::Custom("无法确定 JAR 版本".to_string()))
}
