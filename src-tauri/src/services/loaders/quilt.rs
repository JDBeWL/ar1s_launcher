//! Quilt 加载器安装

use crate::errors::LauncherError;
use log::info;
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Quilt Meta API 基础 URL
const QUILT_META_URL: &str = "https://meta.quiltmc.org/v3";

/// 安装 Quilt 加载器
pub async fn install_quilt(
    mc_version: &str,
    quilt_version: &str,
    instance_name: &str,
    game_dir: &Path,
) -> Result<(), LauncherError> {
    info!(
        "安装 Quilt: MC {} + Quilt {} -> {}",
        mc_version, quilt_version, instance_name
    );

    let client = Client::new();

    // 从 Quilt Meta API 获取版本 JSON
    let profile_url = format!(
        "{}/versions/loader/{}/{}/profile/json",
        QUILT_META_URL, mc_version, quilt_version
    );

    info!("获取 Quilt 版本信息: {}", profile_url);

    let response = client
        .get(&profile_url)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("获取 Quilt 信息失败: {}", e)))?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取 Quilt 信息失败: {}",
            response.status()
        )));
    }

    let mut version_json: Value = response
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析 Quilt JSON 失败: {}", e)))?;

    // 修改版本 ID 为实例名称
    if let Some(obj) = version_json.as_object_mut() {
        obj.insert("id".to_string(), serde_json::json!(instance_name));
    }

    // 保存版本 JSON
    let version_dir = game_dir.join("versions").join(instance_name);
    fs::create_dir_all(&version_dir)?;

    let json_path = version_dir.join(format!("{}.json", instance_name));
    fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

    info!("Quilt 版本 JSON 已创建: {}", json_path.display());

    Ok(())
}

/// 获取 Quilt 加载器版本列表
pub async fn get_quilt_versions(mc_version: &str) -> Result<Vec<QuiltLoaderVersion>, LauncherError> {
    let client = Client::new();
    let url = format!("{}/versions/loader/{}", QUILT_META_URL, mc_version);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("获取 Quilt 版本列表失败: {}", e)))?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取 Quilt 版本列表失败: {}",
            response.status()
        )));
    }

    let versions: Vec<QuiltLoaderInfo> = response
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析 Quilt 版本列表失败: {}", e)))?;

    Ok(versions
        .into_iter()
        .map(|v| QuiltLoaderVersion {
            version: v.loader.version,
        })
        .collect())
}

/// 获取支持 Quilt 的 Minecraft 版本列表
pub async fn get_quilt_game_versions() -> Result<Vec<String>, LauncherError> {
    let client = Client::new();
    let url = format!("{}/versions/game", QUILT_META_URL);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("获取游戏版本列表失败: {}", e)))?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取游戏版本列表失败: {}",
            response.status()
        )));
    }

    let versions: Vec<QuiltGameVersion> = response
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析游戏版本列表失败: {}", e)))?;

    Ok(versions.into_iter().map(|v| v.version).collect())
}

// --- 内部数据结构 ---

#[derive(serde::Deserialize)]
struct QuiltLoaderInfo {
    loader: QuiltLoader,
}

#[derive(serde::Deserialize)]
struct QuiltLoader {
    version: String,
}

#[derive(serde::Deserialize)]
struct QuiltGameVersion {
    version: String,
}

/// Quilt 加载器版本信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuiltLoaderVersion {
    pub version: String,
}
