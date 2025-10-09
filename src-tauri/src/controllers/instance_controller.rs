use crate::services::instance;
use crate::errors::LauncherError;

use crate::models::ForgeVersion;

#[tauri::command]
pub async fn create_instance(new_instance_name: String, base_version_id: String, forge_version: Option<ForgeVersion>, window: tauri::Window) -> Result<(), LauncherError> {
    instance::create_instance(new_instance_name, base_version_id, forge_version, &window).await
}
