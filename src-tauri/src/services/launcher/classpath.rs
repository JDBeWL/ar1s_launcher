//! Classpath 构建和库预检逻辑

use crate::errors::LauncherError;
use std::path::{Path, PathBuf};

/// 通用库文件查找函数
/// 递归扫描指定目录，查找匹配指定模式的JAR文件
pub fn find_library_jar(dir: &Path, patterns: &[&str]) -> Option<PathBuf> {
    let read_dir = std::fs::read_dir(dir).ok()?;

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_library_jar(&path, patterns) {
                return Some(found);
            }
        } else {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let full_path = path.to_string_lossy().to_lowercase();

            for pattern in patterns {
                if (name.contains(pattern) || full_path.contains(pattern)) && name.ends_with(".jar")
                {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// 预检并修复缺失的库
/// 如果找到或已存在返回 true, 否则返回 false
pub fn precheck_and_heal_library(
    classpath: &mut Vec<PathBuf>,
    libraries_base_dir: &Path,
    library_name: &str,
    classpath_patterns: &[&str],
    search_patterns: &[&str],
    emit: &impl Fn(&str, String),
) -> bool {
    let is_missing = !classpath.iter().any(|p| {
        let s = p.to_string_lossy().to_lowercase();
        classpath_patterns.iter().any(|pat| s.contains(pat))
    });

    if !is_missing {
        return true; // 库已存在
    }

    emit(
        "log-debug",
        format!(
            "预检：Classpath 未包含 {}，尝试在 libraries 目录自动查找",
            library_name
        ),
    );

    if let Some(jar) = find_library_jar(libraries_base_dir, search_patterns) {
        emit(
            "log-debug",
            format!(
                "自动自愈：发现 {} 库，加入 Classpath: {}",
                library_name,
                jar.display()
            ),
        );
        classpath.push(jar);
        true
    } else {
        emit(
            "log-error",
            format!("预检失败：在 libraries 中未找到 {} 库。", library_name),
        );
        false
    }
}

/// 构建 Classpath
pub fn build_classpath(
    version_json: &serde_json::Value,
    libraries_base_dir: &Path,
    version_dir: &Path,
    version: &str,
    current_os: &str,
    emit: &impl Fn(&str, String),
) -> Result<Vec<PathBuf>, LauncherError> {
    let mut classpath = vec![];

    if let Some(libraries) = version_json["libraries"].as_array() {
        for lib in libraries {
            // 跳过 Natives 库
            if lib.get("natives").is_some() {
                continue;
            }

            if !should_include_library(lib, current_os) {
                continue;
            }

            if let Some(lib_path) = resolve_library_path(lib, libraries_base_dir, emit) {
                classpath.push(lib_path);
            }
        }
    }

    // 添加主游戏 JAR
    let main_game_jar_path = version_dir.join(format!("{}.jar", version));
    emit(
        "log-debug",
        format!("主游戏JAR路径: {}", main_game_jar_path.display()),
    );

    if !main_game_jar_path.exists() {
        emit(
            "log-error",
            format!("主游戏JAR文件不存在: {}", main_game_jar_path.display()),
        );
        return Err(LauncherError::Custom(format!(
            "主游戏JAR文件不存在: {}",
            main_game_jar_path.display()
        )));
    }

    classpath.push(main_game_jar_path);
    Ok(classpath)
}

/// 检查库是否应该包含在当前操作系统
fn should_include_library(lib: &serde_json::Value, current_os: &str) -> bool {
    let Some(rules) = lib.get("rules").and_then(|r| r.as_array()) else {
        return true;
    };

    let mut allowed = true;
    for rule in rules {
        if let Some(os) = rule.get("os") {
            if let Some(name) = os["name"].as_str() {
                if name == current_os {
                    allowed = rule["action"].as_str() == Some("allow");
                } else {
                    allowed = rule["action"].as_str() != Some("allow");
                }
            }
        }
    }
    allowed
}

/// 解析库文件路径
fn resolve_library_path(
    lib: &serde_json::Value,
    libraries_base_dir: &Path,
    emit: &impl Fn(&str, String),
) -> Option<PathBuf> {
    // 优先使用 downloads.artifact.path
    if let Some(path) = lib["downloads"]
        .get("artifact")
        .and_then(|a| a.get("path"))
        .and_then(|p| p.as_str())
    {
        let lib_path = libraries_base_dir.join(path);
        emit(
            "log-debug",
            format!("添加到Classpath的库: {}", lib_path.display()),
        );

        if !lib_path.exists() {
            emit(
                "log-error",
                format!("Classpath中的库文件不存在: {}", lib_path.display()),
            );
            return None;
        }
        return Some(lib_path);
    }

    // 回退：根据 maven 坐标构建本地路径
    let name = lib.get("name").and_then(|n| n.as_str())?;
    let parts: Vec<&str> = name.split(':').collect();

    if parts.len() < 3 {
        emit(
            "log-error",
            format!(
                "库条目缺少 downloads.artifact.path，且 name 非法: {:?}",
                lib
            ),
        );
        return None;
    }

    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let candidate = libraries_base_dir
        .join(&group)
        .join(artifact)
        .join(version)
        .join(format!("{}-{}.jar", artifact, version));

    emit(
        "log-debug",
        format!("尝试回退解析库路径: {}", candidate.display()),
    );

    if candidate.exists() {
        Some(candidate)
    } else {
        emit(
            "log-error",
            format!(
                "库文件缺失（maven 回退也未找到）: name={}，期望路径: {}",
                name,
                candidate.display()
            ),
        );
        None
    }
}

/// 执行 LaunchWrapper 相关的库预检
pub fn precheck_launchwrapper_libraries(
    classpath: &mut Vec<PathBuf>,
    libraries_base_dir: &Path,
    emit: &impl Fn(&str, String),
) -> Result<(), LauncherError> {
    // 预检 LaunchWrapper
    if !precheck_and_heal_library(
        classpath,
        libraries_base_dir,
        "LaunchWrapper",
        &["net/minecraft/launchwrapper", "launchwrapper-"],
        &["launchwrapper", "net/minecraft/launchwrapper"],
        emit,
    ) {
        let error_msg = "预检失败：缺少 LaunchWrapper 库。请重新运行 Forge 安装或手动补齐 libraries/net/minecraft/launchwrapper/* 并在版本 JSON 的 libraries 中声明 net.minecraft:launchwrapper:1.12（且包含 downloads.artifact.path）".to_string();
        emit("log-error", error_msg.clone());
        return Err(LauncherError::Custom(error_msg));
    }

    // 预检其他依赖库（不强制要求）
    precheck_and_heal_library(
        classpath,
        libraries_base_dir,
        "jopt-simple",
        &["jopt-simple", "joptsimple"],
        &["jopt-simple", "joptsimple"],
        emit,
    );

    precheck_and_heal_library(
        classpath,
        libraries_base_dir,
        "Forge/FML",
        &["minecraftforge", "forge-", "/fml/", "\\fml\\"],
        &["forge", "minecraftforge", "net/minecraftforge/forge"],
        emit,
    );

    precheck_and_heal_library(
        classpath,
        libraries_base_dir,
        "ASM",
        &["asm", "org/objectweb/asm", "asm-all"],
        &["asm", "org/objectweb/asm", "asm-all"],
        emit,
    );

    precheck_and_heal_library(
        classpath,
        libraries_base_dir,
        "LZMA",
        &["lzma", "xz", "org/tukaani", "lzma-sdk"],
        &["xz", "lzma", "org/tukaani", "lzma-sdk"],
        emit,
    );

    Ok(())
}
