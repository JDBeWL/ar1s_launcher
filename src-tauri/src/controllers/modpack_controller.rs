//! 整合包 command 模块
//!
//! Modrinth 搜索、版本列表、安装与取消安装的 invoke 入口

use crate::errors::LauncherError;
use crate::models::modpack::*;
use crate::services::modpack_installer;

#[tauri::command]
pub async fn search_modrinth_modpacks(
    query: Option<String>,
    game_versions: Option<Vec<String>>,
    loaders: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    limit: Option<u32>,
    offset: Option<u32>,
    sort_by: Option<String>,
) -> Result<ModrinthSearchResponse, LauncherError> {
    modpack_installer::get_installer()
        .search_modpacks(
            query,
            game_versions,
            loaders,
            categories,
            limit,
            offset,
            sort_by,
        )
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

#[tauri::command]
pub fn cancel_modpack_install() -> Result<(), LauncherError> {
    modpack_installer::set_modpack_cancel_flag();
    Ok(())
}
