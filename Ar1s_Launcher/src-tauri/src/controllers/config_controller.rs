use std::fs;
use std::path::PathBuf;
use tauri::Emitter;
use sysinfo::System;

use crate::errors::LauncherError;
use crate::models::*;
use crate::services::config::{load_config, save_config};

#[tauri::command(rename = "get_config")]
pub async fn get_config() -> Result<GameConfig, LauncherError> {
    load_config()
}

#[tauri::command]
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

#[tauri::command]
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
        "username" => config.username = Some(value),
        "uuid" => config.uuid = Some(value),
        "maxMemory" => config.max_memory = value.parse().map_err(|_| LauncherError::Custom("Invalid u32 value for maxMemory".to_string()))?,
        _ => return Err(LauncherError::Custom(format!("Unknown config key: {}", key))),
    }
    save_config(&config)?;
    Ok(())
}

#[tauri::command]
pub fn get_game_dir() -> Result<String, LauncherError> {
    let config = load_config()?;
    Ok(config.game_dir)
}

#[tauri::command]
pub async fn select_game_dir(_window: tauri::Window) -> Result<String, LauncherError> {
    // 现在由前端直接处理对话框选择
    get_game_dir()
}

#[tauri::command]
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
        total_size: 0,
    })
}

#[tauri::command]
pub async fn set_game_dir(path: String, window: tauri::Window) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.game_dir = path.clone();
    save_config(&config)?;

    window.emit("game-dir-changed", &path)?;
    Ok(())
}

#[tauri::command]
pub async fn set_version_isolation(enabled: bool) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.version_isolation = enabled;
    save_config(&config)?;
    Ok(())
}

#[tauri::command]
pub fn get_download_threads() -> Result<u8, LauncherError> {
    let config = load_config()?;
    Ok(config.download_threads)
}

#[tauri::command]
pub async fn set_download_threads(threads: u8) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.download_threads = threads;
    save_config(&config)?;
    Ok(())
}

#[tauri::command]
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

    let (libraries_base_dir, _assets_base_dir) = (game_dir.join("libraries"), game_dir.join("assets"));

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
                if let Some(path) = lib["downloads"]["artifact"]["path"].as_str() {
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

#[tauri::command]
pub fn get_total_memory() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.total_memory()
}