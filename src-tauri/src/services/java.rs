use crate::{load_config, save_config, LauncherError};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

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
            stderr_str.contains("java version") ||
            stderr_str.contains("openjdk version") ||
            stderr_str.contains("Java(TM)") ||
            stdout_str.contains("java version") ||
            stdout_str.contains("openjdk version") ||
            stdout_str.contains("Java(TM)")
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
        let program_files = std::env::var("ProgramFiles")
            .unwrap_or_else(|_| r"C:\\Program Files".into());
        let program_files_x86 = std::env::var("ProgramFiles(x86)")
            .unwrap_or_else(|_| r"C:\\Program Files (x86)".into());
        
        dirs.push(PathBuf::from(program_files).join("Java"));
        dirs.push(PathBuf::from(program_files_x86).join("Java"));
        dirs.push(PathBuf::from(r"C:\\Program Files\\Java"));
        dirs.push(PathBuf::from(r"C:\\Program Files (x86)\\Java"));
    }
    
    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        dirs.push(PathBuf::from("/System/Library/Java/JavaVirtualMachines"));
        dirs.push(PathBuf::from("/usr/local/Cellar/openjdk"));
    }
    
    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/lib/jvm"));
        dirs.push(PathBuf::from("/usr/local/lib/jvm"));
        dirs.push(PathBuf::from("/opt/java"));
    }
    
    dirs.into_iter().filter(|dir| dir.exists()).collect()
}

/// 在指定目录中查找Java安装
fn find_java_in_directory(dir: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                
                // 检查是否为Java安装目录
                if dir_name.contains("jdk") || dir_name.contains("jre") || dir_name.contains("java") {
                    let java_exe = entry.path().join("bin").join(
                        if cfg!(windows) { "java.exe" } else { "java" }
                    );
                    
                    if java_exe.exists() && is_valid_java_executable(&java_exe) {
                        // 使用原始路径，避免canonicalize产生的特殊格式
                        let path_str = java_exe.to_string_lossy().replace("\\", "/");
                        paths.push(path_str.to_owned());
                    }
                }
            }
        }
    }
    
    paths
}

/// 查找Java安装路径
pub async fn find_java_installations_command() -> Result<Vec<String>, LauncherError> {
    let mut paths = Vec::new();
    
    // 1. 检查系统Java安装目录
    for java_dir in get_java_installation_dirs() {
        paths.extend(find_java_in_directory(&java_dir));
    }
    
    // 2. 检查PATH环境变量中的Java
    let path_java = if cfg!(windows) { "java.exe" } else { "java" };
    if find_java_in_path(path_java) {
        paths.push(path_java.to_string());
    }
    
    // 3. 检查JAVA_HOME环境变量
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_home_path = PathBuf::from(&java_home);
        let java_exe = java_home_path.join("bin").join(
            if cfg!(windows) { "java.exe" } else { "java" }
        );
        
        if java_exe.exists() && is_valid_java_executable(&java_exe) {
            let java_exe_path = java_exe.to_string_lossy().replace("\\", "/");
            paths.push(java_exe_path);
        }
    }
    
    // 去重
    let mut unique_paths = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    
    for path in paths {
        // 尝试规范化路径进行比较
        let normalized = if path == "java" || path == "java.exe" {
            path.clone()
        } else {
            // 对于文件路径，尝试获取规范路径进行比较
            match PathBuf::from(&path).canonicalize() {
                Ok(canonical) => canonical.to_string_lossy().replace("\\", "/"),
                Err(_) => path.clone(),
            }
        };
        
        if !seen_paths.contains(&normalized) {
            seen_paths.insert(normalized);
            unique_paths.push(path);
        }
    }
    
    // 按路径排序
    unique_paths.sort();
    
    Ok(unique_paths)
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
        let java_exe = path_buf.join("bin").join(
            if cfg!(windows) { "java.exe" } else { "java" }
        );
        Ok(java_exe.exists() && is_valid_java_executable(&java_exe))
    } else {
        Ok(false)
    }
}
