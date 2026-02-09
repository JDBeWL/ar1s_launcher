//! 版本清单获取逻辑

use super::http::get_manifest_client;
use crate::errors::LauncherError;
use crate::models::VersionManifest;
use log::{debug, info, warn};

/// 获取 Minecraft 版本列表
pub async fn get_versions() -> Result<VersionManifest, LauncherError> {
    let client = get_manifest_client()?;

    let urls = [
        "https://bmclapi2.bangbang93.com/mc/game/version_manifest.json",
        "https://launchermeta.mojang.com/mc/game/version_manifest.json",
    ];

    info!("开始获取版本列表");

    for (i, url) in urls.iter().enumerate() {
        debug!("尝试第{}个源: {}", i + 1, url);
        match fetch_versions(&client, url).await {
            Ok(manifest) => {
                info!("成功获取版本列表，共{}个版本", manifest.versions.len());
                return Ok(manifest);
            }
            Err(e) => {
                warn!("从源 {} 获取失败: {}", url, e);
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
) -> Result<VersionManifest, LauncherError> {
    debug!("准备发送请求到: {}", url);

    let response = client.get(url).send().await?;
    debug!("响应状态码: {}", response.status());

    let text = response.text().await?;
    let text = text.trim_start_matches('\u{feff}').to_string();

    let manifest = serde_json::from_str::<VersionManifest>(&text).map_err(|e| {
        warn!("JSON 解析错误: {}", e);
        LauncherError::Json(e)
    })?;

    debug!("解析版本清单完成，共 {} 个版本", manifest.versions.len());

    Ok(manifest)
}
