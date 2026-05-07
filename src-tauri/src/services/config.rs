use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use tauri::Emitter;

use crate::errors::LauncherError;
use crate::models::{GameConfig, GameDirInfo};
use crate::services::memory::{
    auto_set_memory_if_enabled, get_memory_warning_message, get_system_memory,
    is_memory_setting_safe, recommend_memory_for_game, AutoMemoryConfig, MemoryRecommendation,
    MemoryStats, DEFAULT_AUTO_MEMORY_MAX_LIMIT_MB, DEFAULT_AUTO_MEMORY_SAFETY_MARGIN_PERCENT,
};

static CONFIG_CACHE: std::sync::LazyLock<RwLock<Option<Arc<GameConfig>>>> =
    std::sync::LazyLock::new(|| RwLock::new(None));

fn with_read_lock<F, R>(f: F) -> R
where
    F: FnOnce(&Option<Arc<GameConfig>>) -> R,
{
    let guard = match CONFIG_CACHE.read().ok() {
        Some(g) => g,
        None => {
            log::warn!("配置缓存读取锁被破坏，使用空缓存");
            return f(&None);
        }
    };
    f(&guard)
}

fn with_write_lock<F, R>(f: F) -> R
where
    F: FnOnce(&mut Option<Arc<GameConfig>>) -> R,
{
    let mut guard = match CONFIG_CACHE.write() {
        Ok(g) => g,
        Err(poisoned) => {
            log::warn!("配置缓存写入锁被破坏，正在恢复");
            poisoned.into_inner()
        }
    };
    f(&mut guard)
}

pub fn preload_config() -> Result<(), LauncherError> {
    if with_read_lock(|cache| cache.is_some()) {
        return Ok(());
    }

    log::info!("预加载配置文件...");
    let config = load_config_internal()?;
    with_write_lock(|c| {
        *c = Some(Arc::new(config));
    });
    log::info!("配置文件预加载完成");
    Ok(())
}

pub fn invalidate_config_cache() {
    with_write_lock(|cache| {
        *cache = None;
    });
    log::debug!("配置缓存已清除");
}

// 获取保存的用户名
pub fn get_saved_username() -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    Ok(config.username.clone())
}

pub fn set_saved_username(username: String) -> Result<(), LauncherError> {
    set_config_value(|config| config.username = Some(username))
}

// 获取保存的UUID
pub fn get_saved_uuid() -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    Ok(config.uuid.clone())
}

pub fn set_saved_uuid(uuid: String) -> Result<(), LauncherError> {
    set_config_value(|config| config.uuid = Some(uuid))
}

pub fn load_config() -> Result<Arc<GameConfig>, LauncherError> {
    if let Some(config) = with_read_lock(|cache| cache.clone()) {
        return Ok(config);
    }

    let config = load_config_internal()?;
    let arc = Arc::new(config);
    with_write_lock(|cache| {
        *cache = Some(arc.clone());
    });
    Ok(arc)
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
    let exe_dir = EXE_DIR
        .as_ref()
        .map_err(|e| LauncherError::Custom(e.clone()))?;

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
        custom_java_paths: Vec::new(),
        auto_match_java: false,
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
        auth_type: crate::models::AuthType::Offline,
        ms_access_token: None,
        ms_refresh_token: None,
        ms_expires_at: None,
    };

    // 首次运行时自动检测Java
    if is_first_run {
        if let Ok(java_paths) = auto_detect_java() {
            if !java_paths.is_empty() {
                let best = java_paths
                    .iter()
                    .filter_map(|p| {
                        crate::services::launcher::java::detect_java_version(p)
                            .map(|v| (p.clone(), v))
                    })
                    .max_by_key(|(_, v)| *v)
                    .map(|(p, _)| p)
                    .unwrap_or_else(|| java_paths[0].clone());
                config.java_path = Some(best.clone());
                log::info!("首次启动自动检测到Java路径: {}", best);
            }
        }
    }

    save_config_internal(&config)?;
    Ok(config)
}

use crate::services::java::auto_detect_java;

pub fn save_config(config: &GameConfig) -> Result<(), LauncherError> {
    save_config_internal(config)?;
    with_write_lock(|cache| {
        *cache = Some(Arc::new(config.clone()));
    });
    Ok(())
}

/// 内部保存函数（不更新缓存）
fn save_config_internal(config: &GameConfig) -> Result<(), LauncherError> {
    let config_path = get_config_path()?;
    fs::write(config_path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

/// 缓存的可执行文件所在目录（避免每次调用 current_exe()）
static EXE_DIR: std::sync::LazyLock<Result<PathBuf, String>> = std::sync::LazyLock::new(|| {
    std::env::current_exe()
        .map_err(|e| format!("无法获取可执行文件路径: {}", e))?
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "无法获取可执行文件目录".to_string())
});

/// 获取配置文件路径（使用缓存的可执行文件目录）
fn get_config_path() -> Result<PathBuf, LauncherError> {
    let exe_dir = EXE_DIR
        .as_ref()
        .map_err(|e| LauncherError::Custom(e.clone()))?;
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
    AutoMatchJava,
    AutoMemoryEnabled,
    WindowWidth,
    WindowHeight,
    Fullscreen,
    AuthType,
    LastSelectedVersion,
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
            "autoMatchJava" => Some(Self::AutoMatchJava),
            "autoMemoryEnabled" => Some(Self::AutoMemoryEnabled),
            "windowWidth" => Some(Self::WindowWidth),
            "windowHeight" => Some(Self::WindowHeight),
            "fullscreen" => Some(Self::Fullscreen),
            "authType" => Some(Self::AuthType),
            "lastSelectedVersion" => Some(Self::LastSelectedVersion),
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
            Self::AutoMatchJava => Some(config.auto_match_java.to_string()),
            Self::AutoMemoryEnabled => Some(config.auto_memory_enabled.to_string()),
            Self::WindowWidth => config.window_width.map(|v| v.to_string()),
            Self::WindowHeight => config.window_height.map(|v| v.to_string()),
            Self::Fullscreen => Some(config.fullscreen.to_string()),
            Self::AuthType => Some(match config.auth_type {
                crate::models::AuthType::Offline => "offline".to_string(),
                crate::models::AuthType::Microsoft => "microsoft".to_string(),
            }),
            Self::LastSelectedVersion => config.last_selected_version.clone(),
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
            Self::AutoMatchJava => {
                config.auto_match_java = value.parse().map_err(|_| {
                    LauncherError::Custom("自动匹配Java设置值无效".to_string())
                })?
            }
            Self::AutoMemoryEnabled => {
                config.auto_memory_enabled = value.parse().map_err(|_| {
                    LauncherError::Custom("自动内存设置值无效".to_string())
                })?
            }
            Self::WindowWidth => {
                config.window_width = Some(value.parse().map_err(|_| {
                    LauncherError::Custom("窗口宽度设置值无效".to_string())
                })?)
            }
            Self::WindowHeight => {
                config.window_height = Some(value.parse().map_err(|_| {
                    LauncherError::Custom("窗口高度设置值无效".to_string())
                })?)
            }
            Self::Fullscreen => {
                config.fullscreen = value.parse().map_err(|_| {
                    LauncherError::Custom("全屏设置值无效".to_string())
                })?
            }
            Self::AuthType => {
                config.auth_type = match value.to_lowercase().as_str() {
                    "offline" => crate::models::AuthType::Offline,
                    "microsoft" => crate::models::AuthType::Microsoft,
                    _ => return Err(LauncherError::Custom("认证类型无效，仅支持 offline 或 microsoft".to_string())),
                }
            }
            Self::LastSelectedVersion => config.last_selected_version = Some(value),
        }
        Ok(())
    }
}

pub fn load_config_key(key: String) -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    match ConfigKey::from_str(&key) {
        Some(config_key) => Ok(config_key.get_value(&config)),
        None => Err(LauncherError::Custom(format!(
            "未知的配置项: {}",
            key
        ))),
    }
}

pub fn save_config_key(key: String, value: String) -> Result<(), LauncherError> {
    let mut config = (*load_config()?).clone();
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
fn set_config_value<T, F>(setter: F) -> Result<(), LauncherError>
where
    F: FnOnce(&mut GameConfig) -> T,
{
    let mut config = (*load_config()?).clone();
    setter(&mut config);
    save_config(&config)
}

pub fn get_game_dir() -> Result<String, LauncherError> {
    get_config_value(|config| config.game_dir.clone())
}

pub fn get_game_dir_info() -> Result<GameDirInfo, LauncherError> {
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

pub fn set_game_dir(path: String, window: &tauri::Window) -> Result<(), LauncherError> {
    let path_clone = path.clone();
    set_config_value(|config| config.game_dir = path_clone)?;
    window.emit("game-dir-changed", &path)?;
    Ok(())
}

pub fn set_version_isolation(enabled: bool) -> Result<(), LauncherError> {
    set_config_value(|config| config.version_isolation = enabled)
}

pub fn get_download_threads() -> Result<u8, LauncherError> {
    get_config_value(|config| config.download_threads)
}

pub fn set_download_threads(threads: u8) -> Result<(), LauncherError> {
    set_config_value(|config| config.download_threads = threads)
}

/// 获取系统总内存（单位: MB）
pub fn get_total_memory() -> u64 {
    let stats = get_system_memory();
    stats.total_memory_mb
}

/// 获取系统内存统计信息
pub fn get_memory_stats() -> Result<MemoryStats, LauncherError> {
    Ok(get_system_memory())
}

/// 为指定游戏版本推荐内存设置
pub fn recommend_memory(
    version: String,
    modded: bool,
) -> Result<MemoryRecommendation, LauncherError> {
    Ok(recommend_memory_for_game(&version, modded))
}

/// 检查内存设置是否安全（只检查最低限制）
pub fn validate_memory_setting(memory_mb: u32) -> Result<bool, LauncherError> {
    is_memory_setting_safe(memory_mb)
}

/// 检查内存设置是否超过系统90%（用于前端警告）
pub fn check_memory_warning(memory_mb: u32) -> Result<Option<String>, LauncherError> {
    Ok(get_memory_warning_message(memory_mb))
}

/// 获取自动内存配置
pub fn get_auto_memory_config() -> Result<AutoMemoryConfig, LauncherError> {
    let config = load_config()?;
    let auto_config = AutoMemoryConfig {
        enabled: config.auto_memory_enabled,
        max_limit_mb: DEFAULT_AUTO_MEMORY_MAX_LIMIT_MB,
        safety_margin_percent: DEFAULT_AUTO_MEMORY_SAFETY_MARGIN_PERCENT,
    };
    Ok(auto_config)
}

pub fn set_auto_memory_enabled(enabled: bool) -> Result<(), LauncherError> {
    set_config_value(|config| config.auto_memory_enabled = enabled)
}

/// 自动设置内存（如果启用自动设置）
pub fn auto_set_memory() -> Result<Option<u32>, LauncherError> {
    let config = load_config()?;
    let auto_config = AutoMemoryConfig {
        enabled: config.auto_memory_enabled,
        max_limit_mb: DEFAULT_AUTO_MEMORY_MAX_LIMIT_MB,
        safety_margin_percent: DEFAULT_AUTO_MEMORY_SAFETY_MARGIN_PERCENT,
    };

    if !auto_config.enabled {
        return Ok(None);
    }

    let recommended_memory = auto_set_memory_if_enabled(&auto_config);
    Ok(recommended_memory)
}

/// 分析内存使用效率
pub fn analyze_memory_efficiency(memory_mb: u32) -> Result<String, LauncherError> {
    Ok(crate::services::memory::analyze_memory_efficiency(
        memory_mb,
    ))
}

pub fn update_instance_last_played(instance_name: &str) -> Result<(), LauncherError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let key = instance_name.to_string();
    set_config_value(|config| config.instance_last_played.insert(key, now))
}

/// 获取实例的上次启动时间
pub fn get_instance_last_played(instance_name: &str) -> Option<i64> {
    load_config().ok()
        .and_then(|config| config.instance_last_played.get(instance_name).copied())
}

pub fn remove_instance_last_played(instance_name: &str) -> Result<(), LauncherError> {
    let key = instance_name.to_string();
    set_config_value(|config| config.instance_last_played.remove(&key))
}

pub fn rename_instance_last_played(old_name: &str, new_name: &str) -> Result<(), LauncherError> {
    let old_key = old_name.to_string();
    let new_key = new_name.to_string();
    set_config_value(|config| {
        if let Some(time) = config.instance_last_played.remove(&old_key) {
            config.instance_last_played.insert(new_key, time);
        }
    })
}

/// 获取上次选择的游戏版本
pub fn get_last_selected_version() -> Option<String> {
    load_config().ok().and_then(|c| c.last_selected_version.clone())
}

pub fn set_last_selected_version(version: &str) -> Result<(), LauncherError> {
    let v = version.to_string();
    set_config_value(|config| config.last_selected_version = Some(v))
}

pub fn get_custom_java_paths() -> Vec<String> {
    load_config().map(|c| c.custom_java_paths.clone()).unwrap_or_default()
}

pub fn add_custom_java_path(path: &str) -> Result<(), LauncherError> {
    let p = path.to_string();
    set_config_value(|config| {
        if !config.custom_java_paths.iter().any(|x| x.eq_ignore_ascii_case(&p)) {
            config.custom_java_paths.push(p);
        }
    })
}

pub fn remove_custom_java_path(path: &str) -> Result<(), LauncherError> {
    let p = path.to_string();
    set_config_value(|config| {
        config.custom_java_paths.retain(|x| !x.eq_ignore_ascii_case(&p));
    })
}
