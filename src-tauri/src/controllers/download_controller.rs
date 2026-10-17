//! 下载 command 模块
//!
//! 版本清单、体积预估与版本下载的 invoke 入口，取消走全局标志加事件双通道

use crate::errors::LauncherError;
use crate::models::VersionManifest;
use crate::services::download;
use crate::services::download::VersionSizeInfo;
use crate::services::download::batch::set_cancel_flag;
use tauri::{Emitter, Window};

#[tauri::command]
pub async fn get_versions() -> Result<VersionManifest, LauncherError> {
    download::get_versions().await
}

#[tauri::command]
pub async fn get_version_size(
    version_id: String,
    mirror: Option<String>,
) -> Result<VersionSizeInfo, LauncherError> {
    download::get_version_size(version_id, mirror).await
}

#[tauri::command]
pub async fn download_version(
    version_id: String,
    mirror: Option<String>,
    window: Window,
) -> Result<(), LauncherError> {
    download::process_and_download_version(version_id, mirror, &window).await
}

#[tauri::command]
pub async fn cancel_download(window: Window) -> Result<(), LauncherError> {
    set_cancel_flag();
    // 事件是给前端监听器用的，和上面的全局标志是两条独立通道
    window
        .emit("cancel-download", ())
        .map_err(|e| LauncherError::Custom(format!("发送取消事件失败: {}", e)))?;
    Ok(())
}
