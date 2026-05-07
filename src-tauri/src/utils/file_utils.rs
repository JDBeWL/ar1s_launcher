use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::utils::minecraft::{evaluate_rules, get_mc_os_name};
use log::{debug, info, warn};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};

/// 实例名称验证结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct InstanceNameValidation {
    pub is_valid: bool,
    pub error_message: Option<String>,
}

/// 验证实例名称是否安全有效
/// 
/// 规则：
/// - 长度在 1-64 字符之间
/// - 不能包含路径分隔符 (/, \)
/// - 不能包含路径遍历字符 (..)
/// - 不能包含 Windows 保留字符 (<, >, :, ", |, ?, *)
/// - 不能以点或空格开头/结尾
/// - 不能是 Windows 保留名称 (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
/// - 不能包含控制字符 (ASCII 0-31)
pub fn validate_instance_name(name: &str) -> InstanceNameValidation {
    // 检查空名称
    if name.is_empty() {
        return InstanceNameValidation {
            is_valid: false,
            error_message: Some("实例名称不能为空".to_string()),
        };
    }

    // 检查长度
    if name.len() > 64 {
        return InstanceNameValidation {
            is_valid: false,
            error_message: Some("实例名称不能超过 64 个字符".to_string()),
        };
    }

    // 检查是否以点或空格开头/结尾
    if name.starts_with('.') || name.starts_with(' ') {
        return InstanceNameValidation {
            is_valid: false,
            error_message: Some("实例名称不能以点或空格开头".to_string()),
        };
    }

    if name.ends_with('.') || name.ends_with(' ') {
        return InstanceNameValidation {
            is_valid: false,
            error_message: Some("实例名称不能以点或空格结尾".to_string()),
        };
    }

    // 检查路径遍历
    if name.contains("..") {
        return InstanceNameValidation {
            is_valid: false,
            error_message: Some("实例名称不能包含 '..'".to_string()),
        };
    }

    // 检查路径分隔符
    if name.contains('/') || name.contains('\\') {
        return InstanceNameValidation {
            is_valid: false,
            error_message: Some("实例名称不能包含路径分隔符 (/ 或 \\)".to_string()),
        };
    }

    // 检查 Windows 保留字符
    const RESERVED_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];
    for c in RESERVED_CHARS {
        if name.contains(*c) {
            return InstanceNameValidation {
                is_valid: false,
                error_message: Some(format!("实例名称不能包含特殊字符 '{}'", c)),
            };
        }
    }

    // 检查控制字符 (ASCII 0-31)
    if name.chars().any(|c| (c as u32) < 32) {
        return InstanceNameValidation {
            is_valid: false,
            error_message: Some("实例名称不能包含控制字符".to_string()),
        };
    }

    // 检查 Windows 保留名称
    let name_upper = name.to_uppercase();
    let base_name = name_upper.split('.').next().unwrap_or(&name_upper);
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    
    if RESERVED_NAMES.contains(&base_name) {
        return InstanceNameValidation {
            is_valid: false,
            error_message: Some(format!("'{}' 是 Windows 保留名称，不能用作实例名称", name)),
        };
    }

    InstanceNameValidation {
        is_valid: true,
        error_message: None,
    }
}

/// 验证实例名称，如果无效则返回错误
pub fn validate_instance_name_or_error(name: &str) -> Result<(), LauncherError> {
    let validation = validate_instance_name(name);
    if !validation.is_valid {
        return Err(LauncherError::Custom(
            validation.error_message.unwrap_or_else(|| "实例名称无效".to_string())
        ));
    }
    Ok(())
}

/// 递归复制目录及其所有内容
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), std::io::Error> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// 验证文件完整性和哈希值
pub fn verify_file(
    path: &std::path::Path,
    expected_hash: Option<&str>,
    expected_size: u64,
) -> Result<bool, LauncherError> {
    if !path.exists() {
        return Ok(false);
    }
    
    let actual_size = std::fs::metadata(path)?.len();
    if expected_size > 0 && actual_size != expected_size {
        warn!("文件大小不匹配: 期望 {} 字节, 实际 {} 字节", expected_size, actual_size);
        return Ok(false);
    }
    
    if let Some(hash) = expected_hash {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha1::new();
        std::io::copy(&mut file, &mut hasher)?;
        let actual_hash = hasher.finalize();
        let actual_hash_str = format!("{:x}", actual_hash);
        let is_valid = actual_hash_str.to_lowercase() == hash.to_lowercase();
        
        if !is_valid {
            warn!("文件哈希不匹配: 期望 {}, 实际 {}", hash, actual_hash_str);
        }
        
        Ok(is_valid)
    } else {
        Ok(true)
    }
}

/// 增强的文件验证和恢复机制
pub async fn verify_and_repair_file(
    job: &crate::models::DownloadJob,
    client: &reqwest::Client,
) -> Result<bool, LauncherError> {
    let path = &job.path;
    
    // 1. 检查文件是否存在
    if !path.exists() {
        debug!("文件不存在，需要下载: {}", path.display());
        return Ok(false);
    }
    
    // 2. 验证文件完整性
    if verify_file(path, job.hash.as_deref(), job.size)? {
        debug!("文件验证通过: {}", path.display());
        return Ok(true);
    }
    
    // 3. 文件损坏，尝试修复
    warn!("文件损坏，尝试修复: {}", path.display());
    
    // 3.1 备份损坏的文件
    let backup_path = path.with_extension("bak");
    if let Err(e) = std::fs::copy(path, &backup_path) {
        warn!("备份损坏文件失败: {}", e);
    }
    
    // 3.2 删除损坏的文件
    if let Err(e) = std::fs::remove_file(path) {
        warn!("删除损坏文件失败: {}", e);
    }
    
    // 3.3 重新下载文件
    info!("重新下载文件: {}", path.display());
    
    // 创建父目录
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // 下载文件
    let response = client.get(&job.url).send().await?;
    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "下载失败: HTTP {}",
            response.status()
        )));
    }
    
    let content = response.bytes().await?;
    std::fs::write(path, &content)?;
    
    // 3.4 验证重新下载的文件
    if verify_file(path, job.hash.as_deref(), job.size)? {
        info!("文件修复成功: {}", path.display());
        // 删除备份文件
        let _ = std::fs::remove_file(&backup_path);
        Ok(true)
    } else {
        warn!("文件修复失败: {}", path.display());
        // 恢复备份文件
        if backup_path.exists() {
            let _ = std::fs::copy(&backup_path, path);
        }
        Ok(false)
    }
}

/// 批量验证文件完整性
pub async fn batch_verify_files(
    jobs: &[crate::models::DownloadJob],
    client: &reqwest::Client,
) -> Result<Vec<(String, bool)>, LauncherError> {
    use tokio::task;
    
    let mut tasks = vec![];
    
    for job in jobs {
        let job_clone = job.clone();
        let client_clone = client.clone();
        
        tasks.push(task::spawn(async move {
            let file_name = job_clone.path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            
            match verify_and_repair_file(&job_clone, &client_clone).await {
                Ok(is_valid) => (file_name, is_valid),
                Err(_) => (file_name, false),
            }
        }));
    }
    
    let mut results = vec![];
    for task in tasks {
        results.push(task.await?);
    }
    
    Ok(results)
}

/// 从版本JSON中收集下载任务
pub fn collect_download_jobs_from_json(
    version_json: &Value,
    game_dir: &PathBuf,
    version_id: &str,
) -> Result<Vec<DownloadJob>, LauncherError> {
    let mut jobs: Vec<DownloadJob> = Vec::new();
    let libraries_base_dir = game_dir.join("libraries");
    let assets_base_dir = game_dir.join("assets");
    let version_dir = game_dir.join("versions").join(version_id);

    // 1) 客户端JAR
    if let Some(client) = version_json.get("downloads").and_then(|d| d.get("client")) {
        if let Some(url) = client.get("url").and_then(|u| u.as_str()) {
            let size = client.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
            let hash = client.get("sha1").and_then(|h| h.as_str()).map(|h| h.to_string());
            let path = version_dir.join(format!("{}.jar", version_id));
            jobs.push(DownloadJob { 
                url: url.to_string(), 
                fallback_url: None, 
                path, 
                size, 
                hash 
            });
        }
    }

    // 2) 资源索引 -> 对象
    if let Some(asset_idx) = version_json.get("assetIndex") {
        if let Some(idx_id) = asset_idx.get("id").and_then(|v| v.as_str()) {
            if let Some(idx_url) = asset_idx.get("url").and_then(|v| v.as_str()) {
                let index_path = assets_base_dir.join("indexes").join(format!("{}.json", idx_id));
                jobs.push(DownloadJob {
                    url: idx_url.to_string(),
                    fallback_url: None,
                    path: index_path.clone(),
                    size: asset_idx.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                    hash: asset_idx.get("sha1").and_then(|h| h.as_str()).map(|h| h.to_string()),
                });
            }
        }
    }

    // 3) 库文件 + 原生库
    if let Some(libs) = version_json.get("libraries").and_then(|v| v.as_array()) {
        for lib in libs {
            if !evaluate_rules(lib.get("rules")) {
                continue;
            }

            // 主构件
            if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                let path = artifact.get("path").and_then(|p| p.as_str()).map(|s| s.to_string());
                let url = artifact.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
                let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                let hash = artifact.get("sha1").and_then(|h| h.as_str()).map(|h| h.to_string());
                if let Some(path_str) = path {
                    let file_path = libraries_base_dir.join(path_str.clone());
                    let download_url = if let Some(u) = url { u } else { format!("https://libraries.minecraft.net/{}", path_str) };
                    jobs.push(DownloadJob { 
                        url: download_url, 
                        fallback_url: None, 
                        path: file_path, 
                        size, 
                        hash 
                    });
                }
            }

            // 原生库/分类器
            if let Some(natives) = lib.get("natives") {
                if let Some(natives_map) = natives.as_object() {
                    let current_os = get_mc_os_name();
                    for (os_name, classifier_val) in natives_map.iter() {
                        let classifier = classifier_val.as_str().unwrap_or("");
                        if os_name == current_os || lib.get("name").and_then(|n| n.as_str()).map_or(false, |s| s.contains("lwjgl")) {
                            // 优先使用 downloads.classifiers
                            if let Some(classifiers) = lib.get("downloads").and_then(|d| d.get("classifiers")) {
                                if let Some(artifact) = classifiers.get(classifier) {
                                    if let (Some(path), Some(url)) = (artifact.get("path").and_then(|p| p.as_str()), artifact.get("url").and_then(|u| u.as_str())) {
                                        let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                        let hash = artifact.get("sha1").and_then(|h| h.as_str()).map(|h| h.to_string());
                                        let file_path = libraries_base_dir.join(path);
                                        jobs.push(DownloadJob { 
                                            url: url.to_string(), 
                                            fallback_url: None, 
                                            path: file_path, 
                                            size, 
                                            hash 
                                        });
                                        continue;
                                    }
                                }
                            }

                            if let Some(classifiers) = lib.get("classifiers") {
                                if let Some(artifact) = classifiers.get(classifier) {
                                    if let (Some(path), Some(url)) = (artifact.get("path").and_then(|p| p.as_str()), artifact.get("url").and_then(|u| u.as_str())) {
                                        let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                        let hash = artifact.get("sha1").and_then(|h| h.as_str()).map(|h| h.to_string());
                                        let file_path = libraries_base_dir.join(path);
                                        jobs.push(DownloadJob { 
                                            url: url.to_string(), 
                                            fallback_url: None, 
                                            path: file_path, 
                                            size, 
                                            hash 
                                        });
                                        continue;
                                    }
                                }
                            }

                            // 最后手段：从名称派生路径
                            if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                                let parts: Vec<&str> = name.split(":").collect();
                                if parts.len() >= 3 {
                                    let group_id = parts[0].replace(".", "/");
                                    let artifact_id = parts[1];
                                    let version = parts[2];
                                    let classifier_replaced = classifier.replace("${arch}", if cfg!(target_pointer_width = "64") { "64" } else { "32" });
                                    let natives_path = format!("{}/{}/{}/{}-{}-{}.jar", group_id, artifact_id, version, artifact_id, version, classifier_replaced);
                                    let natives_url = format!("https://libraries.minecraft.net/{}", natives_path);
                                    let file_path = libraries_base_dir.join(&natives_path);
                                    jobs.push(DownloadJob { 
                                        url: natives_url, 
                                        fallback_url: None, 
                                        path: file_path, 
                                        size: 0, 
                                        hash: None
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(jobs)
}

/// 清理实例创建过程中创建的文件和目录
pub fn cleanup_instance_creation(
    game_dir: &PathBuf,
    instance_name: &str,
    _base_version_id: &str,
) {
    info!("file_utils: 开始清理实例创建过程中的文件和目录");

    // 清理整个实例目录（remove_dir_all 会递归删除所有子文件和子目录）
    let instance_dir = game_dir.join("versions").join(instance_name);
    if instance_dir.exists() {
        info!("file_utils: 清理实例目录: {}", instance_dir.display());
        if let Err(e) = fs::remove_dir_all(&instance_dir) {
            warn!("file_utils: 清理实例目录失败: {}", e);
        } else {
            info!("file_utils: 实例目录清理完成");
        }
    }

    info!("file_utils: 清理完成");
}

/// 清理Forge安装过程中创建的文件和目录
pub fn cleanup_forge_installation(
    instance_path: &PathBuf,
    game_dir: &PathBuf,
    forge_version: &crate::services::loaders::forge::ForgeVersion,
    installer_path: &PathBuf,
) {
    info!("file_utils: 开始清理Forge安装过程中的文件和目录");

    // 1. 清理版本文件夹
    let version_id = format!(
        "{}-forge-{}",
        forge_version.mcversion, forge_version.version
    );
    let version_dir = game_dir.join("versions").join(&version_id);

    if version_dir.exists() {
        info!("file_utils: 清理版本文件夹: {}", version_dir.display());
        if let Err(e) = fs::remove_dir_all(&version_dir) {
            warn!("file_utils: 清理版本文件夹失败: {}", e);
        } else {
            info!("file_utils: 版本文件夹清理完成");
        }
    }

    // 2. 清理实例目录（如果创建了）
    if instance_path.exists() {
        info!("file_utils: 清理实例目录: {}", instance_path.display());
        if let Err(e) = fs::remove_dir_all(instance_path) {
            warn!("file_utils: 清理实例目录失败: {}", e);
        } else {
            info!("file_utils: 实例目录清理完成");
        }
    }

    // 3. 清理临时安装器文件
    if installer_path.exists() {
        info!("file_utils: 清理临时安装器文件: {}", installer_path.display());
        if let Err(e) = fs::remove_file(installer_path) {
            warn!("file_utils: 清理安装器文件失败: {}", e);
        } else {
            info!("file_utils: 临时安装器文件清理完成");
        }
    }

    info!("file_utils: 清理完成");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== validate_instance_name 测试 =====

    #[test]
    fn test_valid_instance_names() {
        assert!(validate_instance_name("MyInstance").is_valid);
        assert!(validate_instance_name("my-instance-1").is_valid);
        assert!(validate_instance_name("我的实例").is_valid);
        assert!(validate_instance_name("test_instance.v2").is_valid);
        assert!(validate_instance_name("a").is_valid);
    }

    #[test]
    fn test_empty_name() {
        let result = validate_instance_name("");
        assert!(!result.is_valid);
        assert!(result.error_message.unwrap().contains("不能为空"));
    }

    #[test]
    fn test_too_long_name() {
        let long_name = "a".repeat(65);
        assert!(!validate_instance_name(&long_name).is_valid);

        // 刚好 64 字符应该有效
        let exact_name = "a".repeat(64);
        assert!(validate_instance_name(&exact_name).is_valid);
    }

    #[test]
    fn test_starts_with_dot_or_space() {
        assert!(!validate_instance_name(".hidden").is_valid);
        assert!(!validate_instance_name(" leading").is_valid);
    }

    #[test]
    fn test_ends_with_dot_or_space() {
        assert!(!validate_instance_name("trailing.").is_valid);
        assert!(!validate_instance_name("trailing ").is_valid);
    }

    #[test]
    fn test_path_traversal() {
        assert!(!validate_instance_name("..").is_valid);
        assert!(!validate_instance_name("foo/../bar").is_valid);
        assert!(!validate_instance_name("a..b").is_valid);
    }

    #[test]
    fn test_path_separators() {
        assert!(!validate_instance_name("foo/bar").is_valid);
        assert!(!validate_instance_name("foo\\bar").is_valid);
    }

    #[test]
    fn test_reserved_characters() {
        for c in &['<', '>', ':', '"', '|', '?', '*'] {
            let name = format!("test{}name", c);
            assert!(!validate_instance_name(&name).is_valid, "应该拒绝包含 '{}' 的名称", c);
        }
    }

    #[test]
    fn test_control_characters() {
        assert!(!validate_instance_name("test\x00name").is_valid);
        assert!(!validate_instance_name("test\x1fname").is_valid);
    }

    #[test]
    fn test_windows_reserved_names() {
        assert!(!validate_instance_name("CON").is_valid);
        assert!(!validate_instance_name("PRN").is_valid);
        assert!(!validate_instance_name("AUX").is_valid);
        assert!(!validate_instance_name("NUL").is_valid);
        assert!(!validate_instance_name("COM1").is_valid);
        assert!(!validate_instance_name("LPT1").is_valid);
        // 大小写不敏感
        assert!(!validate_instance_name("con").is_valid);
        assert!(!validate_instance_name("Con").is_valid);
        // 带扩展名的保留名称
        assert!(!validate_instance_name("CON.txt").is_valid);
        // 类似但不是保留名称的应该有效
        assert!(validate_instance_name("CONNECT").is_valid);
        assert!(validate_instance_name("COM10").is_valid);
    }

    #[test]
    fn test_validate_instance_name_or_error() {
        assert!(validate_instance_name_or_error("valid-name").is_ok());
        assert!(validate_instance_name_or_error("").is_err());
        assert!(validate_instance_name_or_error("CON").is_err());
    }

    // ===== verify_file 测试 =====

    #[test]
    fn test_verify_file_nonexistent() {
        let result = verify_file(Path::new("/nonexistent/path"), Some("abc123"), 100);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_file_no_hash() {
        let dir = std::env::temp_dir().join("ar1s_test_verify");
        let _ = fs::create_dir_all(&dir);
        let file_path = dir.join("test_no_hash.bin");
        fs::write(&file_path, b"hello").unwrap();

        assert!(verify_file(&file_path, None, 0).unwrap());
        assert!(verify_file(&file_path, None, 5).unwrap());
        assert!(!verify_file(&file_path, None, 999).unwrap());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_verify_file_with_sha1() {
        let dir = std::env::temp_dir().join("ar1s_test_sha1");
        let _ = fs::create_dir_all(&dir);
        let file_path = dir.join("test_sha1.bin");
        fs::write(&file_path, b"hello").unwrap();

        let correct_hash = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
        assert!(verify_file(&file_path, Some(correct_hash), 5).unwrap());
        assert!(!verify_file(&file_path, Some("0000000000000000000000000000000000000000"), 5).unwrap());

        let _ = fs::remove_dir_all(&dir);
    }
}