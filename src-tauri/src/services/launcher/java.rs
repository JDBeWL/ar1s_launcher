//! Java 解析与玩家 UUID 模块
//!
//! 相对路径解析、java -version 探测、候选装配与按判据自动挑选

use super::java_requirement::{self, JavaRequirement};
use crate::errors::LauncherError;
use crate::models::GameConfig;
use crate::services::config::load_config;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn generate_offline_uuid(username: &str) -> String {
    if let Ok(config) = load_config() {
        if let (Some(saved_username), Some(saved_uuid)) = (&config.username, &config.uuid) {
            if saved_username == username {
                return saved_uuid.clone();
            }
        }
    }

    Uuid::new_v3(
        &Uuid::NAMESPACE_DNS,
        format!("OfflinePlayer:{}", username).as_bytes(),
    )
    .to_string()
}

/// 将相对路径解析为绝对路径（相对于可执行文件所在目录）
pub fn resolve_path(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        return p;
    }

    if let Ok(exe_dir) = std::env::current_exe()
        .map_err(|e| format!("无法获取可执行文件路径: {}", e))
        .and_then(|exe| {
            exe.parent()
                .map(|p| p.to_path_buf())
                .ok_or("无法获取目录".to_string())
        })
    {
        return exe_dir.join(&p);
    }

    p
}

/// 解析 Java 可执行文件路径（支持相对路径）
pub fn resolve_java_path(config: &GameConfig) -> Result<String, LauncherError> {
    if let Some(config_path) = &config.java_path {
        if !config_path.is_empty() {
            let resolved = resolve_path(config_path);
            if resolved.exists() {
                return Ok(resolved.to_string_lossy().into_owned());
            }
            if PathBuf::from(config_path).is_relative() {
                return Err(LauncherError::Custom(format!(
                    "Java 相对路径 '{}' 解析后不存在: {}",
                    config_path,
                    resolved.display()
                )));
            }
        }
    }

    if Command::new("java").arg("-version").output().is_ok() {
        Ok("java".to_string())
    } else {
        Err(LauncherError::Custom(
            "未在配置中找到有效的Java路径，且系统PATH中也未找到Java。".to_string(),
        ))
    }
}

pub fn resolve_game_dir(game_dir: &str) -> PathBuf {
    resolve_path(game_dir)
}

/// 返回 Java 主版本号；Java 8 的 "1.8.0_x" 版本串按 8 计
pub fn detect_java_version(java_path: &str) -> Option<u32> {
    let resolved = resolve_path(java_path);
    let actual_path = if resolved.exists() {
        resolved.to_string_lossy().into_owned()
    } else {
        java_path.to_string()
    };

    let mut command = Command::new(&actual_path);
    command.arg("-version");

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command.output().ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    for line in stderr.lines() {
        if line.contains("version") {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    let version_str = &line[start + 1..start + 1 + end];
                    let parts: Vec<&str> = version_str.split('.').collect();
                    if parts.is_empty() {
                        continue;
                    }
                    if let Ok(major) = parts[0].parse::<u32>() {
                        return if major == 1 && parts.len() > 1 {
                            parts[1].parse::<u32>().ok()
                        } else {
                            Some(major)
                        };
                    }
                }
            }
        }
    }

    None
}

/// 没有已知上限时的哨兵值，沿用本仓库既有的写法
pub const NO_UPPER_BOUND: u32 = 99;

/// 实例的 Java 判据：读版本 JSON 声明 + mods 实测，不再靠版本表猜
pub fn profile_for(version_id: &str, game_dir: &Path) -> java_requirement::InstanceProfile {
    java_requirement::analyze(version_id, game_dir)
}

#[derive(Debug, Clone)]
pub struct JavaMatchResult {
    pub path: String,
    pub version: u32,
    pub is_optimal: bool, // 是否在推荐版本范围内
    pub warning: Option<String>,
}

/// 候选 = 配置指定 + 手动添加的路径 + 全盘扫描结果，附带各自的 Java 大版本
pub fn collect_candidates(config: &GameConfig) -> Vec<(String, u32)> {
    let mut candidates: Vec<(String, u32)> = Vec::new();

    let mut try_add = |path: &str| {
        if let Some(ver) = detect_java_version(path) {
            if !candidates.iter().any(|(p, _)| p == path) {
                candidates.push((path.to_string(), ver));
            }
        }
    };

    if let Some(ref java_path) = config.java_path {
        try_add(java_path);
    }

    for path in &config.custom_java_paths {
        try_add(path);
    }

    if let Ok(installations) = crate::services::java::find_java_installations_command_sync() {
        for path in &installations {
            try_add(path);
        }
    }

    candidates
}

/// 选取策略：正好等于声明版本最好，否则在允许区间里取最低的；一个都不允许则 None
pub fn choose_java(req: &JavaRequirement, candidates: &[(String, u32)]) -> Option<JavaMatchResult> {
    let allowed: Vec<&(String, u32)> = candidates
        .iter()
        .filter(|c| java_requirement::java_allowed(req, c.1))
        .collect();

    let chosen = allowed
        .iter()
        .copied()
        .find(|c| c.1 == req.target)
        .or_else(|| allowed.iter().copied().min_by_key(|c| c.1))?;

    let is_optimal = chosen.1 == req.target;
    let warning = (!is_optimal).then(|| {
        let band = match req.hard_max {
            Some(max) => format!("{}~{}", req.min, max),
            None => format!("{}+", req.min),
        };
        let text = format!(
            "本机没有该实例声明的 Java {}，改用 Java {}（允许区间 {}）",
            req.target, chosen.1, band
        );
        log::warn!("{}", text);
        text
    });

    Some(JavaMatchResult {
        path: chosen.0.clone(),
        version: chosen.1,
        is_optimal,
        warning,
    })
}

/// 装配候选并按实例判据挑选
pub fn auto_match_java(profile: &java_requirement::InstanceProfile) -> Option<JavaMatchResult> {
    let config = load_config().ok()?;
    let candidates = collect_candidates(&config);

    if candidates.is_empty() {
        log::warn!("未找到任何可用的 Java 安装");
        return None;
    }

    choose_java(&profile.requirement, &candidates)
}
