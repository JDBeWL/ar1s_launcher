use crate::errors::LauncherError;
use crate::models::LaunchOptions;

#[tauri::command]
pub async fn launch_minecraft(options: LaunchOptions, window: tauri::Window) -> Result<(), LauncherError> {
    crate::services::launcher::launch_minecraft(options, window).await
}
