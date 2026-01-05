//! Fabric 加载器安装

use crate::errors::LauncherError;
use log::info;
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Fabric Meta API 基础 URL
const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2";

/// 安装 Fabric 加载器
pub async fn install_fabric(
    mc_version: &str,
    fabric_version: &str,
    instance_name: &str,
    game_dir: &Path,
) -> Result<(), LauncherError> {
    info!(
        "安装 Fabric: MC {} + Fabric {} -> {}",
        mc_version, fabric_version, instance_name
    );

    let client = Client::new();

    // 从 Fabric Meta API 获取版本 JSON
    let profile_url = format!(
        "{}/versions/loader/{}/{}/profile/json",
        FABRIC_META_URL, mc_version, fabric_version
    );

    info!("获取 Fabric 版本信息: {}", profile_url);

    let response = client
        .get(&profile_url)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("获取 Fabric 信息失败: {}", e)))?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取 Fabric 信息失败: {}",
            response.status()
        )));
    }

    let mut version_json: Value = response
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析 Fabric JSON 失败: {}", e)))?;

    // 修改版本 ID 为实例名称
    if let Some(obj) = version_json.as_object_mut() {
        obj.insert("id".to_string(), serde_json::json!(instance_name));
    }

    // 保存版本 JSON
    let version_dir = game_dir.join("versions").join(instance_name);
    fs::create_dir_all(&version_dir)?;

    let json_path = version_dir.join(format!("{}.json", instance_name));
    fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

    info!("Fabric 版本 JSON 已创建: {}", json_path.display());

    Ok(())
}

/// 获取 Fabric 加载器版本列表
pub async fn get_fabric_versions(mc_version: &str) -> Result<Vec<FabricLoaderVersion>, LauncherError> {
    let client = Client::new();
    let url = format!("{}/versions/loader/{}", FABRIC_META_URL, mc_version);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("获取 Fabric 版本列表失败: {}", e)))?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取 Fabric 版本列表失败: {}",
            response.status()
        )));
    }

    let versions: Vec<FabricLoaderInfo> = response
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析 Fabric 版本列表失败: {}", e)))?;

    Ok(versions
        .into_iter()
        .map(|v| FabricLoaderVersion {
            version: v.loader.version,
            stable: v.loader.stable,
        })
        .collect())
}

/// 获取支持 Fabric 的 Minecraft 版本列表
pub async fn get_fabric_game_versions() -> Result<Vec<String>, LauncherError> {
    let client = Client::new();
    let url = format!("{}/versions/game", FABRIC_META_URL);

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

    let versions: Vec<FabricGameVersion> = response
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析游戏版本列表失败: {}", e)))?;

    Ok(versions.into_iter().map(|v| v.version).collect())
}

// --- 内部数据结构 ---

#[derive(serde::Deserialize)]
struct FabricLoaderInfo {
    loader: FabricLoader,
}

#[derive(serde::Deserialize)]
struct FabricLoader {
    version: String,
    stable: bool,
}

#[derive(serde::Deserialize)]
struct FabricGameVersion {
    version: String,
}

/// Fabric 加载器版本信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct FabricLoaderVersion {
    pub version: String,
    pub stable: bool,
}
