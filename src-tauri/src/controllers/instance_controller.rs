use crate::errors::LauncherError;
use crate::models::InstanceInfo;
use crate::services::instance::{self, ModInfo};
use crate::services::loaders::LoaderType;
use crate::utils::file_utils::{InstanceNameValidation, validate_instance_name};

/// 验证实例名称是否有效
#[tauri::command]
pub fn validate_instance_name_cmd(name: String) -> InstanceNameValidation {
    validate_instance_name(&name)
}

/// 检查实例名称是否可用（验证格式并检查是否已存在）
#[tauri::command]
pub fn check_instance_name_available(name: String) -> InstanceNameValidation {
    instance::check_instance_name_available(&name)
}

#[tauri::command]
pub async fn create_instance(
    new_instance_name: String,
    base_version_id: String,
    loader: Option<LoaderType>,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    instance::create_instance(new_instance_name, base_version_id, loader, &window).await
}

#[tauri::command]
pub async fn get_instances() -> Result<Vec<InstanceInfo>, LauncherError> {
    instance::get_instances().await
}

#[tauri::command]
pub async fn delete_instance(instance_name: String) -> Result<(), LauncherError> {
    instance::delete_instance(instance_name).await
}

#[tauri::command]
pub async fn rename_instance(old_name: String, new_name: String) -> Result<(), LauncherError> {
    instance::rename_instance(old_name, new_name).await
}

#[tauri::command]
pub async fn open_instance_folder(instance_name: String) -> Result<(), LauncherError> {
    instance::open_instance_folder(instance_name).await
}

#[tauri::command]
pub async fn launch_instance(
    instance_name: String,
    override_java_path: Option<String>,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    instance::launch_instance(instance_name, override_java_path, window).await
}

#[tauri::command]
pub async fn get_instance_mods(instance_name: String) -> Result<Vec<ModInfo>, LauncherError> {
    instance::get_instance_mods(instance_name)
}

#[tauri::command]
pub async fn set_mod_enabled(
    instance_name: String,
    file_name: String,
    enabled: bool,
) -> Result<(), LauncherError> {
    instance::set_mod_enabled(instance_name, file_name, enabled)
}

#[tauri::command]
pub async fn delete_instance_mod(
    instance_name: String,
    file_name: String,
) -> Result<(), LauncherError> {
    instance::delete_instance_mod(instance_name, file_name)
}
