//! 文件与下载校验工具
//!
//! 实例名校验、目录递归拷贝、sha1 与大小校验修复

use crate::errors::LauncherError;
use log::{debug, info, warn};
use sha1::{Digest, Sha1};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstanceNameValidation {
    pub is_valid: bool,
    pub error_message: Option<String>,
}

/// 会直接当目录名/文件名用的字符串：拒绝路径分隔符、`..`、盘符与 Windows 保留名
fn validate_safe_component(name: &str, label: &str, max_len: usize) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!("{label}不能为空"));
    }

    if name.len() > max_len {
        return Err(format!("{label}不能超过 {max_len} 个字符"));
    }

    if name.starts_with('.') || name.starts_with(' ') {
        return Err(format!("{label}不能以点或空格开头"));
    }

    if name.ends_with('.') || name.ends_with(' ') {
        return Err(format!("{label}不能以点或空格结尾"));
    }

    if name.contains("..") {
        return Err(format!("{label}不能包含 '..'"));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(format!("{label}不能包含路径分隔符 (/ 或 \\)"));
    }

    const RESERVED_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];
    for c in RESERVED_CHARS {
        if name.contains(*c) {
            return Err(format!("{label}不能包含特殊字符 '{}'", c));
        }
    }

    if name.chars().any(|c| (c as u32) < 32) {
        return Err(format!("{label}不能包含控制字符"));
    }

    let name_upper = name.to_uppercase();
    let base_name = name_upper.split('.').next().unwrap_or(&name_upper);
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    if RESERVED_NAMES.contains(&base_name) {
        return Err(format!("'{}' 是 Windows 保留名称，不能用作{label}", name));
    }

    Ok(())
}

/// 实例名会直接当目录名用，带路径分隔符或 `..` 就能跳出版本目录
/// Windows 保留名大小写不敏感，且只看首个 `.` 之前的部分（`CON.txt` 同样拒绝）
pub fn validate_instance_name(name: &str) -> InstanceNameValidation {
    match validate_safe_component(name, "实例名称", 64) {
        Ok(()) => InstanceNameValidation {
            is_valid: true,
            error_message: None,
        },
        Err(error_message) => InstanceNameValidation {
            is_valid: false,
            error_message: Some(error_message),
        },
    }
}

pub fn validate_instance_name_or_error(name: &str) -> Result<(), LauncherError> {
    validate_safe_component(name, "实例名称", 64).map_err(LauncherError::Custom)
}

/// 版本 ID 会拼进 versions/<id>，规则与实例目录名一致
pub fn validate_version_id_or_error(version_id: &str) -> Result<(), LauncherError> {
    validate_safe_component(version_id, "版本 ID", 64).map_err(LauncherError::Custom)
}

/// 远端给的文件名只允许是单个文件名，不能带目录或 ADS 冒号
pub fn validate_file_name_or_error(file_name: &str) -> Result<(), LauncherError> {
    validate_safe_component(file_name, "文件名", 200).map_err(LauncherError::Custom)
}

pub fn instance_dir(base: &Path, instance_name: &str) -> Result<PathBuf, LauncherError> {
    validate_instance_name_or_error(instance_name)?;
    Ok(base.join(instance_name))
}

pub fn version_dir(base: &Path, version_id: &str) -> Result<PathBuf, LauncherError> {
    validate_version_id_or_error(version_id)?;
    Ok(base.join(version_id))
}

/// mrpack 清单和 zip 条目的路径来自远端，拼到目标目录前先确认它是相对路径
pub fn is_safe_relative_path(raw: &str) -> bool {
    if raw.is_empty() || raw.contains("..") || raw.contains(':') {
        return false;
    }
    if raw.starts_with('/') || raw.starts_with('\\') {
        return false;
    }
    true
}

pub fn sha1_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// maven 的 .sha1 文件常见格式是「hash  filename」或只有 hash
pub fn parse_sha1(text: &str) -> Option<String> {
    let hash = text.split_whitespace().next()?;
    (hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| hash.to_ascii_lowercase())
}

pub fn verify_sha1(bytes: &[u8], expected: &str) -> bool {
    let expected = expected.trim();
    !expected.is_empty() && sha1_hex(bytes).eq_ignore_ascii_case(expected)
}

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

/// expected_size 为 0 或 expected_hash 为 None 时，对应校验直接跳过
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
        warn!(
            "文件大小不匹配: 期望 {} 字节, 实际 {} 字节",
            expected_size, actual_size
        );
        return Ok(false);
    }

    if let Some(hash) = expected_hash {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha1::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let read = file.read(&mut buf)?;
            if read == 0 {
                break;
            }
            hasher.update(&buf[..read]);
        }
        let actual_hash: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let is_valid = actual_hash.to_lowercase() == hash.to_lowercase();

        if !is_valid {
            warn!("文件哈希不匹配: 期望 {}, 实际 {}", hash, actual_hash);
        }

        Ok(is_valid)
    } else {
        Ok(true)
    }
}

pub async fn verify_and_repair_file(
    job: &crate::models::DownloadJob,
    client: &reqwest::Client,
) -> Result<bool, LauncherError> {
    let path = &job.path;

    if !path.exists() {
        debug!("文件不存在，需要下载: {}", path.display());
        return Ok(false);
    }

    if verify_file(path, job.hash.as_deref(), job.size)? {
        debug!("文件验证通过: {}", path.display());
        return Ok(true);
    }

    warn!("文件损坏，尝试修复: {}", path.display());

    let backup_path = path.with_extension("bak");
    if let Err(e) = std::fs::copy(path, &backup_path) {
        warn!("备份损坏文件失败: {}", e);
    }

    if let Err(e) = std::fs::remove_file(path) {
        warn!("删除损坏文件失败: {}", e);
    }

    info!("重新下载文件: {}", path.display());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let response = client.get(&job.url).send().await?;
    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "下载失败: HTTP {}",
            response.status()
        )));
    }

    let content = response.bytes().await?;
    std::fs::write(path, &content)?;

    if verify_file(path, job.hash.as_deref(), job.size)? {
        info!("文件修复成功: {}", path.display());
        let _ = std::fs::remove_file(&backup_path);
        Ok(true)
    } else {
        warn!("文件修复失败: {}", path.display());
        if backup_path.exists() {
            let _ = std::fs::copy(&backup_path, path);
        }
        Ok(false)
    }
}

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
            let file_name = job_clone
                .path
                .file_name()
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

pub fn cleanup_instance_creation(game_dir: &PathBuf, instance_name: &str, _base_version_id: &str) {
    info!("file_utils: 开始清理实例创建过程中的文件和目录");

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_is_safe_relative_path() {
        let is_safe = super::is_safe_relative_path;
        assert!(is_safe("mods/x.jar"));
        assert!(is_safe("config/foo/bar.toml"));
        assert!(!is_safe("mods/../../evil.jar"));
        assert!(!is_safe("/etc/passwd"));
        assert!(!is_safe("\\windows\\system32"));
        assert!(!is_safe("C:\\evil\\x.jar"));
        assert!(!is_safe("mods/a:b.jar"));
        assert!(!is_safe(""));
    }

    #[test]
    fn test_too_long_name() {
        let long_name = "a".repeat(65);
        assert!(!validate_instance_name(&long_name).is_valid);

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
            assert!(
                !validate_instance_name(&name).is_valid,
                "应该拒绝包含 '{}' 的名称",
                c
            );
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
        assert!(!validate_instance_name("con").is_valid);
        assert!(!validate_instance_name("Con").is_valid);
        assert!(!validate_instance_name("CON.txt").is_valid);
        // 相似但不在保留名单里的名称应当通过
        assert!(validate_instance_name("CONNECT").is_valid);
        assert!(validate_instance_name("COM10").is_valid);
    }

    #[test]
    fn test_validate_instance_name_or_error() {
        assert!(validate_instance_name_or_error("valid-name").is_ok());
        assert!(validate_instance_name_or_error("").is_err());
        assert!(validate_instance_name_or_error("CON").is_err());
    }

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
        assert!(
            !verify_file(
                &file_path,
                Some("0000000000000000000000000000000000000000"),
                5
            )
            .unwrap()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_checked_instance_dir_rejects_path_components() {
        let base = Path::new("versions");

        assert!(instance_dir(base, "../outside").is_err());
        assert!(instance_dir(base, "a/b").is_err());
        assert!(instance_dir(base, "a\\b").is_err());
        assert!(instance_dir(base, "").is_err());
        assert_eq!(
            instance_dir(base, "1.20.1-Forge").unwrap(),
            base.join("1.20.1-Forge")
        );
        assert_eq!(
            instance_dir(base, "我的实例").unwrap(),
            base.join("我的实例")
        );
    }

    #[test]
    fn test_checked_version_dir_rejects_path_components() {
        let base = Path::new("versions");

        assert!(version_dir(base, "../../etc").is_err());
        assert!(version_dir(base, "1.20.1/evil").is_err());
        assert!(version_dir(base, "C:\\evil").is_err());
        assert!(version_dir(base, ".").is_err());
        assert_eq!(
            version_dir(base, "1.20.1-forge-47.2.0").unwrap(),
            base.join("1.20.1-forge-47.2.0")
        );
    }

    #[test]
    fn test_sha1_helpers() {
        let text = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";

        assert_eq!(sha1_hex(b"hello"), text);
        assert!(verify_sha1(b"hello", &text.to_uppercase()));
        assert!(!verify_sha1(
            b"hello",
            "0000000000000000000000000000000000000000"
        ));
        assert_eq!(
            parse_sha1("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d  forge.jar\n").as_deref(),
            Some(text)
        );
        assert_eq!(parse_sha1("not-a-hash"), None);
        assert_eq!(parse_sha1(""), None);
    }

    #[test]
    fn test_download_filename_rejects_paths_but_allows_long_names() {
        assert!(validate_file_name_or_error("pack.mrpack").is_ok());
        assert!(validate_file_name_or_error(&format!("{}.mrpack", "a".repeat(120))).is_ok());
        assert!(validate_file_name_or_error("").is_err());
        assert!(validate_file_name_or_error("../pack.mrpack").is_err());
        assert!(validate_file_name_or_error("mods/pack.mrpack").is_err());
        assert!(validate_file_name_or_error("mods\\pack.mrpack").is_err());
        assert!(validate_file_name_or_error("C:\\pack.mrpack").is_err());
        assert!(validate_file_name_or_error("CON.mrpack").is_err());
    }
}
