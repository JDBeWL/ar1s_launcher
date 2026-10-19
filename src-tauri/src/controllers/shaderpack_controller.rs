//! 光影包 command 模块
//!
//! 光影包列表、选择/启停、删除、导入，以及 Modrinth 搜索、安装、更新检查的 invoke 入口

use crate::errors::LauncherError;
use crate::models::modpack::{ModrinthModpackVersion, ModrinthSearchResponse};
use crate::services::shaderpack::{self, ImportSummary, ShaderPackList};
use crate::services::shaderpack_installer::{self, InstalledShaderPack, ShaderPackUpdate};

#[tauri::command]
pub fn get_shader_packs(instance_name: String) -> Result<ShaderPackList, LauncherError> {
    shaderpack::list_shader_packs(&instance_name)
}

/// `file_name` 为 None 表示关闭光影；传名字则选中并启用该包
#[tauri::command]
pub async fn set_shader_pack(
    instance_name: String,
    file_name: Option<String>,
) -> Result<ShaderPackList, LauncherError> {
    shaderpack::set_shader_pack(instance_name, file_name).await
}

#[tauri::command]
pub async fn set_shaders_enabled(
    instance_name: String,
    enabled: bool,
) -> Result<ShaderPackList, LauncherError> {
    shaderpack::set_shaders_enabled(instance_name, enabled).await
}

#[tauri::command]
pub async fn delete_instance_shader_pack(
    instance_name: String,
    file_name: String,
) -> Result<(), LauncherError> {
    shaderpack::delete_shader_pack(instance_name, file_name).await
}

#[tauri::command]
pub async fn import_shader_packs(
    instance_name: String,
    source_paths: Vec<String>,
) -> Result<ImportSummary, LauncherError> {
    shaderpack::import_shader_packs(instance_name, source_paths).await
}

#[tauri::command]
pub async fn open_shaderpacks_folder(instance_name: String) -> Result<(), LauncherError> {
    shaderpack::open_shaderpacks_folder(instance_name).await
}

#[tauri::command]
pub async fn search_modrinth_shader_packs(
    instance_name: String,
    query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    sort_by: Option<String>,
) -> Result<ModrinthSearchResponse, LauncherError> {
    shaderpack_installer::get_installer()
        .search(&instance_name, query, limit, offset, sort_by)
        .await
}

#[tauri::command]
pub async fn get_modrinth_shader_pack_versions(
    instance_name: String,
    project_id: String,
) -> Result<Vec<ModrinthModpackVersion>, LauncherError> {
    shaderpack_installer::get_installer()
        .get_versions(&instance_name, &project_id)
        .await
}

#[tauri::command]
pub async fn install_modrinth_shader_pack(
    instance_name: String,
    project_id: String,
    version_id: Option<String>,
) -> Result<InstalledShaderPack, LauncherError> {
    shaderpack_installer::get_installer()
        .install(instance_name, project_id, version_id)
        .await
}

#[tauri::command]
pub async fn check_modrinth_shader_pack_updates(
    instance_name: String,
) -> Result<Vec<ShaderPackUpdate>, LauncherError> {
    shaderpack_installer::get_installer()
        .check_updates(instance_name)
        .await
}

#[tauri::command]
pub async fn update_modrinth_shader_pack(
    instance_name: String,
    file_name: String,
    version_id: String,
) -> Result<InstalledShaderPack, LauncherError> {
    shaderpack_installer::get_installer()
        .update(instance_name, file_name, version_id)
        .await
}
