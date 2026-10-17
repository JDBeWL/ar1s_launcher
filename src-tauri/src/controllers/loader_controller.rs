//! Mod 加载器 command 模块
//!
//! 四种加载器版本查询的 invoke 入口，Fabric/Quilt/NeoForge 收敛成统一的 LoaderVersionInfo

use crate::errors::LauncherError;
use crate::services::loaders::{
    fabric,
    forge::{self, ForgeVersion},
    neoforge, quilt,
};
use serde::Serialize;

/// 四种加载器统一成这一种结构返回
#[derive(Debug, Clone, Serialize)]
pub struct LoaderVersionInfo {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stable: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableLoaders {
    pub forge: bool,
    pub fabric: bool,
    pub quilt: bool,
    pub neoforge: bool,
}

#[tauri::command]
pub async fn get_forge_versions(
    minecraft_version: String,
) -> Result<Vec<ForgeVersion>, LauncherError> {
    forge::get_forge_versions(&minecraft_version).await
}

#[tauri::command]
pub async fn get_fabric_versions(
    minecraft_version: String,
) -> Result<Vec<LoaderVersionInfo>, LauncherError> {
    let versions = fabric::get_fabric_versions(&minecraft_version).await?;
    Ok(versions
        .into_iter()
        .map(|v| LoaderVersionInfo {
            version: v.version,
            stable: Some(v.stable),
        })
        .collect())
}

#[tauri::command]
pub async fn get_quilt_versions(
    minecraft_version: String,
) -> Result<Vec<LoaderVersionInfo>, LauncherError> {
    let versions = quilt::get_quilt_versions(&minecraft_version).await?;
    Ok(versions
        .into_iter()
        .map(|v| LoaderVersionInfo {
            version: v.version,
            stable: Some(v.stable),
        })
        .collect())
}

#[tauri::command]
pub async fn get_neoforge_versions(
    minecraft_version: String,
) -> Result<Vec<LoaderVersionInfo>, LauncherError> {
    let versions = neoforge::get_neoforge_versions(&minecraft_version).await?;
    Ok(versions
        .into_iter()
        .map(|v| LoaderVersionInfo {
            version: v.version,
            stable: Some(v.stable),
        })
        .collect())
}

#[tauri::command]
pub async fn get_available_loaders(
    minecraft_version: String,
) -> Result<AvailableLoaders, LauncherError> {
    let (forge_result, fabric_result, quilt_result, neoforge_result) = tokio::join!(
        check_forge_available(&minecraft_version),
        check_fabric_available(&minecraft_version),
        check_quilt_available(&minecraft_version),
        check_neoforge_available(&minecraft_version),
    );

    Ok(AvailableLoaders {
        forge: forge_result,
        fabric: fabric_result,
        quilt: quilt_result,
        neoforge: neoforge_result,
    })
}

async fn check_forge_available(mc_version: &str) -> bool {
    match forge::get_forge_versions(mc_version).await {
        Ok(versions) => !versions.is_empty(),
        Err(_) => false,
    }
}

async fn check_fabric_available(mc_version: &str) -> bool {
    match fabric::get_fabric_versions(mc_version).await {
        Ok(versions) => !versions.is_empty(),
        Err(_) => false,
    }
}

async fn check_quilt_available(mc_version: &str) -> bool {
    match quilt::get_quilt_versions(mc_version).await {
        Ok(versions) => !versions.is_empty(),
        Err(_) => false,
    }
}

async fn check_neoforge_available(mc_version: &str) -> bool {
    match neoforge::get_neoforge_versions(mc_version).await {
        Ok(versions) => !versions.is_empty(),
        Err(_) => false,
    }
}
