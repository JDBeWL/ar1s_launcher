//! 资源包 command 模块
//!
//! 资源包列表、启停、删除、导入，以及 Modrinth 搜索、安装、更新检查的 invoke 入口

use crate::errors::LauncherError;
use crate::models::modpack::{ModrinthModpackVersion, ModrinthSearchResponse};
use crate::services::resourcepack::{self, ImportSummary, ResourcePackInfo};
use crate::services::resourcepack_installer::{self, InstalledResourcePack, ResourcePackUpdate};

#[tauri::command]
pub fn get_resource_packs(instance_name: String) -> Result<Vec<ResourcePackInfo>, LauncherError> {
    resourcepack::list_resource_packs(&instance_name)
}

#[tauri::command]
pub async fn set_resource_pack_enabled(
    instance_name: String,
    file_name: String,
    enabled: bool,
) -> Result<(), LauncherError> {
    resourcepack::set_resource_pack_enabled(instance_name, file_name, enabled).await
}

#[tauri::command]
pub async fn delete_instance_resource_pack(
    instance_name: String,
    file_name: String,
) -> Result<(), LauncherError> {
    resourcepack::delete_resource_pack(instance_name, file_name).await
}

#[tauri::command]
pub async fn import_resource_packs(
    instance_name: String,
    source_paths: Vec<String>,
) -> Result<ImportSummary, LauncherError> {
    resourcepack::import_resource_packs(instance_name, source_paths).await
}

#[tauri::command]
pub async fn open_resourcepacks_folder(instance_name: String) -> Result<(), LauncherError> {
    resourcepack::open_resourcepacks_folder(instance_name).await
}

#[tauri::command]
pub async fn search_modrinth_resource_packs(
    instance_name: String,
    query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    sort_by: Option<String>,
) -> Result<ModrinthSearchResponse, LauncherError> {
    resourcepack_installer::get_installer()
        .search(&instance_name, query, limit, offset, sort_by)
        .await
}

#[tauri::command]
pub async fn get_modrinth_resource_pack_versions(
    instance_name: String,
    project_id: String,
) -> Result<Vec<ModrinthModpackVersion>, LauncherError> {
    resourcepack_installer::get_installer()
        .get_versions(&instance_name, &project_id)
        .await
}

#[tauri::command]
pub async fn install_modrinth_resource_pack(
    instance_name: String,
    project_id: String,
    version_id: Option<String>,
) -> Result<InstalledResourcePack, LauncherError> {
    resourcepack_installer::get_installer()
        .install(instance_name, project_id, version_id)
        .await
}

#[tauri::command]
pub async fn check_modrinth_resource_pack_updates(
    instance_name: String,
) -> Result<Vec<ResourcePackUpdate>, LauncherError> {
    resourcepack_installer::get_installer()
        .check_updates(instance_name)
        .await
}

#[tauri::command]
pub async fn update_modrinth_resource_pack(
    instance_name: String,
    file_name: String,
    version_id: String,
) -> Result<InstalledResourcePack, LauncherError> {
    resourcepack_installer::get_installer()
        .update(instance_name, file_name, version_id)
        .await
}
