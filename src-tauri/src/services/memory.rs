use crate::errors::LauncherError;
use sysinfo::{System, MemoryRefreshKind};
use std::sync::Mutex;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

lazy_static! {
    static ref MEMORY_SYSTEM: Mutex<System> = Mutex::new(System::new());
}

/// 内存使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub available_memory_mb: u64,
    pub memory_usage_percent: f64,
}

/// 游戏内存推荐配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecommendation {
    pub min_memory_mb: u32,
    pub recommended_memory_mb: u32,
    pub max_memory_mb: u32,
    pub reason: String,
}

/// 自动内存推荐配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoMemoryConfig {
    pub enabled: bool,
    pub max_limit_mb: u32, // 最大内存限制（如8500MB）
    pub safety_margin_percent: f32, // 安全余量百分比
}

/// 获取系统内存信息
pub fn get_system_memory() -> MemoryStats {
    let mut system = MEMORY_SYSTEM.lock().unwrap();
    system.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());
    
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

/// 根据系统配置和游戏版本推荐内存
pub fn recommend_memory_for_game(version: &str, modded: bool) -> MemoryRecommendation {
    let memory_stats = get_system_memory();
    let total_memory_mb = memory_stats.total_memory_mb as u32;
    
    // 基础内存需求
    let base_memory = if version.starts_with("1.17") || version.starts_with("1.18") || 
                         version.starts_with("1.19") || version.starts_with("1.20") ||
                         version.starts_with("1.21") {
        // 新版本需要更多内存
        2048
    } else if version.starts_with("1.12") || version.starts_with("1.13") || 
               version.starts_with("1.14") || version.starts_with("1.15") || 
               version.starts_with("1.16") {
        // 中等版本
        1536
    } else {
        // 旧版本
        1024
    };
    
    // 模组调整
    let mod_adjustment = if modded { 1024 } else { 0 };
    
    let base_need = base_memory + mod_adjustment;
    
    // 计算推荐值
    let min_memory = base_need.max(512); // 最小512MB
    let recommended = calculate_recommended_memory(total_memory_mb, base_need);
    let max_memory = calculate_max_memory(total_memory_mb, base_need);
    
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

/// 基于系统内存大小的智能推荐（不依赖游戏类型）
pub fn recommend_memory_by_system(config: &AutoMemoryConfig) -> MemoryRecommendation {
    let memory_stats = get_system_memory();
    let total_memory_mb = memory_stats.total_memory_mb as u32;
    let available_memory_mb = memory_stats.available_memory_mb as u32;
    
    // 计算基于可用内存的推荐值
    let recommended = calculate_smart_memory(total_memory_mb, available_memory_mb, config);
    
    // 确保不超过最大限制
    let recommended = recommended.min(config.max_limit_mb);
    
    // 最小内存512MB
    let min_memory = 512;
    
    // 最大内存不超过系统内存的70%
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

/// 智能计算内存（基于可用内存百分比）
fn calculate_smart_memory(total_memory: u32, available_memory: u32, config: &AutoMemoryConfig) -> u32 {
    // 计算安全可用内存（减去安全余量）
    let safe_available = (available_memory as f32 * (1.0 - config.safety_margin_percent / 100.0)) as u32;
    
    // 根据系统总内存大小采用不同的策略
    match total_memory {
        // 4GB以下系统：使用可用内存的60%
        t if t <= 4096 => (safe_available as f32 * 0.6) as u32,
        
        // 4-8GB系统：使用可用内存的50%
        t if t <= 8192 => (safe_available as f32 * 0.5) as u32,
        
        // 8-16GB系统：使用可用内存的40%
        t if t <= 16384 => (safe_available as f32 * 0.4) as u32,
        
        // 16GB以上系统：使用可用内存的35%
        _ => (safe_available as f32 * 0.35) as u32,
    }
    .max(1024) // 最小1024MB
    .min(total_memory) // 不超过总内存
}

/// 计算推荐内存
fn calculate_recommended_memory(total_memory: u32, base_need: u32) -> u32 {
    if total_memory <= 4096 {
        // 4GB以下系统：使用系统内存的40%
        (total_memory as f32 * 0.4) as u32
    } else if total_memory <= 8192 {
        // 4-8GB系统：使用2-3GB
        base_need.max(2048).min(3072)
    } else if total_memory <= 16384 {
        // 8-16GB系统：使用3-6GB
        base_need.max(3072).min(6144)
    } else {
        // 16GB以上系统：使用4-8GB
        base_need.max(4096).min(8192)
    }
}

/// 计算最大内存
fn calculate_max_memory(total_memory: u32, base_need: u32) -> u32 {
    let max_safe = (total_memory as f32 * 0.7) as u32; // 不超过系统内存的70%
    base_need.max(1024).min(max_safe)
}

/// 优化JVM内存参数
pub fn optimize_jvm_memory_args(memory_mb: u32, version: &str) -> Vec<String> {
    let mut args = Vec::new();
    
    // 基础内存参数
    args.push(format!("-Xmx{}M", memory_mb));
    args.push(format!("-Xms{}M", memory_mb / 2)); // 初始堆大小为最大堆的一半
    
    // 垃圾回收优化
    if version.starts_with("1.17") || version.starts_with("1.18") || 
       version.starts_with("1.19") || version.starts_with("1.20") ||
       version.starts_with("1.21") {
        // 新版本使用G1GC
        args.push("-XX:+UseG1GC".to_string());
        args.push("-XX:G1HeapRegionSize=4M".to_string());
        args.push("-XX:+UnlockExperimentalVMOptions".to_string());
        args.push("-XX:G1NewSizePercent=20".to_string());
        args.push("-XX:G1ReservePercent=20".to_string());
        args.push("-XX:MaxGCPauseMillis=50".to_string());
        args.push("-XX:G1HeapWastePercent=5".to_string());
    } else {
        // 旧版本使用并行GC
        args.push("-XX:+UseParallelGC".to_string());
        args.push("-XX:ParallelGCThreads=2".to_string());
    }
    
    // 通用优化参数
    args.push("-XX:+AlwaysPreTouch".to_string());
    args.push("-XX:+DisableExplicitGC".to_string());
    args.push("-XX:+UseCompressedOops".to_string());
    args.push("-XX:-UseAdaptiveSizePolicy".to_string());
    
    // 内存溢出时生成堆转储
    args.push("-XX:+HeapDumpOnOutOfMemoryError".to_string());
    args.push("-XX:HeapDumpPath=./logs/heapdump.hprof".to_string());
    
    args
}

/// 检查内存设置是否安全（只检查最低限制，不限制上限）
pub fn is_memory_setting_safe(requested_memory_mb: u32) -> Result<bool, LauncherError> {
    if requested_memory_mb < 512 {
        return Err(LauncherError::Custom(
            "内存设置过低！Minecraft 至少需要 512MB 内存。".to_string()
        ));
    }
    
    Ok(true)
}

/// 获取默认的自动内存配置
pub fn get_default_auto_memory_config() -> AutoMemoryConfig {
    AutoMemoryConfig {
        enabled: false,
        max_limit_mb: 8500, // 整合包优化模组要求的最大限制
        safety_margin_percent: 10.0, // 保留10%的安全余量
    }
}

/// 检查是否应该使用自动内存推荐
pub fn should_use_auto_memory(config: &AutoMemoryConfig) -> bool {
    config.enabled
}

/// 自动设置内存（如果启用自动设置）
pub fn auto_set_memory_if_enabled(config: &AutoMemoryConfig) -> Option<u32> {
    if !config.enabled {
        return None;
    }
    
    let recommendation = recommend_memory_by_system(config);
    Some(recommendation.recommended_memory_mb)
}

/// 获取内存使用效率分析
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

/// 检查内存设置是否超过系统90%（用于前端警告）
pub fn is_memory_over_90_percent(requested_memory_mb: u32) -> bool {
    let memory_stats = get_system_memory();
    let warning_limit = (memory_stats.total_memory_mb as f32 * 0.9) as u32; // 超过系统内存的90%
    
    requested_memory_mb > warning_limit
}

/// 获取内存设置警告信息（用于前端显示）
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

/// 监控内存使用情况（需要定期调用）
pub fn monitor_memory_usage() -> MemoryStats {
    get_system_memory()
}

/// 获取内存使用趋势（用于检测内存泄漏）
pub fn get_memory_trend(samples: &[MemoryStats]) -> MemoryTrend {
    if samples.len() < 2 {
        return MemoryTrend::Stable;
    }
    
    let first = samples.first().unwrap();
    let last = samples.last().unwrap();
    let usage_increase = last.used_memory_mb as i64 - first.used_memory_mb as i64;
    let time_span = samples.len() as u64; // 假设每个样本间隔1分钟
    
    if usage_increase > 200 && time_span > 10 {
        // 10分钟内内存增加超过200MB，可能内存泄漏
        MemoryTrend::Increasing
    } else if usage_increase < -100 {
        // 内存使用减少
        MemoryTrend::Decreasing
    } else {
        MemoryTrend::Stable
    }
}

/// 内存使用趋势
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryTrend {
    Increasing,  // 内存使用增加
    Decreasing,  // 内存使用减少
    Stable,      // 内存使用稳定
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
        let args = optimize_jvm_memory_args(2048, "1.20.1");
        assert!(args.iter().any(|arg| arg.contains("-Xmx2048M")));
        assert!(args.iter().any(|arg| arg.contains("-Xms1024M")));
    }
}