use crate::errors::LauncherError;
use crate::models::AuthType;
use crate::services::config::{load_config, save_config};
use crate::services::microsoft_auth::{
    self, DeviceCodeInfo, MicrosoftAuthResult,
};
use tauri::Emitter;

#[tauri::command]
pub fn get_saved_username() -> Result<Option<String>, LauncherError> {
    crate::services::config::get_saved_username()
}

#[tauri::command]
pub fn set_saved_username(username: String) -> Result<(), LauncherError> {
    crate::services::config::set_saved_username(username)
}

#[tauri::command]
pub fn get_saved_uuid() -> Result<Option<String>, LauncherError> {
    crate::services::config::get_saved_uuid()
}

#[tauri::command]
pub fn set_saved_uuid(uuid: String) -> Result<(), LauncherError> {
    crate::services::config::set_saved_uuid(uuid)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub auth_type: String,
    pub logged_in: bool,
    pub username: Option<String>,
    pub uuid: Option<String>,
    pub access_token: Option<String>,
    pub expires_at: Option<i64>,
}

#[tauri::command]
pub fn get_auth_status() -> Result<AuthStatus, LauncherError> {
    let config = load_config()?;
    let auth_type = match config.auth_type {
        AuthType::Microsoft => "microsoft",
        AuthType::Offline => "offline",
    };
    let logged_in = if matches!(config.auth_type, AuthType::Microsoft) {
        config.ms_access_token.is_some() && config.ms_refresh_token.is_some()
    } else {
        config.username.is_some()
    };
    Ok(AuthStatus {
        auth_type: auth_type.to_string(),
        logged_in,
        username: config.username,
        uuid: config.uuid,
        access_token: config.ms_access_token,
        expires_at: config.ms_expires_at,
    })
}

#[tauri::command]
pub async fn start_microsoft_login(
    window: tauri::Window,
) -> Result<DeviceCodeInfo, LauncherError> {
    let device_code_info = microsoft_auth::start_device_code_flow().await?;

    let _ = window.emit(
        "microsoft-login-device-code",
        serde_json::json!({
            "userCode": device_code_info.user_code,
            "verificationUri": device_code_info.verification_uri,
        }),
    );

    Ok(device_code_info)
}

#[tauri::command]
pub async fn complete_microsoft_login(
    device_code: String,
) -> Result<MicrosoftAuthResult, LauncherError> {
    let mut interval = 5u64;
    let max_attempts = 180 / interval as u32;

    for _ in 0..max_attempts {
        let result = microsoft_auth::poll_device_token(&device_code).await;

        match result {
            Ok(token_resp) => {
                let refresh = token_resp
                    .refresh_token
                    .clone()
                    .unwrap_or_default();
                let expires_in = token_resp.expires_in.unwrap_or(3600);

                let auth_result = microsoft_auth::complete_microsoft_auth(
                    &token_resp.access_token,
                    &refresh,
                    expires_in,
                )
                .await?;

                let mut config = load_config()?;
                config.auth_type = AuthType::Microsoft;
                config.username = Some(auth_result.username.clone());
                config.uuid = Some(auth_result.uuid.clone());
                config.ms_access_token = Some(auth_result.access_token.clone());
                config.ms_refresh_token = Some(auth_result.refresh_token.clone());
                config.ms_expires_at = Some(auth_result.expires_at);
                save_config(&config)?;

                return Ok(auth_result);
            }
            Err(LauncherError::Custom(msg))
                if msg == "authorization_pending" || msg == "slow_down" =>
            {
                if msg == "slow_down" {
                    interval = (interval + 5).min(15);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(LauncherError::Custom(
        "登录超时，请重新尝试".to_string(),
    ))
}

#[tauri::command]
pub async fn refresh_microsoft_auth() -> Result<MicrosoftAuthResult, LauncherError> {
    let config = load_config()?;
    let refresh_token = config
        .ms_refresh_token
        .ok_or_else(|| LauncherError::Custom("未找到 refresh_token，请重新登录".to_string()))?;

    let auth_result = microsoft_auth::refresh_and_authenticate(&refresh_token).await?;

    let mut config = load_config()?;
    config.auth_type = AuthType::Microsoft;
    config.username = Some(auth_result.username.clone());
    config.uuid = Some(auth_result.uuid.clone());
    config.ms_access_token = Some(auth_result.access_token.clone());
    config.ms_refresh_token = Some(auth_result.refresh_token.clone());
    config.ms_expires_at = Some(auth_result.expires_at);
    save_config(&config)?;

    Ok(auth_result)
}

#[tauri::command]
pub fn logout_microsoft() -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.auth_type = AuthType::Offline;
    config.ms_access_token = None;
    config.ms_refresh_token = None;
    config.ms_expires_at = None;
    save_config(&config)
}

#[tauri::command]
pub fn set_auth_type(auth_type: String) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.auth_type = match auth_type.as_str() {
        "microsoft" => AuthType::Microsoft,
        _ => AuthType::Offline,
    };
    save_config(&config)
}
