use crate::services::forge;
use crate::errors::LauncherError;
use crate::models::ForgeVersion;

#[tauri::command]
pub async fn get_forge_versions(minecraft_version: String) -> Result<Vec<ForgeVersion>, LauncherError> {
    forge::get_forge_versions(minecraft_version).await
}
