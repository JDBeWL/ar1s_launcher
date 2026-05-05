//! Microsoft / Minecraft 正版登录模块
//!
//! 实现完整的 Microsoft OAuth → Xbox Live → XSTS → Minecraft 认证链：
//! 1. Microsoft OAuth 2.0 授权（设备代码流）
//! 2. Xbox Live 认证
//! 3. XSTS 认证
//! 4. Minecraft 认证
//! 5. Minecraft Profile 获取

use crate::errors::LauncherError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MICROSOFT_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

fn get_auth_client() -> Result<Client, LauncherError> {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(30))
        .user_agent("Ar1s-Launcher/1.0")
        .build()
        .map_err(|e| LauncherError::Custom(format!("HTTP 客户端构建失败: {}", e)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrosoftAuthResult {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    user_code: String,
    device_code: String,
    verification_uri: String,
    interval: u64,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxLiveAuthResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XstsAuthResponse {
    token: String,
    display_claims: XstsDisplayClaims,
}

#[derive(Debug, Deserialize)]
struct XstsDisplayClaims {
    xui: Vec<XstsUserClaim>,
}

#[derive(Debug, Deserialize)]
struct XstsUserClaim {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftAuthResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeInfo {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

pub async fn start_device_code_flow() -> Result<DeviceCodeInfo, LauncherError> {
    let client = get_auth_client()?;
    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("设备代码请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(LauncherError::Custom(format!(
            "设备代码请求失败 ({}): {}",
            status, body
        )));
    }

    let data: DeviceCodeResponse = resp
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析设备代码响应失败: {}", e)))?;

    Ok(DeviceCodeInfo {
        user_code: data.user_code,
        device_code: data.device_code,
        verification_uri: data.verification_uri,
        interval: data.interval,
        expires_in: data.expires_in,
    })
}

pub async fn poll_device_token(device_code: &str) -> Result<TokenResponse, LauncherError> {
    let client = get_auth_client()?;
    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("Token 轮询请求失败: {}", e)))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if status.as_u16() == 400 {
        if body.contains("authorization_pending") {
            return Err(LauncherError::Custom("authorization_pending".to_string()));
        }
        if body.contains("slow_down") {
            return Err(LauncherError::Custom("slow_down".to_string()));
        }
        if body.contains("expired_token") {
            return Err(LauncherError::Custom("设备代码已过期，请重新登录".to_string()));
        }
        if body.contains("declined") {
            return Err(LauncherError::Custom("用户拒绝了授权".to_string()));
        }
    }

    if !status.is_success() {
        return Err(LauncherError::Custom(format!(
            "Token 请求失败 ({}): {}",
            status, body
        )));
    }

    serde_json::from_str(&body)
        .map_err(|e| LauncherError::Custom(format!("解析 Token 响应失败: {}", e)))
}

pub async fn refresh_microsoft_token(refresh_token: &str) -> Result<TokenResponse, LauncherError> {
    let client = get_auth_client()?;
    let resp = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id", MICROSOFT_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("刷新 Token 请求失败: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(LauncherError::Custom(format!(
            "刷新 Token 失败 ({}): {}",
            status, body
        )));
    }

    resp.json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析刷新 Token 响应失败: {}", e)))
}

async fn authenticate_xbox_live(microsoft_access_token: &str) -> Result<String, LauncherError> {
    let client = get_auth_client()?;
    let body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", microsoft_access_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let resp = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&body)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("Xbox Live 认证请求失败: {}", e)))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(LauncherError::Custom(format!(
            "Xbox Live 认证失败 ({}): {}",
            status, body_text
        )));
    }

    let data: XboxLiveAuthResponse = serde_json::from_str(&body_text)
        .map_err(|e| LauncherError::Custom(format!("解析 Xbox Live 响应失败: {} (body: {})", e, body_text.chars().take(500).collect::<String>())))?;

    Ok(data.token)
}

async fn authenticate_xsts(xbox_live_token: &str) -> Result<(String, String), LauncherError> {
    let client = get_auth_client()?;
    let body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbox_live_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let resp = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&body)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("XSTS 认证请求失败: {}", e)))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        if status.as_u16() == 401 {
            return Err(LauncherError::Custom(
                "该 Microsoft 账户未绑定 Minecraft 正版，请确认已购买游戏".to_string(),
            ));
        }
        return Err(LauncherError::Custom(format!(
            "XSTS 认证失败 ({}): {}",
            status, body_text
        )));
    }

    let data: XstsAuthResponse = serde_json::from_str(&body_text)
        .map_err(|e| LauncherError::Custom(format!("解析 XSTS 响应失败: {} (body: {})", e, body_text.chars().take(500).collect::<String>())))?;

    let uhs = data
        .display_claims
        .xui
        .first()
        .ok_or_else(|| LauncherError::Custom("XSTS 响应中缺少 uhs".to_string()))?
        .uhs
        .clone();

    Ok((data.token, uhs))
}

async fn authenticate_minecraft(
    xsts_token: &str,
    uhs: &str,
) -> Result<String, LauncherError> {
    let client = get_auth_client()?;
    let body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token)
    });

    let resp = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&body)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("Minecraft 认证请求失败: {}", e)))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(LauncherError::Custom(format!(
            "Minecraft 认证失败 ({}): {}",
            status, body_text
        )));
    }

    let data: MinecraftAuthResponse = serde_json::from_str(&body_text)
        .map_err(|e| LauncherError::Custom(format!("解析 Minecraft 认证响应失败: {} (body: {})", e, body_text.chars().take(500).collect::<String>())))?;

    Ok(data.access_token)
}

async fn get_minecraft_profile(
    minecraft_access_token: &str,
) -> Result<(String, String), LauncherError> {
    let client = get_auth_client()?;
    let resp = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Authorization", format!("Bearer {}", minecraft_access_token))
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("获取 Minecraft 个人资料失败: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if status.as_u16() == 404 {
            return Err(LauncherError::Custom(
                "该 Microsoft 账户未拥有 Minecraft 正版".to_string(),
            ));
        }
        return Err(LauncherError::Custom(format!(
            "获取 Minecraft 个人资料失败 ({}): {}",
            status, body
        )));
    }

    let data: MinecraftProfileResponse = resp
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析 Minecraft 个人资料失败: {}", e)))?;

    Ok((data.name, data.id))
}

pub async fn complete_microsoft_auth(
    microsoft_access_token: &str,
    microsoft_refresh_token: &str,
    expires_in: u64,
) -> Result<MicrosoftAuthResult, LauncherError> {
    let xbox_live_token = authenticate_xbox_live(microsoft_access_token).await?;
    let (xsts_token, uhs) = authenticate_xsts(&xbox_live_token).await?;
    let mc_access_token = authenticate_minecraft(&xsts_token, &uhs).await?;
    let (mc_username, mc_uuid) = get_minecraft_profile(&mc_access_token).await?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    Ok(MicrosoftAuthResult {
        username: mc_username,
        uuid: mc_uuid,
        access_token: mc_access_token,
        refresh_token: microsoft_refresh_token.to_string(),
        expires_at: now + expires_in as i64,
    })
}

pub async fn refresh_and_authenticate(
    refresh_token: &str,
) -> Result<MicrosoftAuthResult, LauncherError> {
    let token_resp = refresh_microsoft_token(refresh_token).await?;
    let new_refresh = token_resp
        .refresh_token
        .as_deref()
        .unwrap_or(refresh_token);
    let expires_in = token_resp.expires_in.unwrap_or(3600);

    complete_microsoft_auth(&token_resp.access_token, new_refresh, expires_in).await
}
