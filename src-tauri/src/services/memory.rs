//! 内存测算与 JVM 参数模块
//!
//! 按系统内存与游戏版本给出推荐堆大小与超限告警，产出 -Xmx/-Xms、GC 策略与 Metaspace 参数

use crate::errors::LauncherError;
use crate::models::{GcStrategy, JvmAdvancedOptions};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use sysinfo::{MemoryRefreshKind, System};

static MEMORY_SYSTEM: std::sync::LazyLock<Mutex<System>> =
    std::sync::LazyLock::new(|| Mutex::new(System::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub available_memory_mb: u64,
    pub memory_usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecommendation {
    pub min_memory_mb: u32,
    pub recommended_memory_mb: u32,
    pub max_memory_mb: u32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoMemoryConfig {
    pub enabled: bool,
    pub max_limit_mb: u32,
    pub safety_margin_percent: f32,
}

pub const DEFAULT_AUTO_MEMORY_MAX_LIMIT_MB: u32 = 8500;
pub const DEFAULT_AUTO_MEMORY_SAFETY_MARGIN_PERCENT: f32 = 20.0;

pub fn get_system_memory() -> MemoryStats {
    let mut system = MEMORY_SYSTEM.lock().unwrap();
    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());

    let total_memory_mb = system.total_memory() / 1024 / 1024;
    let used_memory_mb = system.used_memory() / 1024 / 1024;
    let available_memory_mb = system.available_memory() / 1024 / 1024;
    let memory_usage_percent = (used_memory_mb as f64 / total_memory_mb as f64) * 100.0;

    MemoryStats {
        total_memory_mb,
        used_memory_mb,
        available_memory_mb,
        memory_usage_percent,
    }
}

pub fn recommend_memory_for_game(version: &str, modded: bool) -> MemoryRecommendation {
    let memory_stats = get_system_memory();
    let total_memory_mb = memory_stats.total_memory_mb as u32;

    let base_memory = if crate::utils::minecraft::is_new_version(version) {
        2048
    } else if crate::utils::minecraft::is_mid_version(version) {
        1536
    } else {
        1024
    };

    let mod_adjustment = if modded { 1024 } else { 0 };

    let base_need = base_memory + mod_adjustment;

    let min_memory = base_need.max(512);
    let recommended = calculate_recommended_memory(total_memory_mb, base_need);
    // max_memory 必须 >= recommended，否则推荐值没有意义
    let max_memory = calculate_max_memory(total_memory_mb, base_need).max(recommended);

    let reason = format!(
        "系统总内存: {}MB, 游戏版本: {}, {}",
        total_memory_mb,
        version,
        if modded { "模组版" } else { "原版" }
    );

    MemoryRecommendation {
        min_memory_mb: min_memory,
        recommended_memory_mb: recommended,
        max_memory_mb: max_memory,
        reason,
    }
}

/// 只依据系统内存推荐，不考虑游戏版本与模组
pub fn recommend_memory_by_system(config: &AutoMemoryConfig) -> MemoryRecommendation {
    let memory_stats = get_system_memory();
    let total_memory_mb = memory_stats.total_memory_mb as u32;
    let available_memory_mb = memory_stats.available_memory_mb as u32;

    let recommended = calculate_smart_memory(total_memory_mb, available_memory_mb, config);

    let recommended = recommended.min(config.max_limit_mb);

    let min_memory = 512;

    let max_memory = (total_memory_mb as f32 * 0.7) as u32;

    let reason = format!(
        "智能推荐：系统总内存{}MB，可用内存{}MB，推荐设置{}MB{}",
        total_memory_mb,
        available_memory_mb,
        recommended,
        if recommended >= config.max_limit_mb {
            format!("（已达到最大限制{}MB）", config.max_limit_mb)
        } else {
            String::new()
        }
    );

    MemoryRecommendation {
        min_memory_mb: min_memory,
        recommended_memory_mb: recommended,
        max_memory_mb: max_memory,
        reason,
    }
}

fn calculate_smart_memory(
    total_memory: u32,
    available_memory: u32,
    config: &AutoMemoryConfig,
) -> u32 {
    let safe_available =
        (available_memory as f32 * (1.0 - config.safety_margin_percent / 100.0)) as u32;

    match total_memory {
        t if t <= 4096 => (safe_available as f32 * 0.6) as u32,

        t if t <= 8192 => (safe_available as f32 * 0.5) as u32,

        t if t <= 16384 => (safe_available as f32 * 0.4) as u32,

        _ => (safe_available as f32 * 0.35) as u32,
    }
    .max(1024)
    .min(total_memory)
}

fn calculate_recommended_memory(total_memory: u32, base_need: u32) -> u32 {
    if total_memory <= 4096 {
        (total_memory as f32 * 0.4) as u32
    } else if total_memory <= 8192 {
        base_need.max(2048).min(3072)
    } else if total_memory <= 16384 {
        base_need.max(3072).min(6144)
    } else {
        base_need.max(4096).min(8192)
    }
}

fn calculate_max_memory(total_memory: u32, base_need: u32) -> u32 {
    let max_safe = (total_memory as f32 * 0.7) as u32;
    base_need.max(1024).min(max_safe)
}

/// 默认仅追加 G1GC 调优，其余维持官方启动器的最小参数集
///
/// 更多调优项由 `JvmAdvancedOptions` 开关控制
pub fn optimize_jvm_memory_args(
    memory_mb: u32,
    java_version: Option<u32>,
    advanced: &JvmAdvancedOptions,
) -> Vec<String> {
    let mut args = Vec::new();

    let cpu_cores = num_cpus::get() as u32;
    // GC 线程数：留 1-2 核给游戏主线程和系统
    let gc_threads = if cpu_cores <= 2 {
        1
    } else {
        (cpu_cores - 1).min(4)
    };

    // Xms 与 Xmx 相等，避免堆动态扩缩的开销
    args.push(format!("-Xmx{}M", memory_mb));
    args.push(format!("-Xms{}M", memory_mb));

    let java_major = java_version.unwrap_or(17);

    // 新版 Java 下 LWJGL/JNA/JOML 正常工作的前提，不属于调优项
    if java_major >= 17 {
        args.push("--enable-native-access=ALL-UNNAMED".to_string());
    }
    if java_major >= 23 {
        args.push("--sun-misc-unsafe-memory-access=allow".to_string());
    }

    match advanced.gc_strategy {
        GcStrategy::G1 => {
            args.push("-XX:+UseG1GC".to_string());
            args.push("-XX:G1NewSizePercent=20".to_string());
            args.push("-XX:G1ReservePercent=20".to_string());
            args.push("-XX:MaxGCPauseMillis=50".to_string());
        }
        GcStrategy::Parallel => {
            args.push("-XX:+UseParallelGC".to_string());
        }
        GcStrategy::Serial => {
            args.push("-XX:+UseSerialGC".to_string());
        }
        GcStrategy::Z => {
            // ZGC 需要 Java 15+，旧版本只能回退到 G1，否则 JVM 根本起不来
            if java_major < 15 {
                args.push("-XX:+UseG1GC".to_string());
            } else {
                args.push("-XX:+UseZGC".to_string());
                // 分代 ZGC：Java 21~23 需显式开启，Java 24+ 默认分代且该标志已移除
                if java_major >= 21 && java_major < 24 {
                    args.push("-XX:+ZGenerational".to_string());
                }
            }
        }
        // 不加参数，行为等同官方启动器
        GcStrategy::Default => {}
    }

    if advanced.always_pre_touch {
        args.push("-XX:+AlwaysPreTouch".to_string());
    }

    if advanced.disable_explicit_gc {
        args.push("-XX:+DisableExplicitGC".to_string());
    }

    if advanced.string_deduplication {
        args.push("-XX:+UseStringDeduplication".to_string());
    }

    if advanced.perf_disable_shared_mem {
        args.push("-XX:+PerfDisableSharedMem".to_string());
    }

    // Metaspace 上限默认留空：不设上限会持续索取本地内存拖垮系统，限太紧又触发 Metaspace OOM
    let metaspace_initial = advanced.metaspace_size_mb.filter(|mb| *mb > 0);
    let metaspace_max = advanced.metaspace_max_size_mb.filter(|mb| *mb > 0);

    match (metaspace_initial, metaspace_max) {
        (Some(initial), Some(max)) => {
            // 初始水位不能高于上限，否则参数自相矛盾
            args.push(format!("-XX:MetaspaceSize={}m", initial.min(max)));
            args.push(format!("-XX:MaxMetaspaceSize={}m", max));
        }
        (Some(initial), None) => {
            args.push(format!("-XX:MetaspaceSize={}m", initial));
        }
        (None, Some(max)) => {
            // 只填上限时初始值对齐上限，避免加载期反复触发带 Full GC 的类卸载
            args.push(format!("-XX:MetaspaceSize={}m", max));
            args.push(format!("-XX:MaxMetaspaceSize={}m", max));
        }
        (None, None) => {}
    }

    if advanced.heap_dump_on_oom {
        args.push("-XX:+HeapDumpOnOutOfMemoryError".to_string());
        args.push("-XX:HeapDumpPath=./logs/heapdump.hprof".to_string());
    }

    // JIT 编译器线程数，加速启动时的代码编译
    args.push(format!("-XX:CICompilerCount={}", gc_threads.min(4)));

    args.extend(parse_extra_args(&advanced.extra_args));

    // 兜底：自定义参数可能带实验性选项，统一补解锁开关防止 JVM 拒绝启动
    ensure_experimental_unlock(&mut args);

    args
}

const UNLOCK_EXPERIMENTAL: &str = "-XX:+UnlockExperimentalVMOptions";

/// 解锁开关必须先于实验性选项，否则 JVM 直接拒绝创建虚拟机
fn ensure_experimental_unlock(args: &mut Vec<String>) {
    if !args.iter().any(|a| a.starts_with("-XX:")) {
        return;
    }

    // 旧位置的开关（多来自自定义参数）同样不合法，先删再插到最前
    args.retain(|a| a != UNLOCK_EXPERIMENTAL);
    args.insert(0, UNLOCK_EXPERIMENTAL.to_string());
}

/// 解析用户自定义 JVM 参数：支持换行或空格分隔，`#` 开头视为注释
fn parse_extra_args(raw: &str) -> Vec<String> {
    let mut result = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for token in line.split_whitespace() {
            result.push(token.to_string());
        }
    }

    result
}

/// 仅校验最低 512MB，不检查上限
pub fn is_memory_setting_safe(requested_memory_mb: u32) -> Result<bool, LauncherError> {
    if requested_memory_mb < 512 {
        return Err(LauncherError::Custom(
            "内存设置过低！Minecraft 至少需要 512MB 内存。".to_string(),
        ));
    }

    Ok(true)
}

pub fn get_default_auto_memory_config() -> AutoMemoryConfig {
    AutoMemoryConfig {
        enabled: false,
        max_limit_mb: DEFAULT_AUTO_MEMORY_MAX_LIMIT_MB,
        safety_margin_percent: DEFAULT_AUTO_MEMORY_SAFETY_MARGIN_PERCENT,
    }
}

pub fn should_use_auto_memory(config: &AutoMemoryConfig) -> bool {
    config.enabled
}

pub fn auto_set_memory_if_enabled(config: &AutoMemoryConfig) -> Option<u32> {
    if !config.enabled {
        return None;
    }

    let recommendation = recommend_memory_by_system(config);
    Some(recommendation.recommended_memory_mb)
}

pub fn analyze_memory_efficiency(requested_memory: u32) -> String {
    let memory_stats = get_system_memory();
    let total_memory_mb = memory_stats.total_memory_mb;
    let usage_percent = (requested_memory as f64 / total_memory_mb as f64) * 100.0;

    match usage_percent {
        p if p < 25.0 => "内存使用效率较低，可以考虑增加内存设置".to_string(),
        p if p < 50.0 => "内存使用效率良好".to_string(),
        p if p < 75.0 => "内存使用效率较高".to_string(),
        _ => "内存使用接近系统极限，建议适当降低设置".to_string(),
    }
}

pub fn is_memory_over_90_percent(requested_memory_mb: u32) -> bool {
    let memory_stats = get_system_memory();
    let warning_limit = (memory_stats.total_memory_mb as f32 * 0.9) as u32;

    requested_memory_mb > warning_limit
}

pub fn get_memory_warning_message(requested_memory_mb: u32) -> Option<String> {
    let memory_stats = get_system_memory();
    let warning_limit = (memory_stats.total_memory_mb as f32 * 0.9) as u32;

    if requested_memory_mb > warning_limit {
        Some(format!(
            "警告：内存设置 {}MB 超过系统总内存 {}MB 的90%。这可能导致系统不稳定。",
            requested_memory_mb, memory_stats.total_memory_mb
        ))
    } else {
        None
    }
}

pub fn monitor_memory_usage() -> MemoryStats {
    get_system_memory()
}

/// 连续采样间的内存趋势，用于检测内存泄漏
pub fn get_memory_trend(samples: &[MemoryStats]) -> MemoryTrend {
    if samples.len() < 2 {
        return MemoryTrend::Stable;
    }

    let first = samples.first().unwrap();
    let last = samples.last().unwrap();
    let usage_increase = last.used_memory_mb as i64 - first.used_memory_mb as i64;
    let time_span = samples.len() as u64; // 每个样本间隔 1 分钟

    if usage_increase > 200 && time_span > 10 {
        // 持续增长视为疑似泄漏
        MemoryTrend::Increasing
    } else if usage_increase < -100 {
        MemoryTrend::Decreasing
    } else {
        MemoryTrend::Stable
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemoryTrend {
    Increasing,
    Decreasing,
    Stable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_recommendation() {
        let recommendation = recommend_memory_for_game("1.20.1", false);
        assert!(recommendation.recommended_memory_mb >= 1024);
        assert!(recommendation.max_memory_mb >= recommendation.recommended_memory_mb);
    }

    #[test]
    fn test_jvm_args_generation() {
        let args = optimize_jvm_memory_args(2048, Some(17), &JvmAdvancedOptions::default());
        assert!(args.iter().any(|arg| arg.contains("-Xmx2048M")));
        assert!(args.iter().any(|arg| arg.contains("-Xms2048M")));
    }

    #[test]
    fn test_memory_recommendation_modded() {
        let rec = recommend_memory_for_game("1.20.1", true);
        let rec_vanilla = recommend_memory_for_game("1.20.1", false);
        assert!(rec.min_memory_mb >= rec_vanilla.min_memory_mb);
        assert!(rec.max_memory_mb >= rec.recommended_memory_mb);
    }

    #[test]
    fn test_memory_recommendation_old_version() {
        let rec = recommend_memory_for_game("1.7.10", false);
        assert!(rec.recommended_memory_mb >= 512);
        assert!(rec.max_memory_mb >= rec.recommended_memory_mb);
    }

    #[test]
    fn test_memory_recommendation_mid_version() {
        let rec = recommend_memory_for_game("1.16.5", false);
        assert!(rec.recommended_memory_mb >= 1024);
        assert!(rec.max_memory_mb >= rec.recommended_memory_mb);
    }

    #[test]
    fn test_jvm_args_default_has_only_g1gc() {
        let args = optimize_jvm_memory_args(2048, Some(17), &JvmAdvancedOptions::default());
        assert!(args.iter().any(|arg| arg == "-XX:+UseG1GC"));
        assert!(args.iter().any(|arg| arg == "-XX:G1NewSizePercent=20"));
        assert!(args.iter().any(|arg| arg == "-XX:G1ReservePercent=20"));
        assert!(args.iter().any(|arg| arg == "-XX:MaxGCPauseMillis=50"));
        // 实验性选项必须先解锁，否则 JVM 拒绝启动
        let unlock_pos = args
            .iter()
            .position(|arg| arg == "-XX:+UnlockExperimentalVMOptions")
            .expect("必须包含 UnlockExperimentalVMOptions");
        let g1_new_pos = args
            .iter()
            .position(|arg| arg == "-XX:G1NewSizePercent=20")
            .unwrap();
        assert!(unlock_pos < g1_new_pos, "解锁开关必须排在实验性选项之前");
        for unexpected in [
            "-XX:+AlwaysPreTouch",
            "-XX:+DisableExplicitGC",
            "-XX:+PerfDisableSharedMem",
            "-XX:+HeapDumpOnOutOfMemoryError",
            "-XX:+UseStringDeduplication",
            "-XX:+UseParallelGC",
            "-XX:+UseZGC",
        ] {
            assert!(!args.iter().any(|arg| arg == unexpected), "{}", unexpected);
        }
        assert!(!args.iter().any(|arg| arg.starts_with("-XX:MetaspaceSize")));
    }

    #[test]
    fn test_jvm_args_advanced_options_are_off_by_default() {
        let opts = JvmAdvancedOptions::default();
        assert_eq!(opts.gc_strategy, GcStrategy::G1, "默认应为 G1，与官方启动器一致");
        assert!(!opts.always_pre_touch);
        assert!(!opts.disable_explicit_gc);
        assert!(!opts.string_deduplication);
        assert!(!opts.perf_disable_shared_mem);
        assert!(!opts.heap_dump_on_oom);
        assert!(opts.metaspace_size_mb.is_none());
        assert!(opts.metaspace_max_size_mb.is_none());
        assert!(opts.extra_args.is_empty());
    }

    #[test]
    fn test_jvm_args_advanced_flags_applied_when_enabled() {
        let opts = JvmAdvancedOptions {
            always_pre_touch: true,
            disable_explicit_gc: true,
            perf_disable_shared_mem: true,
            string_deduplication: true,
            heap_dump_on_oom: true,
            metaspace_size_mb: Some(256),
            metaspace_max_size_mb: Some(512),
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(17), &opts);
        assert!(args.iter().any(|arg| arg == "-XX:+AlwaysPreTouch"));
        assert!(args.iter().any(|arg| arg == "-XX:+DisableExplicitGC"));
        assert!(args.iter().any(|arg| arg == "-XX:+PerfDisableSharedMem"));
        assert!(args.iter().any(|arg| arg == "-XX:+UseStringDeduplication"));
        assert!(args.iter().any(|arg| arg == "-XX:MetaspaceSize=256m"));
        assert!(args.iter().any(|arg| arg == "-XX:+HeapDumpOnOutOfMemoryError"));
    }

    #[test]
    fn test_jvm_args_gc_strategy_default_matches_official() {
        // Default 策略不加任何 GC 参数，与官方启动器一致
        let opts = JvmAdvancedOptions {
            gc_strategy: GcStrategy::Default,
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(17), &opts);
        for gc in [
            "-XX:+UseG1GC",
            "-XX:+UseParallelGC",
            "-XX:+UseSerialGC",
            "-XX:+UseZGC",
        ] {
            assert!(!args.iter().any(|arg| arg == gc), "{}", gc);
        }
        assert!(!args.iter().any(|arg| arg.starts_with("-XX:G1")));
    }

    #[test]
    fn test_jvm_args_gc_strategy_parallel_and_serial() {
        let parallel = JvmAdvancedOptions {
            gc_strategy: GcStrategy::Parallel,
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(2048, Some(17), &parallel);
        assert!(args.iter().any(|arg| arg == "-XX:+UseParallelGC"));
        assert!(!args.iter().any(|arg| arg == "-XX:+UseG1GC"));

        let serial = JvmAdvancedOptions {
            gc_strategy: GcStrategy::Serial,
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(2048, Some(17), &serial);
        assert!(args.iter().any(|arg| arg == "-XX:+UseSerialGC"));
        assert!(!args.iter().any(|arg| arg == "-XX:+UseG1GC"));
    }

    #[test]
    fn test_jvm_args_gc_strategy_z_selected_explicitly() {
        let opts = JvmAdvancedOptions {
            gc_strategy: GcStrategy::Z,
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(8192, Some(21), &opts);
        assert!(args.iter().any(|arg| arg == "-XX:+UseZGC"));
        assert!(args.iter().any(|arg| arg == "-XX:+ZGenerational"));
        assert!(!args.iter().any(|arg| arg == "-XX:+UseG1GC"));
    }

    #[test]
    fn test_jvm_args_gc_strategy_z_falls_back_on_old_java() {
        // Java 8 无 ZGC，必须回退 G1 而不是启动失败
        let opts = JvmAdvancedOptions {
            gc_strategy: GcStrategy::Z,
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(8), &opts);
        assert!(!args.iter().any(|arg| arg == "-XX:+UseZGC"));
        assert!(args.iter().any(|arg| arg == "-XX:+UseG1GC"));
    }

    #[test]
    fn test_jvm_args_metaspace_initial_and_max() {
        let opts = JvmAdvancedOptions {
            metaspace_size_mb: Some(256),
            metaspace_max_size_mb: Some(512),
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(17), &opts);
        assert!(args.iter().any(|arg| arg == "-XX:MetaspaceSize=256m"));
        assert!(args.iter().any(|arg| arg == "-XX:MaxMetaspaceSize=512m"));
    }

    #[test]
    fn test_jvm_args_metaspace_max_only_sets_initial_too() {
        let opts = JvmAdvancedOptions {
            metaspace_max_size_mb: Some(768),
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(17), &opts);
        assert!(args.iter().any(|arg| arg == "-XX:MetaspaceSize=768m"));
        assert!(args.iter().any(|arg| arg == "-XX:MaxMetaspaceSize=768m"));
    }

    #[test]
    fn test_jvm_args_metaspace_initial_clamped_to_max() {
        let opts = JvmAdvancedOptions {
            metaspace_size_mb: Some(1024),
            metaspace_max_size_mb: Some(512),
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(17), &opts);
        assert!(args.iter().any(|arg| arg == "-XX:MetaspaceSize=512m"));
        assert!(!args.iter().any(|arg| arg == "-XX:MetaspaceSize=1024m"));
        assert!(args.iter().any(|arg| arg == "-XX:MaxMetaspaceSize=512m"));
    }

    #[test]
    fn test_jvm_args_metaspace_zero_values_ignored() {
        let opts = JvmAdvancedOptions {
            metaspace_size_mb: Some(0),
            metaspace_max_size_mb: Some(0),
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(17), &opts);
        assert!(!args.iter().any(|arg| arg.starts_with("-XX:MetaspaceSize")));
        assert!(!args.iter().any(|arg| arg.starts_with("-XX:MaxMetaspaceSize")));
    }

    #[test]
    fn test_jvm_args_custom_experimental_arg_gets_unlock_prepended() {
        let opts = JvmAdvancedOptions {
            gc_strategy: GcStrategy::Default,
            extra_args: "-XX:+UseZGC\n-XX:+ZGenerational".to_string(),
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(21), &opts);
        assert_eq!(args.first().unwrap(), "-XX:+UnlockExperimentalVMOptions");
        assert!(args.contains(&"-XX:+UseZGC".to_string()));
    }

    #[test]
    fn test_jvm_args_no_duplicate_unlock() {
        let opts = JvmAdvancedOptions {
            extra_args: "-XX:+UnlockExperimentalVMOptions\n-XX:+UseZGC".to_string(),
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(21), &opts);
        let count = args
            .iter()
            .filter(|a| *a == "-XX:+UnlockExperimentalVMOptions")
            .count();
        assert_eq!(count, 1, "解锁开关不应重复出现");
    }

    #[test]
    fn test_jvm_args_extra_args_parsed_and_appended() {
        let opts = JvmAdvancedOptions {
            extra_args: "# 我想试试 ZGC\n-XX:+UseZGC\n-XX:+ZGenerational\n\n  -Dfoo=bar -Dbar=baz \n"
                .to_string(),
            ..JvmAdvancedOptions::default()
        };
        let args = optimize_jvm_memory_args(4096, Some(21), &opts);
        assert!(args.iter().any(|arg| arg == "-XX:+UseZGC"));
        assert!(args.iter().any(|arg| arg == "-XX:+ZGenerational"));
        assert!(args.iter().any(|arg| arg == "-Dfoo=bar"));
        assert!(args.iter().any(|arg| arg == "-Dbar=baz"));
        assert!(!args.iter().any(|arg| arg.starts_with('#')));
        // 自定义参数排在最后，便于覆盖前面的设置
        let last_index = args
            .iter()
            .rposition(|arg| arg == "-XX:+UseG1GC")
            .expect("G1GC 仍在启用");
        assert!(args[last_index..].contains(&"-XX:+UseZGC".to_string()));
    }

    #[test]
    fn test_jvm_args_enable_native_access_java17() {
        let args = optimize_jvm_memory_args(2048, Some(17), &JvmAdvancedOptions::default());
        assert!(
            args.iter()
                .any(|arg| arg == "--enable-native-access=ALL-UNNAMED")
        );
    }

    #[test]
    fn test_jvm_args_no_native_access_java8() {
        let args = optimize_jvm_memory_args(2048, Some(8), &JvmAdvancedOptions::default());
        assert!(
            !args
                .iter()
                .any(|arg| arg == "--enable-native-access=ALL-UNNAMED")
        );
    }

    #[test]
    fn test_jvm_args_unsafe_memory_access_java25() {
        // Java 23+ 需显式允许 Unsafe 内存访问
        let args = optimize_jvm_memory_args(2048, Some(25), &JvmAdvancedOptions::default());
        assert!(
            args.iter()
                .any(|arg| arg == "--sun-misc-unsafe-memory-access=allow")
        );
    }

    #[test]
    fn test_jvm_args_no_unsafe_memory_access_java21() {
        let args = optimize_jvm_memory_args(2048, Some(21), &JvmAdvancedOptions::default());
        assert!(
            !args
                .iter()
                .any(|arg| arg == "--sun-misc-unsafe-memory-access=allow")
        );
    }

    #[test]
    fn test_jvm_args_no_zgc_on_java8() {
        let args = optimize_jvm_memory_args(4096, Some(8), &JvmAdvancedOptions::default());
        assert!(!args.iter().any(|arg| arg == "-XX:+UseZGC"));
        assert!(args.iter().any(|arg| arg == "-XX:+UseG1GC"));
        assert!(
            !args
                .iter()
                .any(|arg| arg == "--enable-native-access=ALL-UNNAMED")
        );
    }

    #[test]
    fn test_memory_setting_safe() {
        assert!(is_memory_setting_safe(256).is_err());
        assert!(is_memory_setting_safe(511).is_err());
        assert!(is_memory_setting_safe(512).is_ok());
        assert!(is_memory_setting_safe(4096).is_ok());
    }

    #[test]
    fn test_analyze_memory_efficiency() {
        let result = analyze_memory_efficiency(1024);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_memory_trend_stable() {
        let samples = vec![
            MemoryStats {
                total_memory_mb: 16000,
                used_memory_mb: 8000,
                available_memory_mb: 8000,
                memory_usage_percent: 50.0,
            },
            MemoryStats {
                total_memory_mb: 16000,
                used_memory_mb: 8050,
                available_memory_mb: 7950,
                memory_usage_percent: 50.3,
            },
        ];
        assert_eq!(get_memory_trend(&samples), MemoryTrend::Stable);
    }

    #[test]
    fn test_memory_trend_insufficient_samples() {
        let samples = vec![MemoryStats {
            total_memory_mb: 16000,
            used_memory_mb: 8000,
            available_memory_mb: 8000,
            memory_usage_percent: 50.0,
        }];
        assert_eq!(get_memory_trend(&samples), MemoryTrend::Stable);
        assert_eq!(get_memory_trend(&[]), MemoryTrend::Stable);
    }
}
