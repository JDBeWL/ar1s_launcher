use crate::errors::LauncherError;
use crate::models::JavaCompatibilityResult;
use crate::services::config::load_config;
use crate::services::launcher::java;

#[tauri::command]
pub async fn find_java_installations_command() -> Result<Vec<String>, LauncherError> {
    crate::services::java::find_java_installations_command().await
}

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

#[tauri::command]
pub fn get_custom_java_paths() -> Vec<String> {
    crate::services::config::get_custom_java_paths()
}

#[tauri::command]
pub fn add_custom_java_path(path: String) -> Result<(), LauncherError> {
    crate::services::config::add_custom_java_path(&path)
}

#[tauri::command]
pub fn remove_custom_java_path(path: String) -> Result<(), LauncherError> {
    crate::services::config::remove_custom_java_path(&path)
}

#[tauri::command]
pub fn check_java_compatibility(mc_version: String) -> Result<JavaCompatibilityResult, LauncherError> {
    let config = load_config()?;
    
    // 解析实际的 Minecraft 版本号（处理整合包 ID）
    let actual_mc_version = java::resolve_actual_mc_version(&mc_version, &config.game_dir);
    log::info!("检查 Java 兼容性: {} -> {}", mc_version, actual_mc_version);

    let required = java::required_java_version(&actual_mc_version);
    let (min_ver, max_ver) = java::recommended_java_version_range(&actual_mc_version);

    let current_java_path = config.java_path.clone();
    let current_java_version = current_java_path
        .as_deref()
        .and_then(java::detect_java_version);

    let compatible = current_java_version.map_or(false, |v| v >= required);

    let match_result = java::auto_match_java_for_version(&actual_mc_version);
    let recommended_java_path = match_result.as_ref().map(|r| r.path.clone());
    let recommended_java_version = match_result.as_ref().map(|r| r.version);
    let recommended_java_optimal = match_result.as_ref().map_or(false, |r| r.is_optimal);
    let recommended_java_warning = match_result.as_ref().and_then(|r| r.warning.clone());

    Ok(JavaCompatibilityResult {
        compatible,
        current_java_version,
        required_java_version: required,
        recommended_min_java_version: min_ver,
        recommended_max_java_version: max_ver,
        current_java_path,
        recommended_java_path,
        recommended_java_version,
        recommended_java_optimal,
        recommended_java_warning,
        auto_match_enabled: config.auto_match_java,
    })
}
