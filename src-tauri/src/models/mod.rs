//! 数据模型模块集合
//!
//! 前后端之间传递的 serde 结构与默认值函数，只有字段定义、不含读写逻辑

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn default_download_threads() -> u8 {
    8
}

// 单位 MB
pub fn default_max_memory() -> u32 {
    4096
}

pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    Offline,
    Microsoft,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::Offline
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GcStrategy {
    G1,
    /// 不指定回收器，交给 JVM 默认，与官方启动器一致
    Default,
    /// 吞吐量优先，适合核心数较少的机器
    Parallel,
    /// 单线程回收，通常只用于排查 GC 相关问题
    Serial,
    /// 极低停顿，适合超大堆；Java 15+，旧 Java 自动回退 G1
    Z,
}

impl Default for GcStrategy {
    fn default() -> Self {
        GcStrategy::G1
    }
}

// 默认只做 G1GC 优化，其余 JVM 参数与官方启动器保持一致；用户可在「设置 - 高级选项」逐项开启
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct JvmAdvancedOptions {
    #[serde(default)]
    pub gc_strategy: GcStrategy,
    /// 启动时提交整个堆换帧时间稳定，代价是启动变慢
    #[serde(default = "default_false")]
    pub always_pre_touch: bool,
    /// 禁止模组调用 System.gc() 触发的 Full GC
    #[serde(default = "default_false")]
    pub disable_explicit_gc: bool,
    /// 字符串去重：省内存，但会拉长 Young GC 时间
    #[serde(default = "default_false")]
    pub string_deduplication: bool,
    /// 关闭 hsperfdata 性能计数器共享内存
    #[serde(default = "default_false")]
    pub perf_disable_shared_mem: bool,
    /// Metaspace 初始水位 (MB)，None 表示不加该参数、交给 JVM 默认
    #[serde(default)]
    pub metaspace_size_mb: Option<u32>,
    /// Metaspace 上限 (MB)，None 表示不限制；不限的话大型整合包的类元数据会一直向系统申请本地内存，最终拖垮整机
    #[serde(default)]
    pub metaspace_max_size_mb: Option<u32>,
    /// OutOfMemoryError 时导出堆转储到 ./logs/heapdump.hprof
    #[serde(default = "default_false")]
    pub heap_dump_on_oom: bool,
    /// 自定义追加参数，每行一条（或空格分隔），追加在所有内置参数之后
    #[serde(default)]
    pub extra_args: String,
}

impl Default for JvmAdvancedOptions {
    fn default() -> Self {
        Self {
            gc_strategy: GcStrategy::default(),
            always_pre_touch: false,
            disable_explicit_gc: false,
            string_deduplication: false,
            perf_disable_shared_mem: false,
            metaspace_size_mb: None,
            metaspace_max_size_mb: None,
            heap_dump_on_oom: false,
            extra_args: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_dir: String,
    #[serde(default = "default_true")]
    pub version_isolation: bool,
    pub java_path: Option<String>,
    #[serde(default)]
    pub custom_java_paths: Vec<String>,
    #[serde(default = "default_false")]
    pub auto_match_java: bool,
    #[serde(default = "default_download_threads")]
    pub download_threads: u8,
    pub language: Option<String>,
    #[serde(default = "default_true")]
    pub isolate_saves: bool,
    #[serde(default = "default_true")]
    pub isolate_resourcepacks: bool,
    #[serde(default = "default_true")]
    pub isolate_logs: bool,
    pub username: Option<String>,
    pub uuid: Option<String>,
    /// 正版皮肤原图地址，登录/刷新 token 时从 Mojang 个人资料接口取
    pub skin_url: Option<String>,
    #[serde(default = "default_max_memory")]
    pub max_memory: u32,
    pub download_mirror: Option<String>,
    #[serde(default = "default_false")]
    pub auto_memory_enabled: bool,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    #[serde(default = "default_false")]
    pub fullscreen: bool,
    /// 实例上次启动时间 (实例名 -> 时间戳毫秒)
    #[serde(default)]
    pub instance_last_played: HashMap<String, i64>,
    pub last_selected_version: Option<String>,
    /// 界面配色方案 (indigo / emerald / rose)
    pub theme_palette: Option<String>,
    /// 界面明暗模式 (dark / light)
    pub theme_mode: Option<String>,
    #[serde(default)]
    pub auth_type: AuthType,
    /// Microsoft access_token (仅后端使用)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms_access_token: Option<String>,
    /// Microsoft refresh_token (仅后端使用)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms_refresh_token: Option<String>,
    /// 正版 token 过期时间 (Unix 时间戳秒)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ms_expires_at: Option<i64>,
    #[serde(default)]
    pub jvm_advanced: JvmAdvancedOptions,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            game_dir: String::new(),
            version_isolation: true,
            java_path: None,
            custom_java_paths: Vec::new(),
            auto_match_java: false,
            download_threads: default_download_threads(),
            language: None,
            isolate_saves: true,
            isolate_resourcepacks: true,
            isolate_logs: true,
            username: None,
            uuid: None,
            skin_url: None,
            max_memory: default_max_memory(),
            download_mirror: None,
            auto_memory_enabled: false,
            window_width: None,
            window_height: None,
            fullscreen: false,
            instance_last_played: HashMap::new(),
            last_selected_version: None,
            theme_palette: None,
            theme_mode: None,
            auth_type: AuthType::default(),
            ms_access_token: None,
            ms_refresh_token: None,
            ms_expires_at: None,
            jvm_advanced: JvmAdvancedOptions::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameDirInfo {
    pub path: String,
    pub versions: Vec<String>,
    pub total_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchOptions {
    pub version: String,
    pub username: String,
    pub memory: Option<u32>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub fullscreen: Option<bool>,
    pub auth_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_java_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaCompatibilityResult {
    pub compatible: bool,
    pub current_java_version: Option<u32>,
    pub required_java_version: u32,
    pub recommended_min_java_version: u32,
    pub recommended_max_java_version: u32,
    pub current_java_path: Option<String>,
    pub recommended_java_path: Option<String>,
    pub recommended_java_version: Option<u32>,
    pub recommended_java_optimal: bool,
    pub recommended_java_warning: Option<String>,
    pub auto_match_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Downloading,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub speed: f64,
    pub status: DownloadStatus,
    pub percent: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub url: String,
    pub fallback_url: Option<String>,
    pub path: PathBuf,
    pub size: u64,
    pub hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub display_name: String,
    pub minecraft_version: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: String,
    pub created_time: Option<String>,
    pub loader_type: Option<String>,
    pub loader_version: Option<String>,
    pub game_version: Option<String>,
    pub last_played: Option<i64>,
    pub mod_count: Option<u32>,
    /// 整合包自带的封面图 URL，没有时前端回退到加载器图标
    pub icon_url: Option<String>,
}

pub mod modpack;
