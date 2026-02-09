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
    // 合并不同来源的版本参数
    let merged_versions = game_versions
        .or(versions)
        .or_else(|| game_version.map(|single| vec![single]));

    modpack_installer::get_installer()
        .search_modpacks(query, merged_versions, loaders, categories, limit, offset, sort_by)
        .await
}

#[tauri::command]
pub async fn get_modrinth_modpack_versions(
    project_id: String,
    game_versions: Option<Vec<String>>,
    loaders: Option<Vec<String>>,
) -> Result<Vec<ModrinthModpackVersion>, LauncherError> {
    modpack_installer::get_installer()
        .get_modpack_versions(&project_id, game_versions, loaders)
        .await
}

#[tauri::command]
pub async fn install_modrinth_modpack(
    options: ModpackInstallOptions,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    modpack_installer::get_installer()
        .install_modrinth_modpack(options, &window)
        .await
}

/// 取消整合包安装
#[tauri::command]
pub fn cancel_modpack_install() -> Result<(), LauncherError> {
    modpack_installer::set_modpack_cancel_flag();
    Ok(())
}