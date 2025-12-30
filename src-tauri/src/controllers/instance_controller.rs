use crate::errors::LauncherError;
use crate::models::InstanceInfo;
use crate::services::instance;
use crate::services::loaders::LoaderType;


#[tauri::command]
pub async fn create_instance(
    new_instance_name: String,
    base_version_id: String,
    loader: Option<LoaderType>,
    window: tauri::Window
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
pub async fn launch_instance(instance_name: String, window: tauri::Window) -> Result<(), LauncherError> {
    instance::launch_instance(instance_name, window).await
}