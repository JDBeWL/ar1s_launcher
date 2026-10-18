//! 配置文件读写模块
//!
//! 配置固定放在 exe 同级目录；写入走临时文件 rename，
//! 解析失败时备份 .bak 再重建默认配置

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use tauri::Emitter;

use crate::errors::LauncherError;
use crate::models::{GameConfig, GameDirInfo};
use crate::services::credentials;
use crate::services::memory::{
    AutoMemoryConfig, DEFAULT_AUTO_MEMORY_MAX_LIMIT_MB, DEFAULT_AUTO_MEMORY_SAFETY_MARGIN_PERCENT,
    auto_set_memory_if_enabled, get_memory_warning_message, get_system_memory,
};

static CONFIG_CACHE: std::sync::LazyLock<RwLock<Option<Arc<GameConfig>>>> =
    std::sync::LazyLock::new(|| RwLock::new(None));

static CONFIG_MUTATION_LOCK: Mutex<()> = Mutex::new(());

fn exe_dir_or_dot() -> PathBuf {
    match EXE_DIR.as_ref() {
        Ok(dir) => dir.clone(),
        Err(_) => PathBuf::from("."),
    }
}

static STORAGE: OnceLock<PathBuf> = OnceLock::new();

/// 未初始化时退回 exe 目录，保证测试和早期调用仍有可用路径
fn storage() -> &'static Path {
    STORAGE.get_or_init(exe_dir_or_dot).as_path()
}

pub fn init_storage() -> &'static Path {
    storage()
}

fn default_game_dir(root: &Path) -> PathBuf {
    root.join(".minecraft")
}

fn lock_config() -> Result<std::sync::MutexGuard<'static, ()>, LauncherError> {
    match CONFIG_MUTATION_LOCK.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            log::warn!("配置写锁被破坏，正在恢复");
            Ok(poisoned.into_inner())
        }
    }
}

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

pub fn get_saved_username() -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    Ok(config.username.clone())
}

pub fn set_saved_username(username: String) -> Result<(), LauncherError> {
    set_config_value(|config| config.username = Some(username))
}

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

fn load_config_internal() -> Result<GameConfig, LauncherError> {
    let config_path = get_config_path()?;
    let is_first_run = !config_path.exists();

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        // 配置损坏时备份为 .bak 并重建默认配置，不向上报错
        match serde_json::from_str::<GameConfig>(&content) {
            Ok(mut config) => {
                // 旧版本会把 token 留在 ar1s.json 里；读到后迁移到 credentials.json
                let had_plaintext_tokens =
                    config.ms_access_token.is_some() || config.ms_refresh_token.is_some();
                match credentials::load() {
                    // 凭据文件比配置文件新：刷新 token 轮换过以后不能再用旧值覆盖
                    Ok(Some(stored)) => {
                        if stored.access_token.is_some() {
                            config.ms_access_token = stored.access_token;
                        }
                        if stored.refresh_token.is_some() {
                            config.ms_refresh_token = stored.refresh_token;
                        }
                        if stored.expires_at.is_some() {
                            config.ms_expires_at = stored.expires_at;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => log::warn!("读取凭据文件失败，继续使用配置文件: {}", e),
                }

                if had_plaintext_tokens && let Err(e) = save_config_internal(&config) {
                    log::warn!("迁移 token 到凭据文件失败: {}", e);
                }

                Ok(config)
            }
            Err(_) => {
                let backup_path = config_path.with_extension("bak");
                let _ = fs::copy(&config_path, &backup_path);
                log::warn!("配置文件损坏，已备份并重建默认配置");
                create_default_config(is_first_run)
            }
        }
    } else {
        create_default_config(is_first_run)
    }
}

fn create_default_config(is_first_run: bool) -> Result<GameConfig, LauncherError> {
    let mc_dir = default_game_dir(storage());
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
        username: None,
        uuid: None,
        skin_url: None,
        max_memory: crate::models::default_max_memory(),
        download_mirror: Some("bmcl".to_string()),
        auto_memory_enabled: false,
        window_width: None,
        window_height: None,
        fullscreen: false,
        instance_last_played: std::collections::HashMap::new(),
        last_selected_version: None,
        theme_palette: None,
        theme_mode: None,
        auth_type: crate::models::AuthType::Offline,
        ms_access_token: None,
        ms_refresh_token: None,
        ms_expires_at: None,
        jvm_advanced: crate::models::JvmAdvancedOptions::default(),
    };

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
    let _guard = lock_config()?;
    save_config_internal(config)?;
    with_write_lock(|cache| {
        *cache = Some(Arc::new(config.clone()));
    });
    Ok(())
}

/// 所有读改写都走这里，避免两个 command 各自 load 后用旧快照覆盖对方
pub fn update_config<F, R>(update: F) -> Result<R, LauncherError>
where
    F: FnOnce(&mut GameConfig) -> Result<R, LauncherError>,
{
    let _guard = lock_config()?;
    let mut config = (*load_config()?).clone();
    let result = update(&mut config)?;
    save_config_internal(&config)?;
    with_write_lock(|cache| {
        *cache = Some(Arc::new(config));
    });
    Ok(result)
}

fn has_tokens(config: &GameConfig) -> bool {
    config.ms_access_token.is_some() || config.ms_refresh_token.is_some()
}

fn config_for_disk(config: &GameConfig, tokens_stored: bool) -> GameConfig {
    let mut disk_config = config.clone();
    if tokens_stored {
        disk_config.ms_access_token = None;
        disk_config.ms_refresh_token = None;
    }
    disk_config
}

fn save_config_internal(config: &GameConfig) -> Result<(), LauncherError> {
    // token 优先落 credentials.json；写入失败时退回原行为，保证登录仍能工作
    let tokens_stored = if has_tokens(config) {
        let stored = credentials::Credentials {
            access_token: config.ms_access_token.clone(),
            refresh_token: config.ms_refresh_token.clone(),
            expires_at: config.ms_expires_at,
        };
        match credentials::save(&stored) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("凭据文件写入失败，token 仍会写入配置文件: {}", e);
                false
            }
        }
    } else {
        false
    };

    let config_path = get_config_path()?;
    let dir = config_path
        .parent()
        .ok_or_else(|| LauncherError::Custom("配置文件没有父目录".to_string()))?;
    fs::create_dir_all(dir)?;
    let json = serde_json::to_string_pretty(&config_for_disk(config, tokens_stored))?;

    // 直接覆写会在崩溃时留下截断的 ar1s.json，先写临时文件再 rename
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(json.as_bytes())?;
    tmp.flush()?;
    tmp.persist(&config_path).map_err(|e| e.error)?;
    Ok(())
}

static EXE_DIR: std::sync::LazyLock<Result<PathBuf, String>> = std::sync::LazyLock::new(|| {
    std::env::current_exe()
        .map_err(|e| format!("无法获取可执行文件路径: {}", e))?
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "无法获取可执行文件目录".to_string())
});

fn get_config_path() -> Result<PathBuf, LauncherError> {
    Ok(storage().join("ar1s.json"))
}

pub(crate) fn credentials_path() -> PathBuf {
    storage().join("credentials.json")
}

#[derive(Debug, Clone, Copy)]
enum ConfigKey {
    JavaPath,
    GameDir,
    VersionIsolation,
    DownloadThreads,
    Language,
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
    ThemePalette,
    ThemeMode,
}

impl ConfigKey {
    fn from_str(key: &str) -> Option<Self> {
        match key {
            "javaPath" => Some(Self::JavaPath),
            "gameDir" => Some(Self::GameDir),
            "versionIsolation" => Some(Self::VersionIsolation),
            "downloadThreads" => Some(Self::DownloadThreads),
            "language" => Some(Self::Language),
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
            "themePalette" => Some(Self::ThemePalette),
            "themeMode" => Some(Self::ThemeMode),
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
            Self::ThemePalette => config.theme_palette.clone(),
            Self::ThemeMode => config.theme_mode.clone(),
        }
    }

    fn set_value(&self, config: &mut GameConfig, value: String) -> Result<(), LauncherError> {
        match self {
            Self::JavaPath => config.java_path = Some(value),
            Self::GameDir => config.game_dir = value,
            Self::VersionIsolation => {
                config.version_isolation = value
                    .parse()
                    .map_err(|_| LauncherError::Custom("版本隔离设置值无效".to_string()))?
            }
            Self::DownloadThreads => {
                config.download_threads = value
                    .parse()
                    .map_err(|_| LauncherError::Custom("下载线程数设置值无效".to_string()))?
            }
            Self::Language => config.language = Some(value),
            Self::Username => config.username = Some(value),
            Self::Uuid => config.uuid = Some(value),
            Self::MaxMemory => {
                config.max_memory = value
                    .parse()
                    .map_err(|_| LauncherError::Custom("最大内存设置值无效".to_string()))?
            }
            Self::DownloadMirror => config.download_mirror = Some(value),
            Self::AutoMatchJava => {
                config.auto_match_java = value
                    .parse()
                    .map_err(|_| LauncherError::Custom("自动匹配Java设置值无效".to_string()))?
            }
            Self::AutoMemoryEnabled => {
                config.auto_memory_enabled = value
                    .parse()
                    .map_err(|_| LauncherError::Custom("自动内存设置值无效".to_string()))?
            }
            Self::WindowWidth => {
                config.window_width = Some(
                    value
                        .parse()
                        .map_err(|_| LauncherError::Custom("窗口宽度设置值无效".to_string()))?,
                )
            }
            Self::WindowHeight => {
                config.window_height = Some(
                    value
                        .parse()
                        .map_err(|_| LauncherError::Custom("窗口高度设置值无效".to_string()))?,
                )
            }
            Self::Fullscreen => {
                config.fullscreen = value
                    .parse()
                    .map_err(|_| LauncherError::Custom("全屏设置值无效".to_string()))?
            }
            Self::AuthType => {
                config.auth_type = match value.to_lowercase().as_str() {
                    "offline" => crate::models::AuthType::Offline,
                    "microsoft" => crate::models::AuthType::Microsoft,
                    _ => {
                        return Err(LauncherError::Custom(
                            "认证类型无效，仅支持 offline 或 microsoft".to_string(),
                        ));
                    }
                }
            }
            Self::LastSelectedVersion => config.last_selected_version = Some(value),
            Self::ThemePalette => config.theme_palette = Some(value),
            Self::ThemeMode => {
                config.theme_mode = match value.to_lowercase().as_str() {
                    "dark" | "light" => Some(value.to_lowercase()),
                    _ => {
                        return Err(LauncherError::Custom(
                            "主题模式无效，仅支持 dark 或 light".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn load_config_key(key: String) -> Result<Option<String>, LauncherError> {
    let config = load_config()?;
    match ConfigKey::from_str(&key) {
        Some(config_key) => Ok(config_key.get_value(&config)),
        None => Err(LauncherError::Custom(format!("未知的配置项: {}", key))),
    }
}

pub fn save_config_key(key: String, value: String) -> Result<(), LauncherError> {
    let config_key = ConfigKey::from_str(&key)
        .ok_or_else(|| LauncherError::Custom(format!("未知的配置项: {}", key)))?;
    update_config(|config| config_key.set_value(config, value))
}

fn get_config_value<T, F>(getter: F) -> Result<T, LauncherError>
where
    F: FnOnce(&GameConfig) -> T,
{
    let config = load_config()?;
    Ok(getter(&config))
}

fn set_config_value<T, F>(setter: F) -> Result<(), LauncherError>
where
    F: FnOnce(&mut GameConfig) -> T,
{
    update_config(|config| {
        setter(config);
        Ok(())
    })
}

pub fn get_game_dir() -> Result<String, LauncherError> {
    get_config_value(|config| config.game_dir.clone())
}

/// 启动、模组管理、整合包安装都靠它决定内容落到实例目录还是共享根目录；
/// 配置读不出来时按默认（隔离）处理
pub fn is_version_isolated() -> bool {
    load_config()
        .map(|config| config.version_isolation)
        .unwrap_or(true)
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

pub fn get_download_threads() -> Result<u8, LauncherError> {
    get_config_value(|config| config.download_threads)
}

pub fn set_download_threads(threads: u8) -> Result<(), LauncherError> {
    set_config_value(|config| config.download_threads = threads)
}

/// 单位：MB
pub fn get_total_memory() -> u64 {
    let stats = get_system_memory();
    stats.total_memory_mb
}

/// 超过系统内存 90% 时返回警告文案
pub fn check_memory_warning(memory_mb: u32) -> Result<Option<String>, LauncherError> {
    Ok(get_memory_warning_message(memory_mb))
}

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

pub fn get_instance_last_played(instance_name: &str) -> Option<i64> {
    load_config()
        .ok()
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

pub fn get_last_selected_version() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.last_selected_version.clone())
}

pub fn set_last_selected_version(version: &str) -> Result<(), LauncherError> {
    let v = version.to_string();
    set_config_value(|config| config.last_selected_version = Some(v))
}

pub fn get_custom_java_paths() -> Vec<String> {
    load_config()
        .map(|c| c.custom_java_paths.clone())
        .unwrap_or_default()
}

pub fn add_custom_java_path(path: &str) -> Result<(), LauncherError> {
    let p = path.to_string();
    set_config_value(|config| {
        if !config
            .custom_java_paths
            .iter()
            .any(|x| x.eq_ignore_ascii_case(&p))
        {
            config.custom_java_paths.push(p);
        }
    })
}

pub fn remove_custom_java_path(path: &str) -> Result<(), LauncherError> {
    let p = path.to_string();
    set_config_value(|config| {
        config
            .custom_java_paths
            .retain(|x| !x.eq_ignore_ascii_case(&p));
    })
}

#[cfg(test)]
mod tests {
    use super::{config_for_disk, default_game_dir};
    use std::path::PathBuf;

    #[test]
    fn test_game_dir_is_next_to_exe() {
        let exe = PathBuf::from("C:/launcher");
        assert_eq!(default_game_dir(&exe), exe.join(".minecraft"));
    }

    #[test]
    fn test_config_for_disk_only_strips_tokens_when_store_succeeded() {
        let config = crate::models::GameConfig {
            ms_access_token: Some("access".to_string()),
            ms_refresh_token: Some("refresh".to_string()),
            ms_expires_at: Some(123),
            ..Default::default()
        };

        let kept = config_for_disk(&config, false);
        assert_eq!(kept.ms_access_token.as_deref(), Some("access"));
        assert_eq!(kept.ms_refresh_token.as_deref(), Some("refresh"));

        let stripped = config_for_disk(&config, true);
        assert!(stripped.ms_access_token.is_none());
        assert!(stripped.ms_refresh_token.is_none());
        assert_eq!(stripped.ms_expires_at, Some(123));
    }
}
