//! 版本清单获取逻辑

use super::http::get_manifest_client;
use crate::errors::LauncherError;
use crate::models::VersionManifest;
use crate::services::config::load_config;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// 获取 Minecraft 版本列表
pub async fn get_versions() -> Result<VersionManifest, LauncherError> {
    let config = load_config()?;
    let log_dir = PathBuf::from(config.game_dir).join("logs");
    fs::create_dir_all(&log_dir)?;

    let client = get_manifest_client()?;

    let urls = [
        "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json",
        "https://launchermeta.mojang.com/mc/game/version_manifest.json",
    ];

    let log_file = log_dir.join("version_fetch.log");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| {
            LauncherError::Custom(format!("无法创建日志文件 {}: {}", log_file.display(), e))
        })?;

    writeln!(
        log,
        "[{}] 开始获取版本列表",
        chrono::Local::now().to_rfc3339()
    )?;

    for (i, url) in urls.iter().enumerate() {
        writeln!(log, "尝试第{}个源: {}", i + 1, url)?;
        match fetch_versions(&client, url, &mut log).await {
            Ok(manifest) => {
                writeln!(log, "成功获取版本列表，共{}个版本", manifest.versions.len())?;
                return Ok(manifest);
            }
            Err(e) => {
                writeln!(log, "获取失败: {}", e)?;
                continue;
            }
        }
    }

    Err(LauncherError::Custom(
        "所有源都尝试失败，请检查网络连接".to_string(),
    ))
}

/// 从指定 URL 获取版本清单
async fn fetch_versions(
    client: &reqwest::Client,
    url: &str,
    log: &mut fs::File,
) -> Result<VersionManifest, LauncherError> {
    writeln!(log, "[DEBUG] 准备发送请求到: {}", url)?;

    let response = client.get(url).send().await?;
    writeln!(log, "[DEBUG] 响应状态码: {}", response.status())?;

    let text = response.text().await?;
    let text = text.trim_start_matches('\u{feff}').to_string();

    let manifest = serde_json::from_str::<VersionManifest>(&text).map_err(|e| {
        writeln!(log, "JSON parse error: {}", e).ok();
        LauncherError::Json(e)
    })?;

    writeln!(
        log,
        "Parsed manifest with {} versions",
        manifest.versions.len()
    )?;

    Ok(manifest)
}
