//! Java 路径解析和 UUID 生成

use crate::errors::LauncherError;
use crate::models::GameConfig;
use crate::services::config::load_config;
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

/// 生成离线模式 UUID
pub fn generate_offline_uuid(username: &str) -> String {
    // 首先检查配置中是否已有保存的 UUID
    if let Ok(config) = load_config() {
        // 如果用户名匹配且已有 UUID，则直接返回保存的 UUID
        if let (Some(saved_username), Some(saved_uuid)) = (&config.username, &config.uuid) {
            if saved_username == username {
                return saved_uuid.clone();
            }
        }
    }

    // 离线模式：UUID v3 (MD5) 基于 "OfflinePlayer:{username}"
    Uuid::new_v3(
        &Uuid::NAMESPACE_DNS,
        format!("OfflinePlayer:{}", username).as_bytes(),
    )
    .to_string()
}

/// 解析 Java 可执行文件路径
pub fn resolve_java_path(config: &GameConfig) -> Result<String, LauncherError> {
    // 1. 首先尝试使用配置中的 Java 路径
    if let Some(config_path) = &config.java_path {
        if !config_path.is_empty() && PathBuf::from(config_path).exists() {
            return Ok(config_path.clone());
        }
    }

    // 2. 如果未配置或配置路径不存在，尝试在 PATH 中查找
    if Command::new("java").arg("-version").output().is_ok() {
        Ok("java".to_string())
    } else {
        Err(LauncherError::Custom(
            "未在配置中找到有效的Java路径，且系统PATH中也未找到Java。".to_string(),
        ))
    }
}
