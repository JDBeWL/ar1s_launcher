use crate::errors::LauncherError;
use crate::models::VersionManifest;
use crate::services::download;
use tauri::Window;

/// 获取 Minecraft 版本列表
#[tauri::command]
pub async fn get_versions() -> Result<VersionManifest, LauncherError> {
    download::get_versions().await
}

/// 下载 Minecraft 版本
#[tauri::command]
pub async fn download_version(
    version_id: String,
    mirror: Option<String>,
    window: Window,
) -> Result<(), LauncherError> {
    download::process_and_download_version(version_id, mirror, &window).await
}