//! Microsoft 凭据存储
//!
//! token 单独存放在 credentials.json，不写进 ar1s.json；Unix 下权限设为 0600，
//! 便携模式下文件跟着启动器目录走

use crate::errors::LauncherError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

impl Credentials {
    pub fn is_empty(&self) -> bool {
        self.access_token.is_none() && self.refresh_token.is_none()
    }
}

fn credentials_path() -> PathBuf {
    crate::services::config::credentials_path()
}

pub fn load() -> Result<Option<Credentials>, LauncherError> {
    load_from_path(&credentials_path())
}

pub fn save(credentials: &Credentials) -> Result<(), LauncherError> {
    save_to_path(&credentials_path(), credentials)
}

pub fn clear() -> Result<(), LauncherError> {
    clear_path(&credentials_path())
}

fn load_from_path(path: &Path) -> Result<Option<Credentials>, LauncherError> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    let credentials = serde_json::from_str(&content)
        .map_err(|e| LauncherError::Custom(format!("凭据文件损坏: {e}")))?;
    Ok(Some(credentials))
}

fn save_to_path(path: &Path, credentials: &Credentials) -> Result<(), LauncherError> {
    if credentials.is_empty() {
        return clear_path(path);
    }

    let dir = path
        .parent()
        .ok_or_else(|| LauncherError::Custom("凭据文件没有父目录".to_string()))?;
    fs::create_dir_all(dir)?;
    let json = serde_json::to_string_pretty(credentials)?;

    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(json.as_bytes())?;
    tmp.flush()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }

    tmp.persist(path).map_err(|e| e.error)?;
    restrict_to_current_user(path);
    Ok(())
}

/// Windows 没有 unix 的 0600：落盘后显式收紧 DACL，只保留当前用户、SYSTEM 与
/// 管理员。这是加固而不是前置条件，失败只记日志，不能让登录/刷新流程失败。
#[cfg(windows)]
fn restrict_to_current_user(path: &Path) {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let user = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
        (Ok(domain), Ok(name)) if !domain.is_empty() && !name.is_empty() => {
            format!("{}\\{}", domain, name)
        }
        // 域/用户名取不到时不做任何改动，保持原有继承权限
        _ => return,
    };

    let mut command = Command::new("icacls");
    command
        .arg(path)
        .arg("/inheritance:r")
        .creation_flags(CREATE_NO_WINDOW);
    // 用户按名字给，SYSTEM 与 Administrators 用 SID，避免系统语言不同导致名称解析失败
    for grant in [
        format!("{}:F", user),
        "*S-1-5-18:F".to_string(),
        "*S-1-5-32-544:F".to_string(),
    ] {
        command.arg("/grant:r").arg(grant);
    }

    match command.output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("收紧凭据文件权限失败: {}", stderr.trim());
        }
        Err(error) => log::warn!("无法调用 icacls 收紧凭据文件权限: {}", error),
    }
}

#[cfg(not(windows))]
fn restrict_to_current_user(_path: &Path) {}

fn clear_path(path: &Path) -> Result<(), LauncherError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(LauncherError::Io(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::{Credentials, clear_path, load_from_path, save_to_path};
    use std::fs;

    fn sample_credentials() -> Credentials {
        Credentials {
            access_token: Some("access".to_string()),
            refresh_token: Some("refresh".to_string()),
            expires_at: Some(123),
        }
    }

    #[test]
    fn test_credentials_json_roundtrip() {
        let credentials = sample_credentials();

        let json = serde_json::to_string(&credentials).unwrap();
        let restored: Credentials = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, credentials);
        assert!(!restored.is_empty());
        assert!(Credentials::default().is_empty());
    }

    #[test]
    fn test_credentials_file_roundtrip_and_clear() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("credentials.json");
        let credentials = sample_credentials();

        assert_eq!(load_from_path(&path).unwrap(), None);

        save_to_path(&path, &credentials).unwrap();
        assert_eq!(load_from_path(&path).unwrap(), Some(credentials));

        save_to_path(&path, &Credentials::default()).unwrap();
        assert!(!path.exists());
        assert_eq!(load_from_path(&path).unwrap(), None);
    }

    #[test]
    fn test_credentials_file_corruption_is_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("credentials.json");
        fs::write(&path, b"not json").unwrap();

        assert!(load_from_path(&path).is_err());
    }

    #[test]
    fn test_clear_path_ignores_missing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        clear_path(&dir.path().join("credentials.json")).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_credentials_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("credentials.json");
        save_to_path(&path, &sample_credentials()).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
