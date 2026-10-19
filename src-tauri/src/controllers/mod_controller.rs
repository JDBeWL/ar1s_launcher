//! 模组管理 command 模块
//!
//! Modrinth 模组搜索、版本列表、安装、更新检查与更新的 invoke 入口

use crate::errors::LauncherError;
use crate::models::modpack::ModrinthModpackVersion;
use crate::services::mod_installer::{self, ModInstallResult, ModUpdate};

#[tauri::command]
pub async fn search_modrinth_mods(
    instance_name: String,
    query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    sort_by: Option<String>,
) -> Result<crate::models::modpack::ModrinthSearchResponse, LauncherError> {
    mod_installer::get_installer()
        .search_mods(&instance_name, query, limit, offset, sort_by)
        .await
}

#[tauri::command]
pub async fn get_modrinth_mod_versions(
    instance_name: String,
    project_id: String,
) -> Result<Vec<ModrinthModpackVersion>, LauncherError> {
    mod_installer::get_installer()
        .get_mod_versions(&instance_name, &project_id)
        .await
}

#[tauri::command]
pub async fn install_modrinth_mod(
    instance_name: String,
    project_id: String,
    version_id: Option<String>,
    install_dependencies: Option<bool>,
) -> Result<ModInstallResult, LauncherError> {
    mod_installer::get_installer()
        .install_mod(
            instance_name,
            project_id,
            version_id,
            install_dependencies.unwrap_or(true),
        )
        .await
}

#[tauri::command]
pub async fn check_modrinth_mod_updates(
    instance_name: String,
) -> Result<Vec<ModUpdate>, LauncherError> {
    mod_installer::get_installer()
        .check_updates(instance_name)
        .await
}

#[tauri::command]
pub async fn update_modrinth_mod(
    instance_name: String,
    file_name: String,
    version_id: String,
) -> Result<ModInstallResult, LauncherError> {
    let installed = mod_installer::get_installer()
        .update_mod(instance_name, file_name, version_id)
        .await?;
    Ok(ModInstallResult {
        installed: vec![installed],
    })
}
