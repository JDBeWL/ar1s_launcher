use crate::errors::LauncherError;
use crate::models::*;
use crate::services::config;

#[tauri::command(rename = "get_config")]
pub async fn get_config() -> Result<GameConfig, LauncherError> {
    config::load_config()
}

#[tauri::command]
pub async fn load_config_key(key: String) -> Result<Option<String>, LauncherError> {
    config::load_config_key(key).await
}

#[tauri::command]
pub async fn save_config_key(key: String, value: String) -> Result<(), LauncherError> {
    config::save_config_key(key, value).await
}

#[tauri::command]
pub fn get_game_dir() -> Result<String, LauncherError> {
    config::get_game_dir()
}

#[tauri::command]
pub async fn select_game_dir(_window: tauri::Window) -> Result<String, LauncherError> {
    // This command is now just a proxy. The front-end should handle the dialog.
    config::get_game_dir()
}

#[tauri::command]
pub async fn get_game_dir_info() -> Result<GameDirInfo, LauncherError> {
    config::get_game_dir_info().await
}

#[tauri::command]
pub async fn set_game_dir(path: String, window: tauri::Window) -> Result<(), LauncherError> {
    config::set_game_dir(path, &window).await
}

#[tauri::command]
pub async fn set_version_isolation(enabled: bool) -> Result<(), LauncherError> {
    config::set_version_isolation(enabled).await
}

#[tauri::command]
pub fn get_download_threads() -> Result<u8, LauncherError> {
    config::get_download_threads()
}

#[tauri::command]
pub async fn set_download_threads(threads: u8) -> Result<(), LauncherError> {
    config::set_download_threads(threads).await
}

#[tauri::command]
pub async fn validate_version_files(version_id: String) -> Result<Vec<String>, LauncherError> {
    config::validate_version_files(version_id).await
}

#[tauri::command]
pub fn get_total_memory() -> u64 {
    config::get_total_memory()
}
