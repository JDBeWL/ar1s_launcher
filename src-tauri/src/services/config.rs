use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::System;
use tauri::Emitter;

use crate::errors::LauncherError;
use crate::models::{GameConfig, GameDirInfo};
use crate::services::memory::{
    auto_set_memory_if_enabled, get_memory_warning_message, get_system_memory,
    is_memory_setting_safe, recommend_memory_for_game, AutoMemoryConfig, MemoryRecommendation,
    MemoryStats,
};

// 配置缓存
static CONFIG_CACHE: std::sync::LazyLock<RwLock<Option<GameConfig>>> = 
    std::sync::LazyLock::new(|| RwLock::new(None));

// 标记配置是否已预加载
static CONFIG_PRELOADED: AtomicBool = AtomicBool::new(false);

/// 预加载配置（应在应用启动时调用）
/// 这会立即加载配置到缓存，避免后续的锁竞争
pub fn preload_config() -> Result<(), LauncherError> {
    if CONFIG_PRELOADED.load(Ordering::Relaxed) {
        return Ok(());
    }
    
    log::info!("预加载配置文件...");
    let config = load_config_internal()?;
    
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = Some(config);
    }
    
    CONFIG_PRELOADED.store(true, Ordering::Relaxed);
    log::info!("配置文件预加载完成");
    Ok(())
}

/// 清除配置缓存（供外部模块在需要时调用）
pub fn invalidate_config_cache() {
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = None;
    }
    CONFIG_PRELOADED.store(false, Ordering::Relaxed);
    log::debug!("配置缓存已清除");
}

// 获取保存的用户名
pub async fn get_saved_username() -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    Ok(config.username)
}

// 设置保存的用户名
pub async fn set_saved_username(username: String) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.username = Some(username);
    save_config(&config)?;
    Ok(())
}

// 获取保存的UUID
pub async fn get_saved_uuid() -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    Ok(config.uuid)
}

// 设置保存的UUID
pub async fn set_saved_uuid(uuid: String) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.uuid = Some(uuid);
    save_config(&config)?;
    Ok(())
}

/// 加载配置文件（带缓存，优化版本）
pub fn load_config() -> Result<GameConfig, LauncherError> {
    // 快速路径：先尝试读取缓存（使用读锁，允许并发读取）
    if let Ok(cache) = CONFIG_CACHE.read() {
        if let Some(ref config) = *cache {
            return Ok(config.clone());
        }
    }

    // 缓存未命中，加载配置
    let config = load_config_internal()?;
    
    // 更新缓存
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = Some(config.clone());
    }
    
    Ok(config)
}

/// 内部配置加载函数（不使用缓存）
fn load_config_internal() -> Result<GameConfig, LauncherError> {
    let config_path = get_config_path()?;
    let is_first_run = !config_path.exists();

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        // 如果配置文件内容为空或损坏，自动备份并重建默认配置
        match serde_json::from_str::<GameConfig>(&content) {
            Ok(config) => Ok(config),
            Err(_) => {
                // 备份损坏的配置文件
                let backup_path = config_path.with_extension("bak");
                let _ = fs::copy(&config_path, &backup_path);
                log::warn!("配置文件损坏，已备份并重建默认配置");
                // 重建默认配置
                create_default_config(is_first_run)
            }
        }
    } else {
        // 创建默认配置
        create_default_config(is_first_run)
    }
}

/// 创建默认配置
fn create_default_config(is_first_run: bool) -> Result<GameConfig, LauncherError> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| LauncherError::Custom("无法获取可执行文件目录".to_string()))?;

    let mc_dir = exe_dir.join(".minecraft");
    let mc_dir_str = mc_dir.to_string_lossy().into_owned();

    if !mc_dir.exists() {
        fs::create_dir_all(&mc_dir)?;
        let sub_dirs = [
            "versions",
            "libraries",
            "assets",
            "saves",
            "resourcepacks",
            "logs",
        ];
        for dir in sub_dirs {
            fs::create_dir_all(mc_dir.join(dir))?;
        }
    }

    let mut config = GameConfig {
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
        download_mirror: Some("bmcl".to_string()),
        auto_memory_enabled: false,
        window_width: None,
        window_height: None,
        fullscreen: false,
        instance_last_played: std::collections::HashMap::new(),
        last_selected_version: None,
    };

    // 首次运行时自动检测Java
    if is_first_run {
        if let Ok(java_paths) = auto_detect_java() {
            if let Some(java_path) = java_paths.first() {
                config.java_path = Some(java_path.clone());
                log::info!("首次启动自动检测到Java路径: {}", java_path);
            }
        }
    }

    save_config_internal(&config)?;
    Ok(config)
}

use crate::services::java::auto_detect_java;

/// 保存配置文件（同时更新缓存）
pub fn save_config(config: &GameConfig) -> Result<(), LauncherError> {
    save_config_internal(config)?;
    
    // 更新缓存
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = Some(config.clone());
    }
    
    Ok(())
}

/// 内部保存函数（不更新缓存）
fn save_config_internal(config: &GameConfig) -> Result<(), LauncherError> {
    let config_path = get_config_path()?;
    fs::write(config_path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

/// 获取配置文件路径
fn get_config_path() -> Result<PathBuf, LauncherError> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| LauncherError::Custom("无法获取可执行文件目录".to_string()))?;

    Ok(exe_dir.join("ar1s.json"))
}

/// 配置键值映射定义
#[derive(Debug, Clone, Copy)]
enum ConfigKey {
    JavaPath,
    GameDir,
    VersionIsolation,
    DownloadThreads,
    Language,
    IsolateSaves,
    IsolateResourcepacks,
    IsolateLogs,
    Username,
    Uuid,
    MaxMemory,
    DownloadMirror,
}

impl ConfigKey {
    fn from_str(key: &str) -> Option<Self> {
        match key {
            "javaPath" => Some(Self::JavaPath),
            "gameDir" => Some(Self::GameDir),
            "versionIsolation" => Some(Self::VersionIsolation),
            "downloadThreads" => Some(Self::DownloadThreads),
            "language" => Some(Self::Language),
            "isolateSaves" => Some(Self::IsolateSaves),
            "isolateResourcepacks" => Some(Self::IsolateResourcepacks),
            "isolateLogs" => Some(Self::IsolateLogs),
            "username" => Some(Self::Username),
            "uuid" => Some(Self::Uuid),
            "maxMemory" => Some(Self::MaxMemory),
            "downloadMirror" => Some(Self::DownloadMirror),
            _ => None,
        }
    }

    fn get_value(&self, config: &GameConfig) -> Option<String> {
        match self {
            Self::JavaPath => config.java_path.clone(),
            Self::GameDir => Some(config.game_dir.clone()),
            Self::VersionIsolation => Some(config.version_isolation.to_string()),
            Self::DownloadThreads => Some(config.download_threads.to_string()),
            Self::Language => config.language.clone(),
            Self::IsolateSaves => Some(config.isolate_saves.to_string()),
            Self::IsolateResourcepacks => Some(config.isolate_resourcepacks.to_string()),
            Self::IsolateLogs => Some(config.isolate_logs.to_string()),
            Self::Username => config.username.clone(),
            Self::Uuid => config.uuid.clone(),
            Self::MaxMemory => Some(config.max_memory.to_string()),
            Self::DownloadMirror => config.download_mirror.clone(),
        }
    }

    fn set_value(&self, config: &mut GameConfig, value: String) -> Result<(), LauncherError> {
        match self {
            Self::JavaPath => config.java_path = Some(value),
            Self::GameDir => config.game_dir = value,
            Self::VersionIsolation => {
                config.version_isolation = value.parse().map_err(|_| {
                    LauncherError::Custom("版本隔离设置值无效".to_string())
                })?
            }
            Self::DownloadThreads => {
                config.download_threads = value.parse().map_err(|_| {
                    LauncherError::Custom("下载线程数设置值无效".to_string())
                })?
            }
            Self::Language => config.language = Some(value),
            Self::IsolateSaves => {
                config.isolate_saves = value.parse().map_err(|_| {
                    LauncherError::Custom("存档隔离设置值无效".to_string())
                })?
            }
            Self::IsolateResourcepacks => {
                config.isolate_resourcepacks = value.parse().map_err(|_| {
                    LauncherError::Custom("资源包隔离设置值无效".to_string())
                })?
            }
            Self::IsolateLogs => {
                config.isolate_logs = value.parse().map_err(|_| {
                    LauncherError::Custom("日志隔离设置值无效".to_string())
                })?
            }
            Self::Username => config.username = Some(value),
            Self::Uuid => config.uuid = Some(value),
            Self::MaxMemory => {
                config.max_memory = value.parse().map_err(|_| {
                    LauncherError::Custom("最大内存设置值无效".to_string())
                })?
            }
            Self::DownloadMirror => config.download_mirror = Some(value),
        }
        Ok(())
    }
}

pub async fn load_config_key(key: String) -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    match ConfigKey::from_str(&key) {
        Some(config_key) => Ok(config_key.get_value(&config)),
        None => Err(LauncherError::Custom(format!(
            "未知的配置项: {}",
            key
        ))),
    }
}

pub async fn save_config_key(key: String, value: String) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    match ConfigKey::from_str(&key) {
        Some(config_key) => {
            config_key.set_value(&mut config, value)?;
            save_config(&config)
        }
        None => Err(LauncherError::Custom(format!(
            "未知的配置项: {}",
            key
        ))),
    }
}

/// 通用配置获取函数
fn get_config_value<T, F>(getter: F) -> Result<T, LauncherError>
where
    F: FnOnce(&GameConfig) -> T,
{
    let config = load_config()?;
    Ok(getter(&config))
}

/// 通用配置设置函数
async fn set_config_value<T, F>(setter: F) -> Result<(), LauncherError>
where
    F: FnOnce(&mut GameConfig) -> T,
{
    let mut config = load_config()?;
    setter(&mut config);
    save_config(&config)
}

pub fn get_game_dir() -> Result<String, LauncherError> {
    get_config_value(|config| config.game_dir.clone())
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
        total_size: 0,
    })
}

pub async fn set_game_dir(path: String, window: &tauri::Window) -> Result<(), LauncherError> {
    let path_clone = path.clone();
    set_config_value(|config| config.game_dir = path_clone).await?;
    window.emit("game-dir-changed", &path)?;
    Ok(())
}

pub async fn set_version_isolation(enabled: bool) -> Result<(), LauncherError> {
    set_config_value(|config| config.version_isolation = enabled).await
}

pub fn get_download_threads() -> Result<u8, LauncherError> {
    get_config_value(|config| config.download_threads)
}

pub async fn set_download_threads(threads: u8) -> Result<(), LauncherError> {
    set_config_value(|config| config.download_threads = threads).await
}

pub fn get_total_memory() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.total_memory()
}

/// 获取系统内存统计信息
pub async fn get_memory_stats() -> Result<MemoryStats, LauncherError> {
    Ok(get_system_memory())
}

/// 为指定游戏版本推荐内存设置
pub async fn recommend_memory(
    version: String,
    modded: bool,
) -> Result<MemoryRecommendation, LauncherError> {
    Ok(recommend_memory_for_game(&version, modded))
}

/// 检查内存设置是否安全（只检查最低限制）
pub async fn validate_memory_setting(memory_mb: u32) -> Result<bool, LauncherError> {
    is_memory_setting_safe(memory_mb)
}

/// 检查内存设置是否超过系统90%（用于前端警告）
pub async fn check_memory_warning(memory_mb: u32) -> Result<Option<String>, LauncherError> {
    Ok(get_memory_warning_message(memory_mb))
}

/// 获取自动内存配置
pub async fn get_auto_memory_config() -> Result<AutoMemoryConfig, LauncherError> {
    let config = load_config()?;
    let auto_config = AutoMemoryConfig {
        enabled: config.auto_memory_enabled,
        max_limit_mb: 8500,          // 整合包优化模组要求的最大限制
        safety_margin_percent: 20.0, // 保留20%的安全余量
    };
    Ok(auto_config)
}

/// 设置自动内存启用状态
pub async fn set_auto_memory_enabled(enabled: bool) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.auto_memory_enabled = enabled;
    save_config(&config)
}

/// 自动设置内存（如果启用自动设置）
pub async fn auto_set_memory() -> Result<Option<u32>, LauncherError> {
    let config = load_config()?;
    let auto_config = AutoMemoryConfig {
        enabled: config.auto_memory_enabled,
        max_limit_mb: 8500,
        safety_margin_percent: 20.0,
    };

    if !auto_config.enabled {
        return Ok(None);
    }

    let recommended_memory = auto_set_memory_if_enabled(&auto_config);
    Ok(recommended_memory)
}

/// 分析内存使用效率
pub async fn analyze_memory_efficiency(memory_mb: u32) -> Result<String, LauncherError> {
    Ok(crate::services::memory::analyze_memory_efficiency(
        memory_mb,
    ))
}

/// 更新实例的上次启动时间
pub fn update_instance_last_played(instance_name: &str) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    config.instance_last_played.insert(instance_name.to_string(), now);
    save_config(&config)
}

/// 获取实例的上次启动时间
pub fn get_instance_last_played(instance_name: &str) -> Option<i64> {
    load_config().ok()
        .and_then(|config| config.instance_last_played.get(instance_name).copied())
}

/// 删除实例的上次启动时间记录
pub fn remove_instance_last_played(instance_name: &str) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.instance_last_played.remove(instance_name);
    save_config(&config)
}

/// 重命名实例的上次启动时间记录
pub fn rename_instance_last_played(old_name: &str, new_name: &str) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    if let Some(time) = config.instance_last_played.remove(old_name) {
        config.instance_last_played.insert(new_name.to_string(), time);
        save_config(&config)?;
    }
    Ok(())
}

/// 获取上次选择的游戏版本
pub fn get_last_selected_version() -> Option<String> {
    load_config().ok().and_then(|c| c.last_selected_version)
}

/// 设置上次选择的游戏版本
pub fn set_last_selected_version(version: &str) -> Result<(), LauncherError> {
    let mut config = load_config()?;
    config.last_selected_version = Some(version.to_string());
    save_config(&config)
}
