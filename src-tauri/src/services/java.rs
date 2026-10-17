//! Java 检测模块
//!
//! 扫描各发行版安装目录、PATH 与 JAVA_HOME，用 java -version 确认后去重缓存 1 小时

use crate::{LauncherError, load_config, save_config};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

struct JavaCache {
    paths: Vec<String>,
    cached_at: Instant,
}

static JAVA_CACHE: std::sync::LazyLock<RwLock<Option<JavaCache>>> =
    std::sync::LazyLock::new(|| RwLock::new(None));

const JAVA_CACHE_DURATION: Duration = Duration::from_secs(3600);

pub fn invalidate_java_cache() {
    if let Ok(mut cache) = JAVA_CACHE.write() {
        *cache = None;
    }
    log::info!("Java 检测缓存已清除");
}

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

fn update_java_cache(paths: Vec<String>) {
    if let Ok(mut cache) = JAVA_CACHE.write() {
        *cache = Some(JavaCache {
            paths: paths.clone(),
            cached_at: Instant::now(),
        });
        log::info!("Java 检测缓存已更新 ({} 个路径)", paths.len());
    }
}

fn normalize_path(path: &str) -> String {
    if cfg!(windows) {
        path.replace("/", "\\")
    } else {
        path.replace("\\", "/")
    }
}

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

            // 版本信息通常在 stderr，两路都要查
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let stdout_str = String::from_utf8_lossy(&output.stdout);

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

fn find_java_in_path(java_cmd: &str) -> bool {
    is_valid_java_executable(Path::new(java_cmd))
}

fn get_java_installation_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let drives = get_windows_drives();

        for drive in &drives {
            let program_files = format!("{}:\\Program Files", drive);
            let program_files_x86 = format!("{}:\\Program Files (x86)", drive);

            dirs.push(PathBuf::from(&program_files).join("Java"));
            dirs.push(PathBuf::from(&program_files_x86).join("Java"));

            dirs.push(PathBuf::from(&program_files).join("Microsoft"));
            dirs.push(PathBuf::from(&program_files_x86).join("Microsoft"));

            dirs.push(PathBuf::from(&program_files).join("Eclipse Adoptium"));
            dirs.push(PathBuf::from(&program_files_x86).join("Eclipse Adoptium"));
            dirs.push(PathBuf::from(&program_files).join("Eclipse Foundation"));

            dirs.push(PathBuf::from(&program_files).join("AdoptOpenJDK"));
            dirs.push(PathBuf::from(&program_files_x86).join("AdoptOpenJDK"));

            dirs.push(PathBuf::from(&program_files).join("Amazon Corretto"));
            dirs.push(PathBuf::from(&program_files_x86).join("Amazon Corretto"));

            dirs.push(PathBuf::from(&program_files).join("Zulu"));
            dirs.push(PathBuf::from(&program_files_x86).join("Zulu"));
            dirs.push(PathBuf::from(&program_files).join("Azul Zulu"));

            dirs.push(PathBuf::from(&program_files).join("BellSoft"));
            dirs.push(PathBuf::from(&program_files_x86).join("BellSoft"));

            dirs.push(PathBuf::from(&program_files).join("RedHat"));
            dirs.push(PathBuf::from(&program_files_x86).join("RedHat"));

            dirs.push(PathBuf::from(&program_files).join("SapMachine"));

            dirs.push(PathBuf::from(&program_files).join("GraalVM"));

            dirs.push(PathBuf::from(&program_files).join("OpenJDK"));
            dirs.push(PathBuf::from(&program_files_x86).join("OpenJDK"));
        }

        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            dirs.push(
                PathBuf::from(&user_profile)
                    .join("scoop")
                    .join("apps")
                    .join("openjdk"),
            );
            dirs.push(
                PathBuf::from(&user_profile)
                    .join("scoop")
                    .join("apps")
                    .join("temurin-lts-jdk"),
            );
            dirs.push(
                PathBuf::from(&user_profile)
                    .join("scoop")
                    .join("apps")
                    .join("temurin8-jdk"),
            );
            dirs.push(
                PathBuf::from(&user_profile)
                    .join("scoop")
                    .join("apps")
                    .join("zulu-jdk"),
            );
            dirs.push(
                PathBuf::from(&user_profile)
                    .join("scoop")
                    .join("apps")
                    .join("corretto-jdk"),
            );

            // .jdks：IntelliJ IDEA 下载的 JDK 存放处
            dirs.push(PathBuf::from(&user_profile).join(".jdks"));

            dirs.push(
                PathBuf::from(&user_profile)
                    .join(".sdkman")
                    .join("candidates")
                    .join("java"),
            );
        }

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

        if let Ok(home) = std::env::var("HOME") {
            dirs.push(
                PathBuf::from(&home)
                    .join(".sdkman")
                    .join("candidates")
                    .join("java"),
            );
            dirs.push(PathBuf::from(&home).join(".jdks"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/lib/jvm"));
        dirs.push(PathBuf::from("/usr/local/lib/jvm"));
        dirs.push(PathBuf::from("/opt/java"));
        dirs.push(PathBuf::from("/opt/jdk"));

        if let Ok(home) = std::env::var("HOME") {
            dirs.push(
                PathBuf::from(&home)
                    .join(".sdkman")
                    .join("candidates")
                    .join("java"),
            );
            dirs.push(PathBuf::from(&home).join(".jdks"));
        }
    }

    dirs.into_iter().filter(|dir| dir.exists()).collect()
}

#[cfg(target_os = "windows")]
fn get_windows_drives() -> Vec<char> {
    let mut drives = Vec::new();

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

fn find_java_in_directory(dir: &Path) -> Vec<String> {
    let mut paths = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();

                if is_java_directory_name(&dir_name) {
                    let java_exe = entry.path().join("bin").join(if cfg!(windows) {
                        "java.exe"
                    } else {
                        "java"
                    });

                    if java_exe.exists() && is_valid_java_executable(&java_exe) {
                        let path_str = java_exe.to_string_lossy().replace("\\", "/");
                        paths.push(path_str.to_owned());
                    } else {
                        // 某些发行版（如 Eclipse Adoptium）还多一层子目录
                        paths.extend(find_java_in_subdirectory(&entry.path()));
                    }
                } else {
                    // Microsoft、Eclipse Adoptium 这类目录名本身不像 Java 目录，需再搜一层
                    paths.extend(find_java_in_subdirectory(&entry.path()));
                }
            }
        }
    }

    paths
}

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

fn find_java_in_subdirectory(dir: &Path) -> Vec<String> {
    let mut paths = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let sub_dir_name = entry.file_name().to_string_lossy().to_lowercase();

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

                    #[cfg(target_os = "macos")]
                    {
                        let macos_java = entry
                            .path()
                            .join("Contents")
                            .join("Home")
                            .join("bin")
                            .join("java");
                        if macos_java.exists() && is_valid_java_executable(&macos_java) {
                            let path_str = macos_java.to_string_lossy().replace("\\", "/");
                            paths.push(path_str.to_owned());
                        }
                    }
                }

                // scoop 用 current 目录指向实际安装的符号链接
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

/// 查找 Java 安装路径（带缓存，并行扫描）
pub async fn find_java_installations_command() -> Result<Vec<String>, LauncherError> {
    if let Some(cached_paths) = get_cached_java_paths() {
        return Ok(cached_paths);
    }

    log::info!("开始扫描 Java 安装路径...");
    let start_time = Instant::now();

    // 磁盘扫描耗时，放阻塞线程池以免卡住异步运行时
    let paths = tokio::task::spawn_blocking(|| scan_java_installations_parallel())
        .await
        .map_err(|e| LauncherError::Custom(format!("Java 扫描任务失败: {}", e)))?;

    let elapsed = start_time.elapsed();
    log::info!(
        "Java 扫描完成，耗时 {:?}，找到 {} 个安装",
        elapsed,
        paths.len()
    );

    update_java_cache(paths.clone());

    Ok(paths)
}

pub fn find_java_installations_command_sync() -> Result<Vec<String>, LauncherError> {
    if let Some(cached_paths) = get_cached_java_paths() {
        return Ok(cached_paths);
    }
    Ok(scan_java_installations_parallel())
}

/// 并行扫描 Java 安装，须在阻塞线程池中调用
fn scan_java_installations_parallel() -> Vec<String> {
    let java_dirs = get_java_installation_dirs();

    let dir_results: Vec<Vec<String>> = std::thread::scope(|s| {
        let handles: Vec<_> = java_dirs
            .iter()
            .map(|dir| s.spawn(|| find_java_in_directory(dir)))
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });

    let mut paths: Vec<String> = dir_results.into_iter().flatten().collect();

    if let Ok(path_env) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        let path_entries: Vec<&str> = path_env.split(separator).collect();

        let path_results: Vec<String> = std::thread::scope(|s| {
            let handles: Vec<_> = path_entries
                .iter()
                .map(|entry| {
                    s.spawn(|| {
                        let path_buf = PathBuf::from(*entry);
                        let java_exe =
                            path_buf.join(if cfg!(windows) { "java.exe" } else { "java" });

                        if java_exe.exists() && is_valid_java_executable(&java_exe) {
                            Some(java_exe.to_string_lossy().replace("\\", "/"))
                        } else {
                            None
                        }
                    })
                })
                .collect();
            handles
                .into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .collect()
        });

        paths.extend(path_results);
    }

    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_home_path = PathBuf::from(&java_home);
        let java_exe =
            java_home_path
                .join("bin")
                .join(if cfg!(windows) { "java.exe" } else { "java" });

        if java_exe.exists() && is_valid_java_executable(&java_exe) {
            let java_exe_path = java_exe.to_string_lossy().replace("\\", "/");
            if !paths.contains(&java_exe_path) {
                paths.push(java_exe_path);
            }
        }
    }

    for env_var in &[
        "JDK_HOME",
        "JRE_HOME",
        "JAVA_8_HOME",
        "JAVA_11_HOME",
        "JAVA_17_HOME",
        "JAVA_21_HOME",
    ] {
        if let Ok(java_path) = std::env::var(env_var) {
            let java_path_buf = PathBuf::from(&java_path);
            let java_exe =
                java_path_buf
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

    let mut unique_paths = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for path in paths {
        let normalized = match PathBuf::from(&path).canonicalize() {
            Ok(canonical) => canonical
                .to_string_lossy()
                .to_lowercase()
                .replace("\\", "/"),
            Err(_) => path.to_lowercase(),
        };

        if !seen_paths.contains(&normalized) {
            seen_paths.insert(normalized);
            unique_paths.push(path);
        }
    }

    unique_paths.sort();
    unique_paths
}

pub async fn refresh_java_installations() -> Result<Vec<String>, LauncherError> {
    invalidate_java_cache();
    find_java_installations_command().await
}

pub async fn set_java_path_command(path: String) -> Result<(), LauncherError> {
    let normalized_path = normalize_path(&path);

    if !validate_java_path(path.clone()).await? {
        return Err(LauncherError::Custom(format!(
            "无效的Java路径或Java版本: {}",
            normalized_path
        )));
    }

    let mut config = (*load_config()?).clone();
    config.java_path = Some(normalized_path);
    save_config(&config)?;

    Ok(())
}

/// 验证 Java 路径，支持相对路径
pub async fn validate_java_path(path: String) -> Result<bool, LauncherError> {
    if path == "java" || path == "java.exe" {
        let java_cmd = if cfg!(windows) { "java.exe" } else { "java" };
        return Ok(find_java_in_path(java_cmd));
    }

    let path_buf = crate::services::launcher::java::resolve_path(&path);

    if path_buf.is_file() {
        Ok(is_valid_java_executable(&path_buf))
    } else if path_buf.is_dir() {
        let java_exe = path_buf
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });
        Ok(java_exe.exists() && is_valid_java_executable(&java_exe))
    } else {
        Ok(false)
    }
}

/// 获取 Java 版本信息，支持相对路径
pub async fn get_java_version(path: String) -> Result<String, LauncherError> {
    let resolved = crate::services::launcher::java::resolve_path(&path);

    let java_path = if path == "java" || path == "java.exe" {
        PathBuf::from(&path)
    } else if resolved.is_dir() {
        resolved
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" })
    } else {
        resolved
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
            let version_output = if stderr.contains("version") {
                stderr
            } else {
                stdout
            };

            for line in version_output.lines() {
                if line.contains("version") {
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line[start + 1..].find('"') {
                            return Ok(line[start + 1..start + 1 + end].to_string());
                        }
                    }
                    return Ok(line.trim().to_string());
                }
            }

            Err(LauncherError::Custom("无法解析 Java 版本".to_string()))
        }
        Err(e) => Err(LauncherError::Custom(format!("无法获取 Java 版本: {}", e))),
    }
}

/// 自动检测 Java 安装（同步，供配置加载使用）
pub fn auto_detect_java() -> Result<Vec<String>, LauncherError> {
    let mut java_paths = Vec::new();

    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_exe = if cfg!(windows) { "java.exe" } else { "java" };
        let java_path = PathBuf::from(&java_home).join("bin").join(java_exe);

        if java_path.exists() && is_valid_java_executable(&java_path) {
            java_paths.push(java_path.to_string_lossy().into_owned());
        }
    }

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

    let mut unique_paths = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for path in java_paths {
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
