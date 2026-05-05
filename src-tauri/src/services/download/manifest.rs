//! 版本清单获取逻辑

use super::http::get_manifest_client;
use crate::errors::LauncherError;
use crate::models::VersionManifest;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSizeInfo {
    pub version_id: String,
    pub client_size: u64,
    pub total_size: u64,
}

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

/// 获取指定版本的文件大小信息
pub async fn get_version_size(version_id: String, mirror: Option<String>) -> Result<VersionSizeInfo, LauncherError> {
    let client = get_manifest_client()?;

    let is_mirror = mirror.is_some();
    let base_url = if is_mirror {
        "https://bmclapi2.bangbang93.com"
    } else {
        "https://launchermeta.mojang.com"
    };

    let manifest: VersionManifest = client
        .get(&format!("{}/mc/game/version_manifest.json", base_url))
        .send()
        .await?
        .json()
        .await?;

    let version = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| LauncherError::Custom(format!("版本 {} 不存在", version_id)))?;

    let version_json_url = if is_mirror {
        version
            .url
            .replace("https://launchermeta.mojang.com", base_url)
            .replace("https://piston-meta.mojang.com", base_url)
    } else {
        version.url.clone()
    };

    let text = client.get(&version_json_url).send().await?.text().await?;
    let version_json: serde_json::Value = serde_json::from_str(&text)
        .or_else(|_| serde_json::from_str(text.trim_start_matches('\u{feff}')))
        .map_err(|_| LauncherError::Custom(format!("无法解析版本JSON for {}", version_id)))?;

    let client_size = version_json["downloads"]["client"]["size"]
        .as_u64()
        .unwrap_or(0);

    let mut total_size: u64 = client_size;

    if let Some(libraries) = version_json["libraries"].as_array() {
        for lib in libraries {
            if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                total_size += artifact["size"].as_u64().unwrap_or(0);
            }
        }
    }

    if let Some(asset_index_size) = version_json["assetIndex"]["size"].as_u64() {
        total_size += asset_index_size;
    }

    Ok(VersionSizeInfo {
        version_id,
        client_size,
        total_size,
    })
}
