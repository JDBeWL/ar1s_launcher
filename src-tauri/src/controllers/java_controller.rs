use crate::errors::LauncherError;

#[tauri::command]
pub async fn find_java_installations_command() -> Result<Vec<String>, LauncherError> {
    crate::services::java::find_java_installations_command().await
}

/// 强制刷新 Java 安装列表（忽略缓存）
#[tauri::command]
pub async fn refresh_java_installations() -> Result<Vec<String>, LauncherError> {
    crate::services::java::refresh_java_installations().await
}

#[tauri::command]
pub async fn set_java_path_command(path: String) -> Result<(), LauncherError> {
    crate::services::java::set_java_path_command(path).await
}

#[tauri::command]
pub async fn validate_java_path(path: String) -> Result<bool, LauncherError> {
    crate::services::java::validate_java_path(path).await
}

#[tauri::command]
pub async fn get_java_version(path: String) -> Result<String, LauncherError> {
    crate::services::java::get_java_version(path).await
}
