use crate::errors::LauncherError;

// 控制器层作为 #[tauri::command] 入口，调用config服务中的认证相关方法

#[tauri::command]
pub async fn get_saved_username() -> Result<Option<String>, LauncherError> {
    crate::services::config::get_saved_username().await
}

#[tauri::command]
pub async fn set_saved_username(username: String) -> Result<(), LauncherError> {
    crate::services::config::set_saved_username(username).await
}

#[tauri::command]
pub async fn get_saved_uuid() -> Result<Option<String>, LauncherError> {
    crate::services::config::get_saved_uuid().await
}

#[tauri::command]
pub async fn set_saved_uuid(uuid: String) -> Result<(), LauncherError> {
    crate::services::config::set_saved_uuid(uuid).await
}
