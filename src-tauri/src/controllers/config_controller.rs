use crate::errors::LauncherError;
use crate::models::*;
use crate::services::config;

#[tauri::command(rename = "get_config")]
pub fn get_config() -> Result<GameConfig, LauncherError> {
    config::load_config()
}

#[tauri::command]
pub fn load_config_key(key: String) -> Result<Option<String>, LauncherError> {
    config::load_config_key(key)
}

#[tauri::command]
pub fn save_config_key(key: String, value: String) -> Result<(), LauncherError> {
    config::save_config_key(key, value)
}

#[tauri::command]
pub fn get_game_dir() -> Result<String, LauncherError> {
    config::get_game_dir()
}

#[tauri::command]
pub fn select_game_dir(_window: tauri::Window) -> Result<String, LauncherError> {
    // This command is now just a proxy. The front-end should handle the dialog.
    config::get_game_dir()
}

#[tauri::command]
pub fn get_game_dir_info() -> Result<GameDirInfo, LauncherError> {
    config::get_game_dir_info()
}

#[tauri::command]
pub fn set_game_dir(path: String, window: tauri::Window) -> Result<(), LauncherError> {
    config::set_game_dir(path, &window)
}

#[tauri::command]
pub fn set_version_isolation(enabled: bool) -> Result<(), LauncherError> {
    config::set_version_isolation(enabled)
}

#[tauri::command]
pub fn get_download_threads() -> Result<u8, LauncherError> {
    config::get_download_threads()
}

#[tauri::command]
pub fn set_download_threads(threads: u8) -> Result<(), LauncherError> {
    config::set_download_threads(threads)
}

#[tauri::command]
pub async fn validate_version_files(version_id: String) -> Result<Vec<String>, LauncherError> {
    crate::services::file_verification::validate_version_files(version_id).await
}

#[tauri::command]
pub fn get_total_memory() -> u64 {
    config::get_total_memory()
}

#[tauri::command]
pub fn get_memory_stats() -> Result<crate::services::memory::MemoryStats, LauncherError> {
    config::get_memory_stats()
}

#[tauri::command]
pub fn recommend_memory(
    version: String,
    modded: bool,
) -> Result<crate::services::memory::MemoryRecommendation, LauncherError> {
    config::recommend_memory(version, modded)
}

#[tauri::command]
pub fn validate_memory_setting(memory_mb: u32) -> Result<bool, LauncherError> {
    config::validate_memory_setting(memory_mb)
}

#[tauri::command]
pub fn check_memory_warning(memory_mb: u32) -> Result<Option<String>, LauncherError> {
    config::check_memory_warning(memory_mb)
}

#[tauri::command]
pub fn get_auto_memory_config(
) -> Result<crate::services::memory::AutoMemoryConfig, LauncherError> {
    config::get_auto_memory_config()
}

#[tauri::command]
pub fn set_auto_memory_enabled(enabled: bool) -> Result<(), LauncherError> {
    config::set_auto_memory_enabled(enabled)
}

#[tauri::command]
pub fn auto_set_memory() -> Result<Option<u32>, LauncherError> {
    config::auto_set_memory()
}

#[tauri::command]
pub fn analyze_memory_efficiency(memory_mb: u32) -> Result<String, LauncherError> {
    config::analyze_memory_efficiency(memory_mb)
}


/// 窗口设置
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WindowSettings {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fullscreen: bool,
}

#[tauri::command]
pub fn get_window_settings() -> Result<WindowSettings, LauncherError> {
    let config = config::load_config()?;
    Ok(WindowSettings {
        width: config.window_width,
        height: config.window_height,
        fullscreen: config.fullscreen,
    })
}

#[tauri::command]
pub fn set_window_settings(width: Option<u32>, height: Option<u32>, fullscreen: bool) -> Result<(), LauncherError> {
    let mut config = config::load_config()?;
    config.window_width = width;
    config.window_height = height;
    config.fullscreen = fullscreen;
    config::save_config(&config)?;
    Ok(())
}

#[tauri::command]
pub fn get_last_selected_version() -> Option<String> {
    config::get_last_selected_version()
}

#[tauri::command]
pub fn set_last_selected_version(version: String) -> Result<(), LauncherError> {
    config::set_last_selected_version(&version)
}
