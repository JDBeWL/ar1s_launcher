use crate::{load_config, save_config, LauncherError};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

// Java 检测结果缓存
struct JavaCache {
    paths: Vec<String>,
    cached_at: Instant,
}

static JAVA_CACHE: std::sync::LazyLock<RwLock<Option<JavaCache>>> =
    std::sync::LazyLock::new(|| RwLock::new(None));

// 缓存有效期：1小时
const JAVA_CACHE_DURATION: Duration = Duration::from_secs(3600);

/// 清除 Java 检测缓存（供外部模块在需要时调用，如用户手动刷新）
pub fn invalidate_java_cache() {
    if let Ok(mut cache) = JAVA_CACHE.write() {
        *cache = None;
    }
    log::info!("Java 检测缓存已清除");
}

/// 检查缓存是否有效
fn get_cached_java_paths() -> Option<Vec<String>> {
    if let Ok(cache) = JAVA_CACHE.read() {
        if let Some(ref cached) = *cache {
            if cached.cached_at.elapsed() < JAVA_CACHE_DURATION {
                log::debug!("使用缓存的 Java 路径列表 ({} 个)", cached.paths.len());
                return Some(cached.paths.clone());
            }
        }
    }
    None
}

/// 更新缓存
fn update_java_cache(paths: Vec<String>) {
    if let Ok(mut cache) = JAVA_CACHE.write() {
        *cache = Some(JavaCache {
            paths: paths.clone(),
            cached_at: Instant::now(),
        });
        log::info!("Java 检测缓存已更新 ({} 个路径)", paths.len());
    }
}

/// 标准化路径格式
fn normalize_path(path: &str) -> String {
    if cfg!(windows) {
        path.replace("/", "\\")
    } else {
        path.replace("\\", "/")
    }
}

/// 检查Java可执行文件是否有效
fn is_valid_java_executable(java_path: &Path) -> bool {
    if !java_path.exists() {
        return false;
    }

    let mut command = Command::new(java_path);
    command.arg("-version");

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    match command.output() {
        Ok(output) => {
            if !output.status.success() {
                return false;
            }

            // 检查输出中是否包含Java版本信息
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let stdout_str = String::from_utf8_lossy(&output.stdout);

            // 检查常见的Java版本标识
            stderr_str.contains("java version")
                || stderr_str.contains("openjdk version")
                || stderr_str.contains("Java(TM)")
                || stdout_str.contains("java version")
                || stdout_str.contains("openjdk version")
                || stdout_str.contains("Java(TM)")
        }
        Err(_) => false,
    }
}

/// 检查PATH中是否存在Java命令
fn find_java_in_path(java_cmd: &str) -> bool {
    let mut command = Command::new(java_cmd);
    command.arg("-version");

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command.output().is_ok() && is_valid_java_executable(Path::new(java_cmd))
}

/// 获取平台特定的Java安装目录
fn get_java_installation_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // 获取所有逻辑驱动器
        let drives = get_windows_drives();
        
        for drive in &drives {
            // Program Files 目录
            let program_files = format!("{}:\\Program Files", drive);
            let program_files_x86 = format!("{}:\\Program Files (x86)", drive);
            
            // Oracle Java
            dirs.push(PathBuf::from(&program_files).join("Java"));
            dirs.push(PathBuf::from(&program_files_x86).join("Java"));
            
            // Microsoft OpenJDK
            dirs.push(PathBuf::from(&program_files).join("Microsoft"));
            dirs.push(PathBuf::from(&program_files_x86).join("Microsoft"));
            
            // Eclipse Adoptium (Temurin)
            dirs.push(PathBuf::from(&program_files).join("Eclipse Adoptium"));
            dirs.push(PathBuf::from(&program_files_x86).join("Eclipse Adoptium"));
            dirs.push(PathBuf::from(&program_files).join("Eclipse Foundation"));
            
            // AdoptOpenJDK (旧版)
            dirs.push(PathBuf::from(&program_files).join("AdoptOpenJDK"));
            dirs.push(PathBuf::from(&program_files_x86).join("AdoptOpenJDK"));
            
            // Amazon Corretto
            dirs.push(PathBuf::from(&program_files).join("Amazon Corretto"));
            dirs.push(PathBuf::from(&program_files_x86).join("Amazon Corretto"));
            
            // Azul Zulu
            dirs.push(PathBuf::from(&program_files).join("Zulu"));
            dirs.push(PathBuf::from(&program_files_x86).join("Zulu"));
            dirs.push(PathBuf::from(&program_files).join("Azul Zulu"));
            
            // BellSoft Liberica
            dirs.push(PathBuf::from(&program_files).join("BellSoft"));
            dirs.push(PathBuf::from(&program_files_x86).join("BellSoft"));
            
            // Red Hat OpenJDK
            dirs.push(PathBuf::from(&program_files).join("RedHat"));
            dirs.push(PathBuf::from(&program_files_x86).join("RedHat"));
            
            // SAP SapMachine
            dirs.push(PathBuf::from(&program_files).join("SapMachine"));
            
            // GraalVM
            dirs.push(PathBuf::from(&program_files).join("GraalVM"));
            
            // 通用 JDK 目录
            dirs.push(PathBuf::from(&program_files).join("OpenJDK"));
            dirs.push(PathBuf::from(&program_files_x86).join("OpenJDK"));
        }
        
        // 用户目录下的 Java
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            // scoop 安装的 Java
            dirs.push(PathBuf::from(&user_profile).join("scoop").join("apps").join("openjdk"));
            dirs.push(PathBuf::from(&user_profile).join("scoop").join("apps").join("temurin-lts-jdk"));
            dirs.push(PathBuf::from(&user_profile).join("scoop").join("apps").join("temurin8-jdk"));
            dirs.push(PathBuf::from(&user_profile).join("scoop").join("apps").join("zulu-jdk"));
            dirs.push(PathBuf::from(&user_profile).join("scoop").join("apps").join("corretto-jdk"));
            
            // .jdks 目录 (IntelliJ IDEA 下载的 JDK)
            dirs.push(PathBuf::from(&user_profile).join(".jdks"));
            
            // .sdkman (SDKMAN)
            dirs.push(PathBuf::from(&user_profile).join(".sdkman").join("candidates").join("java"));
        }
        
        // 使用环境变量获取的 Program Files
        if let Ok(pf) = std::env::var("ProgramFiles") {
            if !dirs.iter().any(|d| d.starts_with(&pf)) {
                dirs.push(PathBuf::from(&pf).join("Java"));
            }
        }
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            if !dirs.iter().any(|d| d.starts_with(&pf86)) {
                dirs.push(PathBuf::from(&pf86).join("Java"));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        dirs.push(PathBuf::from("/System/Library/Java/JavaVirtualMachines"));
        dirs.push(PathBuf::from("/usr/local/Cellar/openjdk"));
        dirs.push(PathBuf::from("/opt/homebrew/Cellar/openjdk"));
        
        // 用户目录
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(&home).join(".sdkman").join("candidates").join("java"));
            dirs.push(PathBuf::from(&home).join(".jdks"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/lib/jvm"));
        dirs.push(PathBuf::from("/usr/local/lib/jvm"));
        dirs.push(PathBuf::from("/opt/java"));
        dirs.push(PathBuf::from("/opt/jdk"));
        
        // 用户目录
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(&home).join(".sdkman").join("candidates").join("java"));
            dirs.push(PathBuf::from(&home).join(".jdks"));
        }
    }

    dirs.into_iter().filter(|dir| dir.exists()).collect()
}

/// 获取 Windows 系统上的所有逻辑驱动器
#[cfg(target_os = "windows")]
fn get_windows_drives() -> Vec<char> {
    let mut drives = Vec::new();
    
    // 检查 A-Z 驱动器
    for letter in b'A'..=b'Z' {
        let drive_path = format!("{}:\\", letter as char);
        if PathBuf::from(&drive_path).exists() {
            drives.push(letter as char);
        }
    }
    
    drives
}

#[cfg(not(target_os = "windows"))]
fn get_windows_drives() -> Vec<char> {
    Vec::new()
}

/// 在指定目录中查找Java安装
fn find_java_in_directory(dir: &Path) -> Vec<String> {
    let mut paths = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();

                // 检查是否为Java安装目录
                if is_java_directory_name(&dir_name) {
                    // 直接检查 bin/java
                    let java_exe = entry.path().join("bin").join(if cfg!(windows) {
                        "java.exe"
                    } else {
                        "java"
                    });

                    if java_exe.exists() && is_valid_java_executable(&java_exe) {
                        let path_str = java_exe.to_string_lossy().replace("\\", "/");
                        paths.push(path_str.to_owned());
                    } else {
                        // 某些发行版有额外的子目录层级 (如 Eclipse Adoptium)
                        // 检查 Contents/Home/bin/java (macOS) 或直接子目录
                        paths.extend(find_java_in_subdirectory(&entry.path()));
                    }
                } else {
                    // 对于 Microsoft、Eclipse Adoptium 等目录，需要递归搜索一层
                    paths.extend(find_java_in_subdirectory(&entry.path()));
                }
            }
        }
    }

    paths
}

/// 检查目录名是否可能是 Java 安装目录
fn is_java_directory_name(name: &str) -> bool {
    name.contains("jdk") 
        || name.contains("jre") 
        || name.contains("java")
        || name.contains("openjdk")
        || name.contains("temurin")
        || name.contains("corretto")
        || name.contains("zulu")
        || name.contains("liberica")
        || name.contains("sapmachine")
        || name.contains("graalvm")
        || name.contains("semeru")
        || name.contains("dragonwell")
        || name.contains("bisheng")
}

/// 在子目录中查找 Java（处理额外的目录层级）
fn find_java_in_subdirectory(dir: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let sub_dir_name = entry.file_name().to_string_lossy().to_lowercase();
                
                // 检查子目录是否是 Java 目录
                if is_java_directory_name(&sub_dir_name) {
                    let java_exe = entry.path().join("bin").join(if cfg!(windows) {
                        "java.exe"
                    } else {
                        "java"
                    });

                    if java_exe.exists() && is_valid_java_executable(&java_exe) {
                        let path_str = java_exe.to_string_lossy().replace("\\", "/");
                        paths.push(path_str.to_owned());
                    }
                    
                    // macOS 的 Contents/Home 结构
                    #[cfg(target_os = "macos")]
                    {
                        let macos_java = entry.path().join("Contents").join("Home").join("bin").join("java");
                        if macos_java.exists() && is_valid_java_executable(&macos_java) {
                            let path_str = macos_java.to_string_lossy().replace("\\", "/");
                            paths.push(path_str.to_owned());
                        }
                    }
                }
                
                // scoop 的 current 符号链接
                if sub_dir_name == "current" {
                    let java_exe = entry.path().join("bin").join(if cfg!(windows) {
                        "java.exe"
                    } else {
                        "java"
                    });

                    if java_exe.exists() && is_valid_java_executable(&java_exe) {
                        let path_str = java_exe.to_string_lossy().replace("\\", "/");
                        paths.push(path_str.to_owned());
                    }
                }
            }
        }
    }
    
    paths
}

/// 查找Java安装路径（带缓存，并行扫描）
pub async fn find_java_installations_command() -> Result<Vec<String>, LauncherError> {
    // 先检查缓存
    if let Some(cached_paths) = get_cached_java_paths() {
        return Ok(cached_paths);
    }

    log::info!("开始扫描 Java 安装路径...");
    let start_time = Instant::now();

    // 使用 spawn_blocking 将 CPU 密集型操作移到阻塞线程池
    let paths = tokio::task::spawn_blocking(|| {
        scan_java_installations_parallel()
    }).await.map_err(|e| LauncherError::Custom(format!("Java 扫描任务失败: {}", e)))?;

    let elapsed = start_time.elapsed();
    log::info!("Java 扫描完成，耗时 {:?}，找到 {} 个安装", elapsed, paths.len());

    // 更新缓存
    update_java_cache(paths.clone());

    Ok(paths)
}

/// 并行扫描 Java 安装（同步函数，在阻塞线程池中执行）
fn scan_java_installations_parallel() -> Vec<String> {
    let java_dirs = get_java_installation_dirs();
    
    // 1. 并行扫描所有 Java 安装目录
    let mut paths: Vec<String> = java_dirs
        .par_iter()
        .flat_map(|dir| find_java_in_directory(dir))
        .collect();

    // 2. 从 PATH 环境变量中查找 Java
    if let Ok(path_env) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        let path_entries: Vec<&str> = path_env.split(separator).collect();
        
        let path_java: Vec<String> = path_entries
            .par_iter()
            .filter_map(|path_entry| {
                let path_buf = PathBuf::from(path_entry);
                let java_exe = path_buf.join(if cfg!(windows) { "java.exe" } else { "java" });
                
                if java_exe.exists() && is_valid_java_executable(&java_exe) {
                    Some(java_exe.to_string_lossy().replace("\\", "/"))
                } else {
                    None
                }
            })
            .collect();
        
        paths.extend(path_java);
    }

    // 3. 检查 JAVA_HOME 环境变量
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_home_path = PathBuf::from(&java_home);
        let java_exe = java_home_path
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });

        if java_exe.exists() && is_valid_java_executable(&java_exe) {
            let java_exe_path = java_exe.to_string_lossy().replace("\\", "/");
            if !paths.contains(&java_exe_path) {
                paths.push(java_exe_path);
            }
        }
    }
    
    // 4. 检查其他 Java 相关环境变量
    for env_var in &["JDK_HOME", "JRE_HOME", "JAVA_8_HOME", "JAVA_11_HOME", "JAVA_17_HOME", "JAVA_21_HOME"] {
        if let Ok(java_path) = std::env::var(env_var) {
            let java_path_buf = PathBuf::from(&java_path);
            let java_exe = java_path_buf
                .join("bin")
                .join(if cfg!(windows) { "java.exe" } else { "java" });

            if java_exe.exists() && is_valid_java_executable(&java_exe) {
                let java_exe_path = java_exe.to_string_lossy().replace("\\", "/");
                if !paths.contains(&java_exe_path) {
                    paths.push(java_exe_path);
                }
            }
        }
    }

    // 去重（基于规范化路径）
    let mut unique_paths = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for path in paths {
        let normalized = match PathBuf::from(&path).canonicalize() {
            Ok(canonical) => canonical.to_string_lossy().to_lowercase().replace("\\", "/"),
            Err(_) => path.to_lowercase(),
        };

        if !seen_paths.contains(&normalized) {
            seen_paths.insert(normalized);
            unique_paths.push(path);
        }
    }

    // 按路径排序
    unique_paths.sort();
    unique_paths
}

/// 强制刷新 Java 安装路径（忽略缓存）
pub async fn refresh_java_installations() -> Result<Vec<String>, LauncherError> {
    invalidate_java_cache();
    find_java_installations_command().await
}

/// 设置Java路径
pub async fn set_java_path_command(path: String) -> Result<(), LauncherError> {
    let normalized_path = normalize_path(&path);

    // 验证路径是否有效
    if !validate_java_path(path.clone()).await? {
        return Err(LauncherError::Custom(format!(
            "无效的Java路径或Java版本: {}",
            normalized_path
        )));
    }

    let mut config = load_config()?;
    config.java_path = Some(normalized_path);
    save_config(&config)?;

    Ok(())
}

/// 验证Java路径是否有效
pub async fn validate_java_path(path: String) -> Result<bool, LauncherError> {
    let path_buf = PathBuf::from(&path);

    // 如果是相对路径或命令名称（如"java"）
    if path == "java" || (!path_buf.is_absolute() && !path.contains(std::path::MAIN_SEPARATOR)) {
        let java_cmd = if cfg!(windows) { "java.exe" } else { "java" };
        return Ok(find_java_in_path(java_cmd));
    }

    // 如果是绝对路径
    if path_buf.is_file() {
        Ok(is_valid_java_executable(&path_buf))
    } else if path_buf.is_dir() {
        // 如果是目录，尝试查找bin目录下的java可执行文件
        let java_exe = path_buf
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });
        Ok(java_exe.exists() && is_valid_java_executable(&java_exe))
    } else {
        Ok(false)
    }
}

/// 获取 Java 版本信息
pub async fn get_java_version(path: String) -> Result<String, LauncherError> {
    let path_buf = PathBuf::from(&path);
    
    let java_path = if path == "java" || path == "java.exe" {
        PathBuf::from(&path)
    } else if path_buf.is_dir() {
        path_buf.join("bin").join(if cfg!(windows) { "java.exe" } else { "java" })
    } else {
        path_buf
    };

    let mut command = Command::new(&java_path);
    command.arg("-version");

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    match command.output() {
        Ok(output) => {
            // Java 版本信息通常输出到 stderr
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let version_output = if stderr.contains("version") { stderr } else { stdout };

            // 提取版本号
            for line in version_output.lines() {
                if line.contains("version") {
                    // 提取引号中的版本号
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start + 1..].find('"') {
                            return Ok(line[start + 1..start + 1 + end].to_string());
                        }
                    }
                    // 如果没有引号，返回整行
                    return Ok(line.trim().to_string());
                }
            }
            
            Err(LauncherError::Custom("无法解析 Java 版本".to_string()))
        }
        Err(e) => Err(LauncherError::Custom(format!("无法获取 Java 版本: {}", e))),
    }
}

/// 自动检测Java安装 (同步版本，用于配置加载)
pub fn auto_detect_java() -> Result<Vec<String>, LauncherError> {
    let mut java_paths = Vec::new();

    // 1. 检查JAVA_HOME环境变量（优先）
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_exe = if cfg!(windows) { "java.exe" } else { "java" };
        let java_path = PathBuf::from(&java_home).join("bin").join(java_exe);

        if java_path.exists() && is_valid_java_executable(&java_path) {
            java_paths.push(java_path.to_string_lossy().into_owned());
        }
    }

    // 2. 检查常见的Java安装目录
    #[cfg(target_os = "windows")]
    {
        let program_files =
            std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
        let program_files_x86 = std::env::var("ProgramFiles(x86)")
            .unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());

        let common_dirs = vec![program_files, program_files_x86];

        for base_dir in common_dirs {
            if let Ok(entries) = std::fs::read_dir(&base_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                            if dir_name.contains("java")
                                || dir_name.contains("jdk")
                                || dir_name.contains("jre")
                            {
                                let java_exe = entry.path().join("bin").join("java.exe");
                                if java_exe.exists() && is_valid_java_executable(&java_exe) {
                                    java_paths.push(java_exe.to_string_lossy().into_owned());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. 从 PATH 环境变量中查找 Java（获取完整路径）
    if let Ok(path_env) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for path_entry in path_env.split(separator) {
            let path_buf = PathBuf::from(path_entry);
            let java_exe = path_buf.join(if cfg!(windows) { "java.exe" } else { "java" });
            
            if java_exe.exists() && is_valid_java_executable(&java_exe) {
                let path_str = java_exe.to_string_lossy().into_owned();
                if !java_paths.contains(&path_str) {
                    java_paths.push(path_str);
                }
            }
        }
    }

    // 去重
    let mut unique_paths = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for path in java_paths {
        // 尝试规范化路径进行比较
        let normalized = match PathBuf::from(&path).canonicalize() {
            Ok(canonical) => canonical.to_string_lossy().to_lowercase(),
            Err(_) => path.to_lowercase(),
        };
        
        if !seen.contains(&normalized) {
            seen.insert(normalized);
            unique_paths.push(path);
        }
    }

    Ok(unique_paths)
}
