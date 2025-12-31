//! Forge 加载器安装
//!
//! 支持旧版 (1.12.2-) 和新版 (1.13+) Forge 的安装

use crate::errors::LauncherError;
use crate::services::config;
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use zip::ZipArchive;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// 镜像源常量
const BMCL_API_BASE_URL: &str = "https://bmclapi2.bangbang93.com";
const BMCL_LIBRARIES_URL: &str = "https://bmclapi2.bangbang93.com/libraries";
const MAVEN_FORGE: &str = "https://maven.minecraftforge.net";
const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";
const MAVEN_MINECRAFT: &str = "https://libraries.minecraft.net";

/// Forge 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeVersion {
    pub version: String,
    pub mcversion: String,
}

/// 安装 Forge 加载器（统一入口）
pub async fn install_forge(
    mc_version: &str,
    forge_version: &str,
    instance_name: &str,
    game_dir: &Path,
) -> Result<(), LauncherError> {
    info!(
        "安装 Forge: MC {} + Forge {} -> {}",
        mc_version, forge_version, instance_name
    );

    let app_config = config::load_config()?;
    let java_path = app_config
        .java_path
        .ok_or_else(|| LauncherError::Custom("未设置 Java 路径".to_string()))?;

    let forge_ver = ForgeVersion {
        version: forge_version.to_string(),
        mcversion: mc_version.to_string(),
    };

    // 下载安装器
    let installer_path = download_forge_installer(&forge_ver).await?;

    // 预下载必要库 (旧版 Forge)
    if !is_new_forge(mc_version) {
        let libs_dir = game_dir.join("libraries");
        let _ = download_launchwrapper_library(&libs_dir, mc_version).await;
        let _ = download_asm_library(&libs_dir, mc_version).await;
        let _ = download_lzma_library(&libs_dir, mc_version).await;
    }

    // 准备 launcher_profiles.json
    let profiles_path = game_dir.join("launcher_profiles.json");
    if !profiles_path.exists() {
        fs::write(&profiles_path, r#"{"profiles":{}}"#).ok();
    }

    // 尝试使用官方安装器
    info!("Forge: 尝试官方安装器");
    let install_result = run_official_installer(&installer_path, game_dir, &java_path).await;

    let forge_version_id = get_forge_version_id(mc_version, forge_version);

    match install_result {
        Ok(()) => {
            info!("Forge: 官方安装器成功");
        }
        Err(e) => {
            warn!("Forge: 官方安装器失败: {}, 尝试手动安装", e);

            if is_new_forge(mc_version) {
                manual_install_new_forge(&installer_path, game_dir, &forge_ver, &java_path).await?;
            } else {
                manual_install_old_forge(&installer_path, game_dir, &forge_ver).await?;
            }
        }
    }

    // 清理安装器
    if installer_path.exists() {
        fs::remove_file(&installer_path).ok();
    }

    // 重命名/复制版本 JSON 到实例名称
    let versions_dir = game_dir.join("versions");
    let forge_dir = versions_dir.join(&forge_version_id);
    let instance_dir = versions_dir.join(instance_name);

    if forge_dir.exists() && forge_dir != instance_dir {
        // 读取 Forge 版本 JSON
        let forge_json_path = forge_dir.join(format!("{}.json", forge_version_id));
        if forge_json_path.exists() {
            let content = fs::read_to_string(&forge_json_path)?;
            let mut json: Value = serde_json::from_str(&content)?;

            // 修改 ID 为实例名称
            if let Some(obj) = json.as_object_mut() {
                obj.insert("id".to_string(), serde_json::json!(instance_name));
            }

            // 创建实例目录并保存
            fs::create_dir_all(&instance_dir)?;
            let instance_json_path = instance_dir.join(format!("{}.json", instance_name));
            fs::write(&instance_json_path, serde_json::to_string_pretty(&json)?)?;

            // 删除原 Forge 目录
            let _ = fs::remove_dir_all(&forge_dir);
        }
    }

    info!("Forge: 安装完成");
    Ok(())
}

/// 获取 Forge 版本列表
pub async fn get_forge_versions(mc_version: &str) -> Result<Vec<ForgeVersion>, LauncherError> {
    let client = Client::new();
    let url = format!("{}/forge/minecraft/{}", BMCL_API_BASE_URL, mc_version);

    info!("Forge: 获取版本列表: {}", url);
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取 Forge 版本失败: {}",
            response.status()
        )));
    }

    let mut versions: Vec<ForgeVersion> = response.json().await?;

    // 排序：最新版本在前
    versions.sort_by(|a, b| compare_forge_versions(&b.version, &a.version));

    Ok(versions)
}

// ============ 内部辅助函数 ============

/// 判断是否为新版 Forge (1.13+)
fn is_new_forge(mc_version: &str) -> bool {
    let parts: Vec<&str> = mc_version.split('.').collect();
    if parts.len() >= 2 {
        if let Ok(minor) = parts[1].parse::<u32>() {
            return minor >= 13;
        }
    }
    false
}

/// 生成标准的 Forge 版本 ID
fn get_forge_version_id(mc_version: &str, forge_version: &str) -> String {
    format!("{}-forge-{}", mc_version, forge_version)
}

/// 比较 Forge 版本号
fn compare_forge_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u32> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };

    let a_parts = parse(a);
    let b_parts = parse(b);

    for i in 0..std::cmp::max(a_parts.len(), b_parts.len()) {
        let a_num = a_parts.get(i).unwrap_or(&0);
        let b_num = b_parts.get(i).unwrap_or(&0);
        match a_num.cmp(b_num) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord,
        }
    }
    std::cmp::Ordering::Equal
}

/// 下载 Forge 安装器
async fn download_forge_installer(
    forge_version: &ForgeVersion,
) -> Result<std::path::PathBuf, LauncherError> {
    let installer_filename = format!(
        "forge-{}-{}-installer.jar",
        forge_version.mcversion, forge_version.version
    );
    let installer_path = std::env::temp_dir().join(&installer_filename);

    // 判断是否需要使用旧版 URL 格式
    let needs_old_format = forge_version.mcversion.starts_with("1.7")
        || forge_version.mcversion.starts_with("1.9")
        || forge_version.mcversion == "1.10";

    let sources = if needs_old_format {
        vec![
            format!(
                "{}/net/minecraftforge/forge/{mc}-{v}-{mc}/forge-{mc}-{v}-{mc}-installer.jar",
                BMCL_LIBRARIES_URL,
                mc = forge_version.mcversion,
                v = forge_version.version
            ),
            format!(
                "{}/net/minecraftforge/forge/{mc}-{v}-{mc}/forge-{mc}-{v}-{mc}-installer.jar",
                MAVEN_FORGE,
                mc = forge_version.mcversion,
                v = forge_version.version
            ),
            format!(
                "{}/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
                BMCL_LIBRARIES_URL,
                mc = forge_version.mcversion,
                v = forge_version.version
            ),
        ]
    } else {
        vec![
            format!(
                "{}/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
                BMCL_LIBRARIES_URL,
                mc = forge_version.mcversion,
                v = forge_version.version
            ),
            format!(
                "{}/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
                MAVEN_FORGE,
                mc = forge_version.mcversion,
                v = forge_version.version
            ),
        ]
    };

    let client = Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    for url in &sources {
        info!("Forge: 尝试下载: {}", url);
        if let Ok(resp) = download_with_retry(url, &client, 3).await {
            if let Ok(bytes) = resp.bytes().await {
                if bytes.len() > 1024 && bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
                    fs::write(&installer_path, &bytes)
                        .map_err(|e| LauncherError::Custom(format!("写入安装器失败: {}", e)))?;
                    info!("Forge: 安装器已下载");
                    return Ok(installer_path);
                }
            }
        }
    }

    Err(LauncherError::Custom("安装器下载失败".to_string()))
}

/// 运行官方安装器
async fn run_official_installer(
    installer_path: &Path,
    game_dir: &Path,
    java_path: &str,
) -> Result<(), LauncherError> {
    let mut cmd = Command::new(java_path);
    cmd.current_dir(game_dir)
        .arg("-jar")
        .arg(installer_path)
        .arg("--installClient");

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .output()
        .map_err(|e| LauncherError::Custom(format!("执行安装器失败: {}", e)))?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    if stderr.contains("not a recognized option") || stderr.contains("UnrecognizedOptionException")
    {
        let mut cmd2 = Command::new(java_path);
        cmd2.current_dir(game_dir)
            .arg("-Djava.awt.headless=true")
            .arg("-jar")
            .arg(installer_path);

        #[cfg(windows)]
        cmd2.creation_flags(CREATE_NO_WINDOW);

        let output2 = cmd2
            .output()
            .map_err(|e| LauncherError::Custom(format!("执行安装器失败: {}", e)))?;

        if !output2.status.success() {
            let stderr2 = String::from_utf8_lossy(&output2.stderr);
            if stderr2.contains("HeadlessException") {
                return Err(LauncherError::Custom(
                    "安装器需要 GUI，切换到手动安装".to_string(),
                ));
            }
            return Err(LauncherError::Custom(format!("安装器失败: {}", stderr2)));
        }
    } else if !output.status.success() {
        return Err(LauncherError::Custom(format!("安装器失败: {}", stderr)));
    }

    Ok(())
}

/// 通用下载函数，支持重试
async fn download_with_retry(
    url: &str,
    client: &Client,
    max_retries: usize,
) -> Result<reqwest::Response, LauncherError> {
    let mut retry_count = 0;

    while retry_count <= max_retries {
        retry_count += 1;
        debug!("Forge: 下载尝试第{}次: {}", retry_count, url);

        if retry_count > 1 {
            let delay = std::cmp::min(2u64.pow(retry_count as u32 - 1), 10);
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        }

        match client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok(response);
            }
            Ok(response) => {
                warn!("Forge: 下载失败，状态: {}", response.status());
            }
            Err(e) => {
                warn!("Forge: 网络错误: {}", e);
            }
        }
    }

    Err(LauncherError::Custom(format!(
        "下载失败: 超过最大重试次数 {}",
        url
    )))
}

/// 下载库文件
async fn download_library(
    libraries_dir: &Path,
    rel_path: &str,
    sources: Vec<String>,
    lib_name: &str,
) -> Result<(), LauncherError> {
    let target_path = libraries_dir.join(rel_path);

    if target_path.exists() {
        return Ok(());
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let client = Client::new();
    for url in &sources {
        if let Ok(response) = download_with_retry(url, &client, 3).await {
            if let Ok(bytes) = response.bytes().await {
                if bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                    fs::write(&target_path, &bytes)?;
                    info!("Forge: {} 下载成功", lib_name);
                    return Ok(());
                }
            }
        }
    }

    Err(LauncherError::Custom(format!(
        "所有 {} 下载源均失败",
        lib_name
    )))
}

async fn download_launchwrapper_library(
    libraries_dir: &Path,
    _mc_version: &str,
) -> Result<(), LauncherError> {
    let path = "net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar";
    download_library(
        libraries_dir,
        path,
        vec![
            format!("{}/{}", BMCL_LIBRARIES_URL, path),
            format!("{}/{}", MAVEN_MINECRAFT, path),
        ],
        "LaunchWrapper",
    )
    .await
}

async fn download_asm_library(libraries_dir: &Path, mc_version: &str) -> Result<(), LauncherError> {
    let version = if mc_version.starts_with("1.7") {
        "5.0.3"
    } else if mc_version.starts_with("1.8")
        || mc_version.starts_with("1.9")
        || mc_version.starts_with("1.10")
        || mc_version.starts_with("1.11")
    {
        "5.0.4"
    } else {
        "5.2"
    };

    let path = format!("org/ow2/asm/asm-all/{}/asm-all-{}.jar", version, version);
    download_library(
        libraries_dir,
        &path,
        vec![
            format!("{}/{}", BMCL_LIBRARIES_URL, path),
            format!("{}/{}", MAVEN_CENTRAL, path),
        ],
        "ASM",
    )
    .await
}

async fn download_lzma_library(
    libraries_dir: &Path,
    mc_version: &str,
) -> Result<(), LauncherError> {
    let path = if mc_version.starts_with("1.7") {
        "lzma/lzma/0.0.1/lzma-0.0.1.jar"
    } else {
        "org/tukaani/xz/1.8/xz-1.8.jar"
    };
    download_library(
        libraries_dir,
        path,
        vec![
            format!("{}/{}", BMCL_LIBRARIES_URL, path),
            format!("{}/{}", MAVEN_CENTRAL, path),
        ],
        "LZMA/XZ",
    )
    .await
}


// ============ 手动安装逻辑 ============

/// 从 Maven 坐标解析路径
fn maven_to_path(name: &str, classifier: Option<&str>, extension: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }

    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let group_path = group.replace('.', "/");

    let filename = match classifier {
        Some(c) => format!("{}-{}-{}.{}", artifact, version, c, extension),
        None => format!("{}-{}.{}", artifact, version, extension),
    };

    Some(format!("{}/{}/{}/{}", group_path, artifact, version, filename))
}

/// 从 install_profile 下载库
async fn download_library_from_profile(
    library: &Value,
    libraries_dir: &Path,
    client: &Client,
) -> Result<(), LauncherError> {
    let name = match library["name"].as_str() {
        Some(n) => n,
        None => return Ok(()),
    };

    if let Some(false) = library.get("clientreq").and_then(|v| v.as_bool()) {
        return Ok(());
    }

    // 优先使用 downloads.artifact
    if let Some(artifact) = library.get("downloads").and_then(|d| d.get("artifact")) {
        if let Some(path) = artifact.get("path").and_then(|p| p.as_str()) {
            let target_path = libraries_dir.join(path);
            if target_path.exists() {
                return Ok(());
            }

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).ok();
            }

            let mut sources = Vec::new();
            if let Some(url) = artifact.get("url").and_then(|u| u.as_str()) {
                let mirrored = url
                    .replace("https://libraries.minecraft.net", BMCL_LIBRARIES_URL)
                    .replace(
                        "https://maven.minecraftforge.net",
                        &format!("{}/maven", BMCL_API_BASE_URL),
                    );
                if mirrored != url {
                    sources.push(mirrored);
                }
                sources.push(url.to_string());
            }
            sources.push(format!("{}/{}", BMCL_LIBRARIES_URL, path));
            sources.push(format!("{}/{}", MAVEN_FORGE, path));

            for url in &sources {
                if let Ok(resp) = download_with_retry(url, client, 2).await {
                    if let Ok(bytes) = resp.bytes().await {
                        if bytes.len() > 100 {
                            fs::write(&target_path, &bytes).ok();
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    // 回退到从 name 构建路径
    if let Some(maven_path) = maven_to_path(name, None, "jar") {
        let target_path = libraries_dir.join(&maven_path);
        if target_path.exists() {
            return Ok(());
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let sources = vec![
            format!("{}/{}", BMCL_LIBRARIES_URL, maven_path),
            format!("{}/{}", MAVEN_FORGE, maven_path),
            format!("{}/{}", MAVEN_CENTRAL, maven_path),
        ];

        for url in &sources {
            if let Ok(resp) = download_with_retry(url, &Client::new(), 2).await {
                if let Ok(bytes) = resp.bytes().await {
                    if bytes.len() > 100 {
                        fs::write(&target_path, &bytes).ok();
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

/// 手动安装旧版 Forge (1.12.2-)
async fn manual_install_old_forge(
    installer_path: &Path,
    game_dir: &Path,
    forge_version: &ForgeVersion,
) -> Result<(), LauncherError> {
    info!("Forge: 开始手动安装旧版本 Forge");

    let file = fs::File::open(installer_path)?;
    let mut archive = ZipArchive::new(file)?;

    let profile: Value = {
        let mut content = String::new();
        archive
            .by_name("install_profile.json")
            .map_err(|_| LauncherError::Custom("未找到 install_profile.json".to_string()))?
            .read_to_string(&mut content)?;
        serde_json::from_str(&content)?
    };

    let libraries_dir = game_dir.join("libraries");
    let client = Client::new();

    // 下载库文件
    if let Some(libs) = profile
        .get("versionInfo")
        .and_then(|v| v.get("libraries"))
        .and_then(|l| l.as_array())
    {
        for lib in libs {
            let _ = download_library_from_profile(lib, &libraries_dir, &client).await;
        }
    }

    // 创建版本 JSON
    let version_id = get_forge_version_id(&forge_version.mcversion, &forge_version.version);
    let version_dir = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&version_dir)?;

    let mut version_info = profile
        .get("versionInfo")
        .ok_or_else(|| LauncherError::Custom("缺少 versionInfo".to_string()))?
        .clone();

    if let Value::Object(ref mut obj) = version_info {
        obj.insert("id".to_string(), serde_json::json!(version_id));
        obj.insert(
            "inheritsFrom".to_string(),
            serde_json::json!(forge_version.mcversion),
        );
        obj.insert(
            "jar".to_string(),
            serde_json::json!(forge_version.mcversion),
        );
    }

    let version_json_path = version_dir.join(format!("{}.json", version_id));
    fs::write(&version_json_path, serde_json::to_string_pretty(&version_info)?)?;

    // 提取 maven 和 universal.jar
    let file = fs::File::open(installer_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let name = file.name().to_string();

        // 安全检查：防止路径遍历攻击
        if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
            log::warn!("跳过可疑的 zip 条目: {}", name);
            continue;
        }

        if name.starts_with("maven/") && !name.ends_with('/') {
            if let Some(rel) = name.strip_prefix("maven/") {
                // 再次检查相对路径
                if rel.contains("..") {
                    log::warn!("跳过可疑的 maven 路径: {}", name);
                    continue;
                }
                let target = libraries_dir.join(rel);
                if let Some(p) = target.parent() {
                    fs::create_dir_all(p).ok();
                }
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    fs::write(&target, &buf).ok();
                }
            }
        } else if name.ends_with("-universal.jar") && !name.contains('/') {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                let forge_lib = format!(
                    "net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-universal.jar",
                    mc = forge_version.mcversion,
                    v = forge_version.version
                );
                let target = libraries_dir.join(&forge_lib);
                if let Some(p) = target.parent() {
                    fs::create_dir_all(p).ok();
                }
                fs::write(&target, &buf).ok();
            }
        }
    }

    info!("Forge: 手动安装完成");
    Ok(())
}

/// 手动安装新版 Forge (1.13+)
async fn manual_install_new_forge(
    installer_path: &Path,
    game_dir: &Path,
    forge_version: &ForgeVersion,
    java_path: &str,
) -> Result<(), LauncherError> {
    info!("Forge: 开始手动安装新版 Forge (1.13+)");

    let file = fs::File::open(installer_path)?;
    let mut archive = ZipArchive::new(file)?;

    let profile: Value = {
        let mut content = String::new();
        archive
            .by_name("install_profile.json")
            .map_err(|_| LauncherError::Custom("未找到 install_profile.json".to_string()))?
            .read_to_string(&mut content)?;
        serde_json::from_str(&content)?
    };

    let version_json: Value = {
        let mut content = String::new();
        if let Ok(mut f) = archive.by_name("version.json") {
            f.read_to_string(&mut content).ok();
        }
        if content.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        }
    };

    let libraries_dir = game_dir.join("libraries");
    let client = Client::new();

    // 下载库
    if let Some(libs) = profile.get("libraries").and_then(|l| l.as_array()) {
        for lib in libs {
            let _ = download_library_from_profile(lib, &libraries_dir, &client).await;
        }
    }
    if let Some(libs) = version_json.get("libraries").and_then(|l| l.as_array()) {
        for lib in libs {
            let _ = download_library_from_profile(lib, &libraries_dir, &client).await;
        }
    }

    // 提取 maven 文件
    let file = fs::File::open(installer_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let name = file.name().to_string();

        // 安全检查：防止路径遍历攻击
        if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
            log::warn!("跳过可疑的 zip 条目: {}", name);
            continue;
        }

        if name.starts_with("maven/") && !name.ends_with('/') {
            if let Some(rel) = name.strip_prefix("maven/") {
                // 再次检查相对路径
                if rel.contains("..") {
                    log::warn!("跳过可疑的 maven 路径: {}", name);
                    continue;
                }
                let target = libraries_dir.join(rel);
                if let Some(p) = target.parent() {
                    fs::create_dir_all(p).ok();
                }
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    fs::write(&target, &buf).ok();
                }
            }
        } else if name.starts_with("data/") && !name.ends_with('/') {
            if let Some(rel) = name.strip_prefix("data/") {
                // 再次检查相对路径
                if rel.contains("..") {
                    log::warn!("跳过可疑的 data 路径: {}", name);
                    continue;
                }
                let target = libraries_dir
                    .join("net/minecraftforge/forge")
                    .join(format!(
                        "{}-{}",
                        forge_version.mcversion, forge_version.version
                    ))
                    .join(rel);
                if let Some(p) = target.parent() {
                    fs::create_dir_all(p).ok();
                }
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    fs::write(&target, &buf).ok();
                }
            }
        }
    }

    // 执行 processors
    run_forge_processors(
        &profile,
        game_dir,
        java_path,
        &forge_version.mcversion,
        &forge_version.version,
    )
    .await?;

    // 创建版本 JSON
    let version_id = get_forge_version_id(&forge_version.mcversion, &forge_version.version);
    let version_dir = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&version_dir)?;

    let mut final_version = version_json.clone();
    if let Value::Object(ref mut obj) = final_version {
        obj.insert("id".to_string(), serde_json::json!(version_id));
        obj.insert(
            "inheritsFrom".to_string(),
            serde_json::json!(forge_version.mcversion),
        );
    }

    let version_json_path = version_dir.join(format!("{}.json", version_id));
    fs::write(
        &version_json_path,
        serde_json::to_string_pretty(&final_version)?,
    )?;

    info!("Forge: 新版手动安装完成");
    Ok(())
}

/// 执行 Forge processors
async fn run_forge_processors(
    profile: &Value,
    game_dir: &Path,
    java_path: &str,
    mc_version: &str,
    forge_version: &str,
) -> Result<(), LauncherError> {
    let processors = match profile.get("processors").and_then(|p| p.as_array()) {
        Some(p) => p,
        None => return Ok(()),
    };

    let libraries_dir = game_dir.join("libraries");
    let data = profile.get("data").and_then(|d| d.as_object());

    info!("Forge: 执行 {} 个 processors", processors.len());

    for (idx, processor) in processors.iter().enumerate() {
        if let Some(sides) = processor.get("sides").and_then(|s| s.as_array()) {
            if !sides.iter().any(|s| s.as_str() == Some("client")) {
                continue;
            }
        }

        let jar_name = match processor.get("jar").and_then(|j| j.as_str()) {
            Some(j) => j,
            None => continue,
        };

        let jar_path = match maven_to_path(jar_name, None, "jar") {
            Some(p) => libraries_dir.join(p),
            None => continue,
        };

        if !jar_path.exists() {
            warn!("Forge: Processor JAR 不存在: {}", jar_path.display());
            continue;
        }

        let mut classpath = vec![jar_path.to_string_lossy().to_string()];
        if let Some(cp) = processor.get("classpath").and_then(|c| c.as_array()) {
            for lib in cp {
                if let Some(lib_name) = lib.as_str() {
                    if let Some(lib_path) = maven_to_path(lib_name, None, "jar") {
                        let full_path = libraries_dir.join(&lib_path);
                        if full_path.exists() {
                            classpath.push(full_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        let mut args: Vec<String> = Vec::new();
        if let Some(proc_args) = processor.get("args").and_then(|a| a.as_array()) {
            for arg in proc_args {
                if let Some(arg_str) = arg.as_str() {
                    args.push(resolve_processor_arg(
                        arg_str,
                        data,
                        game_dir,
                        &libraries_dir,
                        mc_version,
                        forge_version,
                    ));
                }
            }
        }

        let main_class = get_jar_main_class(&jar_path)?;

        info!(
            "Forge: 执行 processor {}/{}: {}",
            idx + 1,
            processors.len(),
            main_class
        );

        let cp_separator = if cfg!(windows) { ";" } else { ":" };
        let cp_string = classpath.join(cp_separator);

        let mut cmd = Command::new(java_path);
        cmd.current_dir(game_dir)
            .arg("-cp")
            .arg(&cp_string)
            .arg(&main_class)
            .args(&args);

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Forge: Processor {} 失败: {}", idx, stderr);
        }
    }

    Ok(())
}

fn resolve_processor_arg(
    arg: &str,
    data: Option<&serde_json::Map<String, Value>>,
    game_dir: &Path,
    libraries_dir: &Path,
    mc_version: &str,
    forge_version: &str,
) -> String {
    if arg.starts_with('{') && arg.ends_with('}') {
        let key = &arg[1..arg.len() - 1];
        if let Some(data_map) = data {
            if let Some(value) = data_map.get(key) {
                if let Some(val_obj) = value.as_object() {
                    if let Some(client_val) = val_obj.get("client").and_then(|v| v.as_str()) {
                        return resolve_data_value(
                            client_val,
                            libraries_dir,
                            game_dir,
                            mc_version,
                            forge_version,
                        );
                    }
                } else if let Some(val_str) = value.as_str() {
                    return resolve_data_value(
                        val_str,
                        libraries_dir,
                        game_dir,
                        mc_version,
                        forge_version,
                    );
                }
            }
        }
    } else if arg.starts_with('[') && arg.ends_with(']') {
        let artifact = &arg[1..arg.len() - 1];
        if let Some(path) = maven_to_path(artifact, None, "jar") {
            return libraries_dir.join(path).to_string_lossy().to_string();
        }
    }
    arg.to_string()
}

fn resolve_data_value(
    value: &str,
    libraries_dir: &Path,
    game_dir: &Path,
    mc_version: &str,
    forge_version: &str,
) -> String {
    if value.starts_with('[') && value.ends_with(']') {
        let artifact = &value[1..value.len() - 1];
        if let Some(path) = maven_to_path(artifact, None, "jar") {
            return libraries_dir.join(path).to_string_lossy().to_string();
        }
    }

    if value.starts_with('/') {
        return game_dir.join(&value[1..]).to_string_lossy().to_string();
    }

    value
        .replace("{MINECRAFT_VERSION}", mc_version)
        .replace("{FORGE_VERSION}", forge_version)
        .replace("{ROOT}", &game_dir.to_string_lossy())
        .replace("{LIBRARY_DIR}", &libraries_dir.to_string_lossy())
}

fn get_jar_main_class(jar_path: &Path) -> Result<String, LauncherError> {
    let file = fs::File::open(jar_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut manifest = archive
        .by_name("META-INF/MANIFEST.MF")
        .map_err(|_| LauncherError::Custom("JAR 中没有 MANIFEST.MF".to_string()))?;

    let mut content = String::new();
    manifest.read_to_string(&mut content)?;

    for line in content.lines() {
        if line.starts_with("Main-Class:") {
            return Ok(line.trim_start_matches("Main-Class:").trim().to_string());
        }
    }

    Err(LauncherError::Custom(
        "MANIFEST 中没有 Main-Class".to_string(),
    ))
}
