//! Fabric 加载器安装
//!
//! 从 meta.fabricmc.net v2 取 profile JSON，改写 id 为实例名后写入 versions 目录

use crate::errors::LauncherError;
use crate::services::http_client;
use log::info;
use serde_json::Value;
use std::fs;
use std::path::Path;

const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2";

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

    let client = http_client::get_client();

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

    if let Some(obj) = version_json.as_object_mut() {
        obj.insert("id".to_string(), serde_json::json!(instance_name));
    }

    let version_dir = game_dir.join("versions").join(instance_name);
    fs::create_dir_all(&version_dir)?;

    let json_path = version_dir.join(format!("{}.json", instance_name));
    fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

    info!("Fabric 版本 JSON 已创建: {}", json_path.display());

    Ok(())
}

pub async fn get_fabric_versions(
    mc_version: &str,
) -> Result<Vec<FabricLoaderVersion>, LauncherError> {
    let client = http_client::get_client();
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

pub async fn get_fabric_game_versions() -> Result<Vec<String>, LauncherError> {
    let client = http_client::get_client();
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct FabricLoaderVersion {
    pub version: String,
    pub stable: bool,
}
