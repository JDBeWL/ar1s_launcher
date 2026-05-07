use crate::errors::LauncherError;
use crate::models::AuthType;
use crate::services::config::{load_config, save_config};
use crate::services::microsoft_auth;
use std::sync::Mutex;
use tauri::Emitter;

static PENDING_DEVICE_CODE: Mutex<Option<String>> = Mutex::new(None);
static PENDING_CODE_VERIFIER: Mutex<Option<String>> = Mutex::new(None);

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
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeDisplay {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrosoftLoginResult {
    pub username: String,
    pub uuid: String,
    pub expires_at: i64,
}

#[tauri::command]
pub fn get_auth_status() -> Result<AuthStatus, LauncherError> {
    let config = load_config()?;
    let auth_type = match config.auth_type {
        AuthType::Microsoft => "microsoft",
        AuthType::Offline => "offline",
    };
    let logged_in = if matches!(config.auth_type, AuthType::Microsoft) {
        config.ms_refresh_token.is_some()
    } else {
        config.username.is_some()
    };
    Ok(AuthStatus {
        auth_type: auth_type.to_string(),
        logged_in,
        username: config.username.clone(),
        uuid: config.uuid.clone(),
        expires_at: config.ms_expires_at,
    })
}

#[tauri::command]
pub async fn start_microsoft_login(
    window: tauri::Window,
) -> Result<DeviceCodeDisplay, LauncherError> {
    let device_code_info = microsoft_auth::start_device_code_flow().await?;

    {
        let mut pending = PENDING_DEVICE_CODE
            .lock()
            .map_err(|e| LauncherError::Custom(format!("内部锁错误: {}", e)))?;
        *pending = Some(device_code_info.device_code.clone());
    }

    let display = DeviceCodeDisplay {
        user_code: device_code_info.user_code,
        verification_uri: device_code_info.verification_uri,
        expires_in: device_code_info.expires_in,
    };

    let _ = window.emit(
        "microsoft-login-device-code",
        serde_json::json!({
            "userCode": display.user_code,
            "verificationUri": display.verification_uri,
        }),
    );

    Ok(display)
}

#[tauri::command]
pub async fn complete_microsoft_login() -> Result<MicrosoftLoginResult, LauncherError> {
    let device_code = {
        let mut pending = PENDING_DEVICE_CODE
            .lock()
            .map_err(|e| LauncherError::Custom(format!("内部锁错误: {}", e)))?;
        pending
            .take()
            .ok_or_else(|| LauncherError::Custom("没有进行中的登录流程，请重新开始".to_string()))?
    };

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

                let mut config = (*load_config()?).clone();
                config.auth_type = AuthType::Microsoft;
                config.username = Some(auth_result.username.clone());
                config.uuid = Some(auth_result.uuid.clone());
                config.ms_access_token = Some(auth_result.access_token);
                config.ms_refresh_token = Some(auth_result.refresh_token);
                config.ms_expires_at = Some(auth_result.expires_at);
                save_config(&config)?;

                return Ok(MicrosoftLoginResult {
                    username: auth_result.username,
                    uuid: auth_result.uuid,
                    expires_at: auth_result.expires_at,
                });
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
pub async fn refresh_microsoft_auth() -> Result<MicrosoftLoginResult, LauncherError> {
    let refresh_token = {
        let config = load_config()?;
        config.ms_refresh_token.clone()
            .ok_or_else(|| LauncherError::Custom("未找到 refresh_token，请重新登录".to_string()))?
    };

    let auth_result = microsoft_auth::refresh_and_authenticate(&refresh_token).await?;

    let mut config = (*load_config()?).clone();
    config.auth_type = AuthType::Microsoft;
    config.username = Some(auth_result.username.clone());
    config.uuid = Some(auth_result.uuid.clone());
    config.ms_access_token = Some(auth_result.access_token);
    config.ms_refresh_token = Some(auth_result.refresh_token);
    config.ms_expires_at = Some(auth_result.expires_at);
    save_config(&config)?;

    Ok(MicrosoftLoginResult {
        username: auth_result.username,
        uuid: auth_result.uuid,
        expires_at: auth_result.expires_at,
    })
}

#[tauri::command]
pub async fn logout_microsoft() -> Result<(), LauncherError> {
    let _ = microsoft_auth::revoke_microsoft_token().await;

    let mut config = (*load_config()?).clone();
    config.auth_type = AuthType::Offline;
    config.ms_access_token = None;
    config.ms_refresh_token = None;
    config.ms_expires_at = None;
    save_config(&config)
}

#[tauri::command]
pub fn start_microsoft_auth_code_login() -> Result<String, LauncherError> {
    let flow_info = microsoft_auth::start_auth_code_flow()?;

    {
        let mut pending = PENDING_CODE_VERIFIER
            .lock()
            .map_err(|e| LauncherError::Custom(format!("内部锁错误: {}", e)))?;
        *pending = Some(flow_info.code_verifier);
    }

    Ok(flow_info.auth_url)
}

#[tauri::command]
pub async fn complete_microsoft_auth_code_login() -> Result<MicrosoftLoginResult, LauncherError> {
    let code_verifier = {
        let mut pending = PENDING_CODE_VERIFIER
            .lock()
            .map_err(|e| LauncherError::Custom(format!("内部锁错误: {}", e)))?;
        pending
            .take()
            .ok_or_else(|| LauncherError::Custom("没有进行中的登录流程，请重新开始".to_string()))?
    };

    let auth_result = microsoft_auth::complete_auth_code_flow(&code_verifier).await?;

    let mut config = (*load_config()?).clone();
    config.auth_type = AuthType::Microsoft;
    config.username = Some(auth_result.username.clone());
    config.uuid = Some(auth_result.uuid.clone());
    config.ms_access_token = Some(auth_result.access_token);
    config.ms_refresh_token = Some(auth_result.refresh_token);
    config.ms_expires_at = Some(auth_result.expires_at);
    save_config(&config)?;

    Ok(MicrosoftLoginResult {
        username: auth_result.username,
        uuid: auth_result.uuid,
        expires_at: auth_result.expires_at,
    })
}

#[tauri::command]
pub fn set_auth_type(auth_type: String) -> Result<(), LauncherError> {
    let mut config = (*load_config()?).clone();
    config.auth_type = match auth_type.as_str() {
        "microsoft" => AuthType::Microsoft,
        _ => AuthType::Offline,
    };
    save_config(&config)
}
