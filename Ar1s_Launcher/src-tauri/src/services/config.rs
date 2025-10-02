use std::fs;
use std::path::PathBuf;
use sysinfo::System;
use tauri::Emitter;

use crate::errors::LauncherError;
use crate::models::{GameConfig, GameDirInfo};

/// 加载配置文件
pub fn load_config() -> Result<GameConfig, LauncherError> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let config: GameConfig = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        // 获取可执行文件路径并确保其存在
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| LauncherError::Custom("无法获取可执行文件目录".to_string()))?;
        
        // 创建路径变量并确保所有权
        let mc_dir = exe_dir.join(".minecraft");
        let mc_dir_str = mc_dir.to_string_lossy().into_owned();
        
        // 创建目录结构
        if !mc_dir.exists() {
            fs::create_dir_all(&mc_dir)?;
            // 创建必要的子目录
            let sub_dirs = ["versions", "libraries", "assets", "saves", "resourcepacks", "logs"];
            for dir in sub_dirs {
                fs::create_dir_all(mc_dir.join(dir))?;
            }
        }

        // 创建并返回配置
        let config = GameConfig {
            game_dir: mc_dir_str,
            version_isolation: true,
            java_path: None,
            download_threads: 8,
            language: Some("zh_cn".to_string()),
            isolate_saves: true,
            isolate_resourcepacks: true,
            isolate_logs: true,
            username: None,
            uuid: None,
            max_memory: crate::models::default_max_memory(),
        };
        
        // 保存配置
        save_config(&config)?;

        Ok(config)
    }
}

/// 保存配置文件
pub fn save_config(config: &GameConfig) -> Result<(), LauncherError> {
    let config_path = get_config_path()?;
    fs::write(config_path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

/// 获取配置文件路径
fn get_config_path() -> Result<PathBuf, LauncherError> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent()
        .ok_or_else(|| LauncherError::Custom("无法获取可执行文件目录".to_string()))?;
    
    Ok(exe_dir.join("ar1s.json"))
}

pub async fn load_config_key(key: String) -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    match key.as_str() {
        "javaPath" => Ok(config.java_path),
        "gameDir" => Ok(Some(config.game_dir)),
        "versionIsolation" => Ok(Some(config.version_isolation.to_string())),
        "downloadThreads" => Ok(Some(config.download_threads.to_string())),
        "language" => Ok(config.language),
        "isolateSaves" => Ok(Some(config.isolate_saves.to_string())),
        "isolateResourcepacks" => Ok(Some(config.isolate_resourcepacks.to_string())),
        "isolateLogs" => Ok(Some(config.isolate_logs.to_string())),
        "username" => Ok(config.username),
        "uuid" => Ok(config.uuid),
        "maxMemory" => Ok(Some(config.max_memory.to_string())),
        _ => Err(LauncherError::Custom(format!("Unknown config key: {}", key))),
    }
}

pub async fn save_config_key(key: String, value: String) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    match key.as_str() {
        "javaPath" => config.java_path = Some(value),
        "gameDir" => config.game_dir = value,
        "versionIsolation" => config.version_isolation = value.parse().map_err(|_| LauncherError::Custom("Invalid boolean value for versionIsolation".to_string()))?,
        "downloadThreads" => config.download_threads = value.parse().map_err(|_| LauncherError::Custom("Invalid u8 value for downloadThreads".to_string()))?,
        "language" => config.language = Some(value),
        "isolateSaves" => config.isolate_saves = value.parse().map_err(|_| LauncherError::Custom("Invalid boolean value for isolateSaves".to_string()))?,
        "isolateResourcepacks" => config.isolate_resourcepacks = value.parse().map_err(|_| LauncherError::Custom("Invalid boolean value for isolateResourcepacks".to_string()))?,
        "isolateLogs" => config.isolate_logs = value.parse().map_err(|_| LauncherError::Custom("Invalid boolean value for isolateLogs".to_string()))?,
        // Removed username and uuid to avoid duplication with auth service
        "maxMemory" => config.max_memory = value.parse().map_err(|_| LauncherError::Custom("Invalid u32 value for maxMemory".to_string()))?,
        _ => return Err(LauncherError::Custom(format!("Unknown or restricted config key: {}", key))),
    }
    save_config(&config)?;
    Ok(())
}

pub fn get_game_dir() -> Result<String, LauncherError> {
    let config = load_config()?;
    Ok(config.game_dir)
}

pub async fn get_game_dir_info() -> Result<GameDirInfo, LauncherError> {
    let game_dir_str = get_game_dir()?;
    let versions_dir = PathBuf::from(&game_dir_str).join("versions");
    let mut versions = Vec::new();

    if versions_dir.is_dir() {
        for entry in fs::read_dir(versions_dir)? {
            if let Ok(entry) = entry {
                if entry.file_type()?.is_dir() {
                    let version_id = entry.file_name().to_string_lossy().into_owned();
                    let version_json_path = entry.path().join(format!("{}.json", version_id));
                    if version_json_path.exists() {
                        versions.push(version_id);
                    }
                }
            }
        }
    }

    Ok(GameDirInfo {
        path: game_dir_str,
        versions,
        total_size: 0, // Note: total_size is not calculated
    })
}

pub async fn set_game_dir(path: String, window: &tauri::Window) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.game_dir = path.clone();
    save_config(&config)?;

    window.emit("game-dir-changed", &path)?;
    Ok(())
}

pub async fn set_version_isolation(enabled: bool) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.version_isolation = enabled;
    save_config(&config)?;
    Ok(())
}

pub fn get_download_threads() -> Result<u8, LauncherError> {
    let config = load_config()?;
    Ok(config.download_threads)
}

pub async fn set_download_threads(threads: u8) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.download_threads = threads;
    save_config(&config)?;
    Ok(())
}

pub async fn validate_version_files(version_id: String) -> Result<Vec<String>, LauncherError> {
    let config = load_config()?;
    let game_dir = PathBuf::from(&config.game_dir);
    let version_dir = game_dir.join("versions").join(&version_id);
    let version_json_path = version_dir.join(format!("{}.json", &version_id));

    let mut missing_files = Vec::new();

    if !version_json_path.exists() {
        missing_files.push(format!("版本JSON文件不存在: {}", version_json_path.display()));
        return Ok(missing_files);
    }

    let version_json_str = fs::read_to_string(&version_json_path)?;
    let version_json: serde_json::Value = serde_json::from_str(&version_json_str)?;

    let libraries_base_dir = game_dir.join("libraries");

    let main_game_jar_path = version_dir.join(format!("{}.jar", &version_id));
    if !main_game_jar_path.exists() {
        missing_files.push(format!("主游戏JAR文件不存在: {}", main_game_jar_path.display()));
    }

    if let Some(libraries) = version_json["libraries"].as_array() {
        for lib in libraries {
            if let Some(natives) = lib.get("natives") {
                if let Some(os_classifier) = natives.get(std::env::consts::OS) {
                    if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("classifiers")).and_then(|c| c.get(os_classifier.as_str().unwrap())) {
                        let lib_path = libraries_base_dir.join(artifact["path"].as_str().unwrap());
                        if !lib_path.exists() {
                            missing_files.push(format!("Natives库文件不存在: {}", lib_path.display()));
                        }
                    }
                }
            } else {
                if let Some(rules) = lib.get("rules").and_then(|r| r.as_array()) {
                    let mut allowed = true;
                    for rule in rules {
                        if let Some(os) = rule.get("os") {
                            if let Some(name) = os["name"].as_str() {
                                if name == std::env::consts::OS {
                                    allowed = rule["action"].as_str() == Some("allow");
                                } else {
                                    allowed = rule["action"].as_str() != Some("allow");
                                }
                            }
                        }
                    }
                    if !allowed { continue; }
                }
                if let Some(path) = lib.get("downloads").and_then(|d| d.get("artifact")).and_then(|a| a.get("path")).and_then(|p| p.as_str()) {
                    let lib_path = libraries_base_dir.join(path);
                    if !lib_path.exists() {
                        missing_files.push(format!("库文件不存在: {}", lib_path.display()));
                    }
                }
            }
        }
    }

    Ok(missing_files)
}

pub fn get_total_memory() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.total_memory()
}
