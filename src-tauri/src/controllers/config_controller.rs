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

#[tauri::command]
pub async fn get_memory_stats() -> Result<crate::services::memory::MemoryStats, LauncherError> {
    config::get_memory_stats().await
}

#[tauri::command]
pub async fn recommend_memory(version: String, modded: bool) -> Result<crate::services::memory::MemoryRecommendation, LauncherError> {
    config::recommend_memory(version, modded).await
}

#[tauri::command]
pub async fn validate_memory_setting(memory_mb: u32) -> Result<bool, LauncherError> {
    config::validate_memory_setting(memory_mb).await
}

#[tauri::command]
pub async fn check_memory_warning(memory_mb: u32) -> Result<Option<String>, LauncherError> {
    config::check_memory_warning(memory_mb).await
}

#[tauri::command]
pub async fn get_auto_memory_config() -> Result<crate::services::memory::AutoMemoryConfig, LauncherError> {
    config::get_auto_memory_config().await
}

#[tauri::command]
pub async fn set_auto_memory_enabled(enabled: bool) -> Result<(), LauncherError> {
    config::set_auto_memory_enabled(enabled).await
}

#[tauri::command]
pub async fn auto_set_memory() -> Result<Option<u32>, LauncherError> {
    config::auto_set_memory().await
}

#[tauri::command]
pub async fn analyze_memory_efficiency(memory_mb: u32) -> Result<String, LauncherError> {
    config::analyze_memory_efficiency(memory_mb).await
}
