//! 诊断包 command 模块

use crate::errors::LauncherError;
use crate::services::diagnostics;

#[tauri::command]
pub async fn export_diagnostic_bundle(
    instance_name: String,
    target_path: String,
) -> Result<(), LauncherError> {
    diagnostics::export_bundle(instance_name, target_path).await
}
