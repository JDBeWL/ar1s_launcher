use crate::errors::LauncherError;
use crate::models::modpack::*;
use crate::services::modpack_installer;

#[tauri::command]
pub async fn search_modrinth_modpacks(
    query: Option<String>,
    game_versions: Option<Vec<String>>,
    // 兼容前端可能传来的不同命名
    versions: Option<Vec<String>>,
    game_version: Option<String>,
    loaders: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    limit: Option<u32>,
    offset: Option<u32>,
    sort_by: Option<String>,
) -> Result<ModrinthSearchResponse, LauncherError> {
    let installer = modpack_installer::ModpackInstaller::new();
    // 合并不同来源的版本参数
    let mut merged_versions: Option<Vec<String>> = game_versions;
    if merged_versions.is_none() {
        merged_versions = versions;
    }
    if merged_versions.is_none() {
        if let Some(single) = game_version {
            merged_versions = Some(vec![single]);
        }
    }

    installer
        .search_modpacks(query, merged_versions, loaders, categories, limit, offset, sort_by)
        .await
}

#[tauri::command]
pub async fn get_modrinth_modpack_versions(
    project_id: String,
    game_versions: Option<Vec<String>>,
    loaders: Option<Vec<String>>,
) -> Result<Vec<ModrinthModpackVersion>, LauncherError> {
    let installer = modpack_installer::ModpackInstaller::new();
    installer
        .get_modpack_versions(&project_id, game_versions, loaders)
        .await
}

#[tauri::command]
pub async fn install_modrinth_modpack(
    options: ModpackInstallOptions,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    let installer = modpack_installer::ModpackInstaller::new();
    installer.install_modrinth_modpack(options, &window).await
}

/// 取消整合包安装
#[tauri::command]
pub async fn cancel_modpack_install() -> Result<(), LauncherError> {
    modpack_installer::set_modpack_cancel_flag();
    Ok(())
}