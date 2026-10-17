//! Java 解析与玩家 UUID 模块
//!
//! 相对路径解析、java -version 探测、MC 版本到推荐 Java 范围的映射与多安装自动挑选

use crate::errors::LauncherError;
use crate::models::GameConfig;
use crate::services::config::load_config;
use std::path::PathBuf;
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

/// 所需最低 Java 版本，即 recommended_java_version_range 的下界
pub fn required_java_version(mc_version: &str) -> u32 {
    recommended_java_version_range(mc_version).0
}

use crate::utils::minecraft::parse_mc_version;

/// 推荐 Java 版本范围 (min, max)；范围外的 Java 也会用，但带警告
/// 下界依据：≤1.16.5 用 Java 17+ 启动 Forge 会报错，1.17~1.20.4 用 Java 21+ 有兼容风险，1.20.5+ 用 Java 24+ 有 mod/库兼容风险，26.x 需 Java 25+
pub fn recommended_java_version_range(mc_version: &str) -> (u32, u32) {
    if let Some((major, minor)) = parse_mc_version(mc_version) {
        if major >= 26 {
            return (25, 99);
        }
        if major > 1 || (major == 1 && minor > 20) {
            // parse_mc_version 不返回 patch，1.20.5 只能靠 minor 判定
            return (21, 23);
        }
        if major == 1 && minor == 20 {
            let parts: Vec<&str> = mc_version.split('.').collect();
            if parts.len() >= 3 {
                if let Ok(patch) = parts[2].split('-').next().unwrap_or("0").parse::<u32>() {
                    if patch >= 5 {
                        return (21, 23);
                    }
                }
            }
            return (17, 17);
        }
        if major == 1 && minor >= 17 {
            return (17, 17);
        }
        if major == 1 && minor <= 7 {
            // 某些极旧模组可能只能用 Java 7，故下界放到 7
            return (7, 8);
        }
        // 1.8 ~ 1.16.5
        return (8, 8);
    }

    // 无法解析时按多数现代版本取 17
    (17, 17)
}

/// 把实例 ID 解析为真实 MC 版本，提取顺序：版本 ID 本身 → --fml.mcVersion → patches game 版本 → inheritsFrom 递归
pub fn resolve_actual_mc_version(version_id: &str, game_dir: &str) -> String {
    if parse_mc_version(version_id).is_some() {
        return version_id.to_string();
    }

    let version_dir = std::path::PathBuf::from(game_dir)
        .join("versions")
        .join(version_id);
    let json_path = version_dir.join(format!("{}.json", version_id));

    if !json_path.exists() {
        return version_id.to_string();
    }

    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(mc_ver) = extract_fml_mc_version(&json) {
                return mc_ver;
            }

            if let Some(mc_ver) = extract_patches_game_version(&json) {
                return mc_ver;
            }

            if let Some(inherits) = json["inheritsFrom"].as_str() {
                if parse_mc_version(inherits).is_some() {
                    return inherits.to_string();
                }
                return resolve_actual_mc_version_recursive(inherits, game_dir, 0);
            }
        }
    }

    version_id.to_string()
}

fn extract_fml_mc_version(json: &serde_json::Value) -> Option<String> {
    let args = json
        .get("arguments")
        .and_then(|a| a.get("game"))
        .and_then(|g| g.as_array())?;

    for (i, arg) in args.iter().enumerate() {
        if arg.as_str() == Some("--fml.mcVersion") {
            if let Some(next) = args.get(i + 1).and_then(|v| v.as_str()) {
                return Some(next.to_string());
            }
        }
    }
    None
}

fn extract_patches_game_version(json: &serde_json::Value) -> Option<String> {
    let patches = json["patches"].as_array()?;

    for patch in patches {
        if patch["id"].as_str() == Some("game") {
            return patch["version"].as_str().map(String::from);
        }
    }
    None
}

fn resolve_actual_mc_version_recursive(version_id: &str, game_dir: &str, depth: u32) -> String {
    if depth > 5 {
        return version_id.to_string();
    }

    if parse_mc_version(version_id).is_some() {
        return version_id.to_string();
    }

    let version_dir = std::path::PathBuf::from(game_dir)
        .join("versions")
        .join(version_id);
    let json_path = version_dir.join(format!("{}.json", version_id));

    if !json_path.exists() {
        return version_id.to_string();
    }

    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(inherits) = json["inheritsFrom"].as_str() {
                return resolve_actual_mc_version_recursive(inherits, game_dir, depth + 1);
            }
        }
    }

    version_id.to_string()
}

#[derive(Debug, Clone)]
pub struct JavaMatchResult {
    pub path: String,
    pub version: u32,
    pub is_optimal: bool, // 是否在推荐版本范围内
    pub warning: Option<String>,
}

/// 选取策略：优先推荐范围内最高版本（兼容性最好），否则取满足下界的最低版本
pub fn auto_match_java_for_version(mc_version: &str) -> Option<JavaMatchResult> {
    let (min_ver, max_ver) = recommended_java_version_range(mc_version);
    log::info!(
        "Minecraft {} 推荐 Java 版本范围: {} ~ {}",
        mc_version,
        min_ver,
        max_ver
    );

    let config = load_config().ok()?;
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

    if candidates.is_empty() {
        log::warn!("未找到任何可用的 Java 安装");
        return None;
    }

    let optimal = candidates
        .iter()
        .filter(|(_, ver)| *ver >= min_ver && *ver <= max_ver)
        .max_by_key(|(_, ver)| *ver);

    if let Some((path, ver)) = optimal {
        log::info!("自动匹配 Java: {} (版本 {}, 在推荐范围内)", path, ver);
        return Some(JavaMatchResult {
            path: path.clone(),
            version: *ver,
            is_optimal: true,
            warning: None,
        });
    }

    // 范围外优先取满足下界的最低版本，避免版本过高
    let fallback = candidates
        .iter()
        .filter(|(_, ver)| *ver >= min_ver)
        .min_by_key(|(_, ver)| *ver)
        .or_else(|| candidates.iter().max_by_key(|(_, ver)| *ver));

    fallback.map(|(path, ver)| {
        let warning = if *ver > max_ver {
            format!(
                "当前使用 Java {} 启动 Minecraft {}，推荐 Java {}~{} 以获得最佳兼容性。如遇到异常可尝试切换 Java 版本。",
                ver, mc_version, min_ver, max_ver
            )
        } else {
            format!(
                "未找到推荐的 Java 版本范围 ({}~{}) 内的安装，当前使用 Java {}。如遇到异常可尝试安装推荐版本的 Java。",
                min_ver, max_ver, ver
            )
        };
        log::warn!("{}", warning);
        JavaMatchResult {
            path: path.clone(),
            version: *ver,
            is_optimal: false,
            warning: Some(warning),
        }
    })
}
