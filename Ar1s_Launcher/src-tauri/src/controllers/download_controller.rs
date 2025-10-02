use std::fs;
use std::io::Write;
use std::path::PathBuf;


use crate::errors::LauncherError;
use crate::models::*;
use crate::services::config::load_config;


/// 初始化日志系统（控制器私有）
fn init_logging() -> Result<PathBuf, LauncherError> {
    let config = load_config()?;
    let minecraft_dir = PathBuf::from(&config.game_dir);
    let log_dir = minecraft_dir.join("logs");
    fs::create_dir_all(&log_dir)
        .map_err(|e| LauncherError::Custom(format!("无法创建日志目录: {}", e)))?;
    Ok(log_dir)
}

async fn fetch_versions(client: &reqwest::Client, url: &str) -> Result<VersionManifest, LauncherError> {
    let logs_dir = std::env::current_dir()?.join("logs");
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }
    let log_file = logs_dir.join("network_debug.log");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| LauncherError::Custom(format!("无法创建日志文件 {}: {}", log_file.display(), e)))?;

    writeln!(log, "[DEBUG] 准备发送请求到: {}", url)?;

    let request = client.get(url);
    let default_headers = reqwest::header::HeaderMap::new();
    writeln!(log, "[DEBUG] 请求头: {:?}", default_headers)?;

    let response = request.send().await.map_err(|e| {
        let _ = writeln!(log, "[ERROR] 请求失败: {}", e);
        e
    })?;

    writeln!(log, "[DEBUG] 响应状态码: {}", response.status())?;
    writeln!(log, "[DEBUG] 响应头: {:?}", response.headers())?;

    let content_type = response.headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    writeln!(log, "[DEBUG] Content-Type: {}", content_type)?;

    let bytes = response.bytes().await.map_err(|e| {
        let _ = writeln!(log, "[ERROR] 读取响应体失败: {}", e);
        e
    })?;

    let text = String::from_utf8_lossy(&bytes).into_owned();
    let text = text.trim_start_matches('\u{feff}').to_string();

    log::debug!("Received response (first 100 chars): {:?}", 
        text.chars().take(100).collect::<String>());

    let log_dir = PathBuf::from("logs");
    if !log_dir.exists() {
        fs::create_dir(&log_dir)?;
    }
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("version_fetch.log"))?;
    writeln!(log, "Raw response from {}:\n{}", url, text)?;

    let manifest = serde_json::from_str::<VersionManifest>(&text)
        .map_err(|e| {
            writeln!(log, "JSON parse error: {}", e).ok();
            LauncherError::Json(e)
        })?;

    writeln!(log, "Parsed manifest with {} versions", manifest.versions.len())?;
    Ok(manifest)
}

/// 获取 Minecraft 版本列表
#[tauri::command]
pub async fn get_versions() -> Result<VersionManifest, LauncherError> {
    let _ = init_logging()?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| LauncherError::Custom(format!("创建HTTP客户端失败: {}", e)))?;

    let urls = [
        "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json",
        "https://launchermeta.mojang.com/mc/game/version_manifest.json"
    ];

    let log_file = {
        let log_dir = init_logging()?;
        log_dir.join("version_fetch.log")
    };
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| LauncherError::Custom(format!("无法创建日志文件 {}: {}", log_file.display(), e)))?;

    writeln!(log, "[{}] 开始获取版本列表", chrono::Local::now())?;

    for (i, url) in urls.iter().enumerate() {
        writeln!(log, "尝试第{}个源: {}", i + 1, url)?;
        match fetch_versions(&client, url).await {
            Ok(manifest) => {
                writeln!(log, "成功获取版本列表，共{}个版本", manifest.versions.len())?;
                return Ok(manifest);
            },
            Err(e) => {
                writeln!(log, "获取失败: {}", e)?;
                continue;
            }
        }
    }
    Err(LauncherError::Custom("所有源都尝试失败，请检查网络连接".to_string()))
}

use crate::services::download;

/// 下载 Minecraft 版本
#[tauri::command]
pub async fn download_version(
    version_id: String,
    mirror: Option<String>,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    download::process_and_download_version(version_id, mirror, &window).await
}