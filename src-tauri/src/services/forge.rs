use crate::errors::LauncherError;
use crate::models::ForgeVersion;
use crate::services::config;
use crate::utils::file_utils;

use log::{debug, info, warn}; // FIX: 移除了 unused import `error`
use reqwest::Client;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

// 常用镜像源常量
const BMCL_API_BASE_URL: &str = "https://bmclapi2.bangbang93.com";
const MAVEN_FORGE: &str = "https://maven.minecraftforge.net";
const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";
const MAVEN_MINECRAFT: &str = "https://libraries.minecraft.net";

/// 通用的下载函数，支持多源重试机制
async fn download_with_retry(
    url: &str,
    client: &Client,
    max_retries: usize,
) -> Result<reqwest::Response, LauncherError> {
    let mut retry_count = 0;
    let mut current_url = url.to_string();
    let mut tried_urls = vec![current_url.clone()];

    while retry_count <= max_retries {
        retry_count += 1;
        debug!("Forge: 下载尝试第{}次: {}", retry_count, current_url);

        // 添加重试延迟（指数退避）
        if retry_count > 1 {
            let delay_seconds = std::cmp::min(2u64.pow(retry_count as u32 - 1), 10);
            debug!("Forge: 等待 {} 秒后重试", delay_seconds);
            tokio::time::sleep(std::time::Duration::from_secs(delay_seconds)).await;
        }

        let result = client
            .get(&current_url)
            .header(reqwest::header::CACHE_CONTROL, "no-cache, no-store")
            .header(reqwest::header::PRAGMA, "no-cache")
            .send()
            .await;

        match result {
            Ok(response) => {
                // 处理重定向
                if response.status().is_redirection() {
                    if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
                        if let Ok(redirect_url) = location.to_str() {
                            info!("Forge: 检测到重定向到: {}", redirect_url);
                            if !tried_urls.contains(&redirect_url.to_string()) {
                                current_url = redirect_url.to_string();
                                tried_urls.push(current_url.clone());
                                retry_count = 0; // 重置重试计数
                                continue;
                            }
                        }
                    }
                }

                // 验证响应内容
                if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_MODIFIED {
                    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
                        info!("Forge: 文件未修改，使用本地缓存 (304)");
                        return Ok(response);
                    }

                    // 检查内容类型 (跳过 HTML/JSON 错误页)
                    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
                        let content_type_str = content_type.to_str().unwrap_or("").to_lowercase();
                        if content_type_str.contains("text/html") || content_type_str.contains("application/json") {
                            warn!("Forge: 返回了HTML/JSON内容，跳过: {}", current_url);
                            continue;
                        }
                    }

                    // 检查文件大小
                    if let Some(content_length) = response.headers().get(reqwest::header::CONTENT_LENGTH) {
                        if let Ok(length_str) = content_length.to_str() {
                            if let Ok(file_size) = length_str.parse::<u64>() {
                                if file_size < 1024 {
                                    warn!("Forge: 文件大小异常（{}字节），跳过", file_size);
                                    continue;
                                }
                                debug!("Forge: 文件大小: {} 字节", file_size);
                            }
                        }
                    }

                    return Ok(response);
                } else {
                    warn!("Forge: 下载失败，状态: {}", response.status());
                }
            }
            Err(e) => {
                warn!("Forge: 网络错误: {}", e);
            }
        }
    }

    Err(LauncherError::Custom(format!(
        "下载失败: 超过最大重试次数。尝试过的URL: {:?}",
        tried_urls
    )))
}

/// 通用库下载辅助函数
/// 自动检查文件是否存在，创建目录，并尝试多个源
async fn download_library(
    libraries_dir: &Path,
    rel_path: &str,
    sources: Vec<String>,
    lib_name: &str,
) -> Result<(), LauncherError> {
    let target_path = libraries_dir.join(rel_path);

    if target_path.exists() {
        debug!("Forge: {} 库已存在: {}", lib_name, target_path.display());
        return Ok(());
    }

    info!("Forge: 开始下载 {} 库", lib_name);

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LauncherError::Custom(format!("创建 {} 目录失败: {}", lib_name, e)))?;
    }

    let client = Client::new();
    for (index, source_url) in sources.iter().enumerate() {
        debug!("Forge: 尝试源 {}: {}", index + 1, source_url);
        match download_with_retry(source_url, &client, 3).await {
            Ok(response) => {
                if response.status() == reqwest::StatusCode::NOT_MODIFIED {
                    return Ok(());
                }
                let bytes = response.bytes().await?;
                // 简单的 JAR 头检查
                if bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                    fs::write(&target_path, &bytes).map_err(|e| {
                        LauncherError::Custom(format!("写入 {} 库失败: {}", lib_name, e))
                    })?;
                    info!("Forge: {} 库下载成功", lib_name);
                    return Ok(());
                } else {
                    warn!("Forge: 源 {} 返回的文件不是有效的 JAR 格式", index + 1);
                }
            }
            Err(e) => warn!("Forge: 源 {} 下载失败: {}", index + 1, e),
        }
    }

    Err(LauncherError::Custom(format!(
        "所有 {} 下载源均失败",
        lib_name
    )))
}

/// 自动下载 LaunchWrapper 库
async fn download_launchwrapper_library(
    libraries_dir: &PathBuf,
    mc_version: &str,
) -> Result<(), LauncherError> {
    // 1.8 - 1.12 版本通常使用 1.12 版本的 launchwrapper
    let version = if mc_version.starts_with("1.12")
        || mc_version.starts_with("1.11")
        || mc_version.starts_with("1.10")
        || mc_version.starts_with("1.9")
        || mc_version.starts_with("1.8")
    {
        "1.12"
    } else {
        "1.12"
    };

    let path = format!(
        "net/minecraft/launchwrapper/{}/launchwrapper-{}.jar",
        version, version
    );
    
    let sources = vec![
        format!("{}/maven/{}", BMCL_API_BASE_URL, path),
        format!("{}/{}", MAVEN_CENTRAL, path),
        format!("{}/{}", MAVEN_MINECRAFT, path),
    ];

    download_library(libraries_dir, &path, sources, "LaunchWrapper").await
}

/// 自动下载 ASM 库
async fn download_asm_library(
    libraries_dir: &PathBuf,
    mc_version: &str,
) -> Result<(), LauncherError> {
    let version = if mc_version.starts_with("1.7.10") {
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
    
    let sources = vec![
        format!("{}/maven/{}", BMCL_API_BASE_URL, path),
        format!("{}/{}", MAVEN_CENTRAL, path),
        format!("{}/{}", MAVEN_MINECRAFT, path),
    ];

    download_library(libraries_dir, &path, sources, "ASM").await
}

/// 自动下载 LZMA 库
async fn download_lzma_library(
    libraries_dir: &PathBuf,
    mc_version: &str,
) -> Result<(), LauncherError> {
    let (path, sources) = if mc_version.starts_with("1.7.10") {
        let v = "0.0.1";
        let p = format!("lzma/lzma/{}/lzma-{}.jar", v, v);
        (
            p.clone(),
            vec![
                format!("{}/maven/{}", BMCL_API_BASE_URL, p),
                format!("{}/{}", MAVEN_FORGE, p),
                format!("{}/{}", MAVEN_CENTRAL, p),
            ],
        )
    } else {
        let v = "1.8";
        let p = format!("org/tukaani/xz/{}/xz-{}.jar", v, v);
        (
            p.clone(),
            vec![
                format!("{}/maven/{}", BMCL_API_BASE_URL, p),
                format!("{}/{}", MAVEN_CENTRAL, p),
                format!("{}/{}", MAVEN_MINECRAFT, p),
            ],
        )
    };

    download_library(libraries_dir, &path, sources, "LZMA/XZ").await
}

/// 下载 Forge 核心库 (针对旧版本)
async fn download_forge_core_library(
    libraries_dir: &PathBuf,
    mc_version: &str,
    forge_version: &str,
) -> Result<(), LauncherError> {
    let path = format!(
        "net/minecraftforge/forge/{}-{}/forge-{}-{}-universal.jar",
        mc_version, forge_version, mc_version, forge_version
    );
    
    let sources = vec![
        format!("{}/maven/{}", BMCL_API_BASE_URL, path),
        format!("{}/{}", MAVEN_FORGE, path),
        format!("{}/{}", MAVEN_MINECRAFT, path),
    ];

    download_library(libraries_dir, &path, sources, "Forge Core").await
}

/// 从 install_profile.json 下载单个库
async fn download_library_from_profile(
    library: &serde_json::Value,
    libraries_dir: &PathBuf,
    client: &Client,
) -> Result<(), LauncherError> {
    let name = library["name"].as_str().ok_or_else(|| {
        LauncherError::Custom("库对象缺少 'name' 字段".to_string())
    })?;

    // 检查是否为客户端所需
    if let Some(clientreq) = library.get("clientreq").and_then(|v| v.as_bool()) {
        if !clientreq {
            debug!("Forge: 跳过服务端专用库: {}", name);
            return Ok(());
        }
    }

    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() != 3 {
        warn!("Forge: 库名称格式非标准，跳过: {}", name);
        return Ok(());
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    
    let maven_path = format!(
        "{}/{}/{}/{}-{}.jar",
        group.replace('.', "/"),
        artifact,
        version,
        artifact,
        version
    );
    
    let target_path = libraries_dir.join(&maven_path);
    if target_path.exists() {
        // 这里可以添加 checksum 校验
        return Ok(());
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            LauncherError::Custom(format!("创建库目录失败 {}: {}", name, e))
        })?;
    }

    let mut sources = Vec::new();
    // 优先使用 profile 中指定的 URL
    if let Some(url) = library.get("url").and_then(|u| u.as_str()) {
        let base_url = if url.ends_with('/') {
            url.to_string()
        } else {
            format!("{}/", url)
        };
        sources.push(format!("{}{}", base_url, maven_path));
    }
    
    // 添加默认源
    sources.push(format!("{}/maven/{}", BMCL_API_BASE_URL, maven_path));
    sources.push(format!("{}/{}", MAVEN_MINECRAFT, maven_path));
    sources.push(format!("{}/{}", MAVEN_CENTRAL, maven_path));
    sources.push(format!("{}/{}", MAVEN_FORGE, maven_path));
    sources.dedup();

    for source_url in sources {
        debug!("Forge: 尝试下载 profile 库 {}: {}", name, source_url);
        match download_with_retry(&source_url, client, 3).await {
            Ok(response) => {
                if response.status() == reqwest::StatusCode::NOT_MODIFIED {
                    return Ok(());
                }
                let bytes = response.bytes().await?;
                if bytes.len() > 100 {
                    fs::write(&target_path, &bytes).map_err(|e| {
                        LauncherError::Custom(format!("写入库失败 {}: {}", name, e))
                    })?;
                    info!("Forge: 库下载成功: {}", name);
                    return Ok(());
                }
            }
            Err(e) => warn!("Forge: 下载库 {} 失败: {}", name, e),
        }
    }

    // 即使下载失败也记录并继续，以免阻塞整个流程（有些库可能是可选的或能在运行时下载）
    warn!("Forge: 无法从任何源下载库: {}", name);
    Ok(())
}

/// 手动安装旧版本 Forge (解压和复制文件)
async fn manual_install_old_forge(
    installer_path: &PathBuf,
    game_dir: &PathBuf,
    forge_version: &ForgeVersion,
) -> Result<(), LauncherError> {
    info!("Forge: 开始手动安装旧版本 Forge");

    let file = fs::File::open(installer_path)
        .map_err(|e| LauncherError::Custom(format!("无法打开安装器文件: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| LauncherError::Custom(format!("无法读取安装器 ZIP: {}", e)))?;

    // 1. 读取 install_profile.json
    let mut install_profile_content = String::new();
    if let Ok(mut file) = archive.by_name("install_profile.json") {
        file.read_to_string(&mut install_profile_content)
            .map_err(|e| LauncherError::Custom(format!("读取 install_profile.json 失败: {}", e)))?;
    } else {
        return Err(LauncherError::Custom("安装器中未找到 install_profile.json".to_string()));
    }

    info!("Forge: 已读取 install_profile.json");
    let profile: serde_json::Value = serde_json::from_str(&install_profile_content)
        .map_err(|e| LauncherError::Custom(format!("解析 install_profile.json 失败: {}", e)))?;

    // 2. 下载库文件
    let libraries_dir = game_dir.join("libraries");
    let client = Client::new();
    if let Some(libraries) = profile.get("versionInfo").and_then(|v| v.get("libraries")).and_then(|l| l.as_array()) {
        info!("Forge: 开始从 install_profile 下载 {} 个库", libraries.len());
        for lib in libraries {
            if let Err(e) = download_library_from_profile(lib, &libraries_dir, &client).await {
                warn!("Forge: 库下载过程出错: {}", e);
            }
        }
    }

    // 3. 创建版本 JSON
    let version_id = format!("{}-Forge_{}", forge_version.mcversion, forge_version.version);
    let version_dir = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&version_dir)
        .map_err(|e| LauncherError::Custom(format!("创建版本目录失败: {}", e)))?;

    let mut version_info = profile.get("versionInfo")
        .ok_or_else(|| LauncherError::Custom("缺少 versionInfo".to_string()))?
        .clone();

    if let serde_json::Value::Object(ref mut obj) = version_info {
        obj.insert("id".to_string(), serde_json::Value::String(version_id.clone()));
        obj.insert("mainClass".to_string(), serde_json::Value::String("net.minecraft.launchwrapper.Launch".to_string()));
        obj.insert("inheritsFrom".to_string(), serde_json::Value::String(forge_version.mcversion.clone()));
        obj.insert("jar".to_string(), serde_json::Value::String(forge_version.mcversion.clone()));

        // 处理 minecraftArguments: 确保 FMLTweaker 存在且不重复
        let mut args_string = String::new();
        
        // 尝试获取旧版 arguments 字符串
        if let Some(serde_json::Value::String(s)) = obj.get("minecraftArguments") {
            args_string = s.clone();
        } 
        // 尝试处理新版 arguments 对象 (如果是 1.13+ 风格但混在旧版安装器中)
        else if let Some(serde_json::Value::Object(args_obj)) = obj.get("arguments") {
            if let Some(serde_json::Value::Array(game_args)) = args_obj.get("game") {
                for arg in game_args {
                    if let serde_json::Value::String(s) = arg {
                        if !s.contains('$') {
                            args_string.push_str(" ");
                            args_string.push_str(s);
                        }
                    }
                }
            }
        }

        // 清理并重组参数
        let cleaned_args = args_string.replace("--tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker", "");
        let final_args = format!(
            "--tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker {}", 
            cleaned_args.trim()
        );
        obj.insert("minecraftArguments".to_string(), serde_json::Value::String(final_args));
    }

    let version_json_path = version_dir.join(format!("{}.json", version_id));
    fs::write(&version_json_path, serde_json::to_string_pretty(&version_info).unwrap())
        .map_err(|e| LauncherError::Custom(format!("写入版本 JSON 失败: {}", e)))?;
    info!("Forge: 已写入版本 JSON: {}", version_json_path.display());

    // 4. 重新解压 maven 文件和 universal jar
    // 重新打开 zip 因为之前的 borrow 可能被占用了，或者为了清晰起见
    let file = fs::File::open(installer_path).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let file_name = file.name().to_string();

        // 提取 maven 目录下的库
        if file_name.starts_with("maven/") && !file_name.ends_with('/') {
            let rel_path = file_name.strip_prefix("maven/").unwrap();
            let target_path = libraries_dir.join(rel_path);
            
            if let Some(p) = target_path.parent() {
                fs::create_dir_all(p).ok();
            }
            
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).ok();
            fs::write(&target_path, &buf).ok();
        } 
        // 提取 universal.jar (核心 Forge jar)
        else if file_name.ends_with("-universal.jar") && !file_name.contains('/') {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).ok();
            
            // 写入 libraries
            let forge_lib_path = format!(
                "net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-universal.jar",
                mc = forge_version.mcversion,
                v = forge_version.version
            );
            let lib_target = libraries_dir.join(&forge_lib_path);
            if let Some(p) = lib_target.parent() { fs::create_dir_all(p).ok(); }
            fs::write(&lib_target, &buf).ok();

            // 写入 versions 目录 (作为 version jar)
            let ver_jar_target = version_dir.join(format!("{}.jar", version_id));
            fs::write(&ver_jar_target, &buf).ok();
            
            info!("Forge: 已提取 Universal Jar 到库和版本目录");
        }
    }

    info!("Forge: 手动安装流程完成");
    Ok(())
}

/// 获取 Forge 版本列表
pub async fn get_forge_versions(
    minecraft_version: String,
) -> Result<Vec<ForgeVersion>, LauncherError> {
    let client = Client::new();
    let url = format!(
        "{}/forge/minecraft/{}",
        BMCL_API_BASE_URL, minecraft_version
    );

    info!("Forge: 正在获取版本列表: {}", url);
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取Forge版本失败: API返回状态 {}",
            response.status()
        )));
    }

    let mut versions: Vec<ForgeVersion> = response.json().await?;

    // 智能排序Forge版本号
    versions.sort_by(|a, b| {
        compare_forge_versions(&a.version, &b.version)
    });

    // 反转以显示最新版本在前
    Ok(versions.into_iter().rev().collect())
}

/// 比较两个Forge版本号
fn compare_forge_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    
    for i in 0..std::cmp::max(a_parts.len(), b_parts.len()) {
        let a_part = a_parts.get(i).unwrap_or(&"0");
        let b_part = b_parts.get(i).unwrap_or(&"0");
        
        let a_num = a_part.parse::<u32>().unwrap_or(0);
        let b_num = b_part.parse::<u32>().unwrap_or(0);
        
        match a_num.cmp(&b_num) {
            std::cmp::Ordering::Equal => continue,
            ordering => return ordering,
        }
    }
    a.cmp(b)
}

/// 安装指定版本的 Forge
pub async fn install_forge(
    instance_path: PathBuf,
    forge_version: ForgeVersion,
) -> Result<(), LauncherError> {
    // 1. 加载配置
    let app_config = config::load_config()?;
    let java_path = app_config
        .java_path
        .ok_or_else(|| LauncherError::Custom("未设置Java路径，无法安装Forge.".to_string()))?;
    let game_dir = std::path::PathBuf::from(&app_config.game_dir);

    info!("Forge: 开始安装流程. MC: {}, Forge: {}", forge_version.mcversion, forge_version.version);

    // 2. 下载安装器
    let installer_filename = format!(
        "forge-{}-{}-installer.jar",
        forge_version.mcversion, forge_version.version
    );
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(&installer_filename);

    // 构建多源下载链接
    let installer_sources = vec![
        format!(
            "{}/forge/download/{}",
            BMCL_API_BASE_URL, forge_version.build
        ),
        format!(
            "{}/maven/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
            BMCL_API_BASE_URL, mc=forge_version.mcversion, v=forge_version.version
        ),
        format!(
            "{}/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
            MAVEN_FORGE, mc=forge_version.mcversion, v=forge_version.version
        ),
    ];

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // 下载安装器
    let mut download_success = false;
    for url in installer_sources {
        info!("Forge: 尝试下载安装器: {}", url);
        if let Ok(response) = download_with_retry(&url, &client, 3).await {
            if let Ok(bytes) = response.bytes().await {
                if bytes.len() > 1024 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                    fs::write(&installer_path, bytes).map_err(|e| LauncherError::Custom(e.to_string()))?;
                    download_success = true;
                    break;
                }
            }
        }
    }

    if !download_success {
        return Err(LauncherError::Custom("安装器下载失败".to_string()));
    }
    info!("Forge: 安装器已就绪: {}", installer_path.display());

    // 3. 预下载必要库 (LaunchWrapper, ASM, LZMA)
    let libraries_dir = game_dir.join("libraries");
    let _ = download_launchwrapper_library(&libraries_dir, &forge_version.mcversion).await;
    let _ = download_asm_library(&libraries_dir, &forge_version.mcversion).await;
    let _ = download_lzma_library(&libraries_dir, &forge_version.mcversion).await;

    // 1.8 特殊处理
    if forge_version.mcversion.starts_with("1.8") {
        let _ = download_forge_core_library(&libraries_dir, &forge_version.mcversion, &forge_version.version).await;
    }

    // 4. 准备占位 launcher_profiles.json
    let launcher_profiles = game_dir.join("launcher_profiles.json");
    if !launcher_profiles.exists() {
        if let Some(p) = launcher_profiles.parent() { fs::create_dir_all(p).ok(); }
        let _ = fs::write(&launcher_profiles, r#"{"profiles":{},"selectedProfile":"","clientToken":"","authenticationDatabase":{}}"#);
    }

    // 5. 执行安装器策略
    // 策略 A: 新版安装器 (--installClient)
    info!("Forge: 执行策略 A (新版安装器)");
    let mut cmd = Command::new(&java_path);
    cmd.current_dir(&game_dir)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient");
    
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output()?;
    let _stdout = String::from_utf8_lossy(&output.stdout); // FIX: 加了下划线前缀
    let stderr = String::from_utf8_lossy(&output.stderr);

    let is_arg_error = !output.status.success() && (stderr.contains("not a recognized option") || stderr.contains("UnrecognizedOptionException"));

    if is_arg_error {
        info!("Forge: 检测到旧版安装器，切换到策略 B");
        
        // 策略 B: 旧版安装器 (Headless + 目录参数)
        let mut cmd_old = Command::new(&java_path);
        cmd_old.current_dir(&game_dir)
            .arg("-Djava.awt.headless=true")
            .arg("-jar")
            .arg(&installer_path)
            .arg("--installClient")
            .arg(game_dir.to_str().unwrap_or("."));
        
        #[cfg(windows)]
        cmd_old.creation_flags(CREATE_NO_WINDOW);

        let mut output_old = cmd_old.output()?;
        let mut stderr_old = String::from_utf8_lossy(&output_old.stderr);

        // 如果还是参数错误，尝试无参数模式 (策略 B2)
        if !output_old.status.success() && (stderr_old.contains("not a recognized option") || stderr_old.contains("UnrecognizedOptionException")) {
            info!("Forge: 尝试策略 B2 (无参数模式)");
            let mut cmd_old2 = Command::new(&java_path);
            cmd_old2.current_dir(&game_dir)
                .arg("-Djava.awt.headless=true")
                .arg("-jar")
                .arg(&installer_path);
            #[cfg(windows)]
            cmd_old2.creation_flags(CREATE_NO_WINDOW);
            output_old = cmd_old2.output()?;
            stderr_old = String::from_utf8_lossy(&output_old.stderr);
        }

        if !output_old.status.success() {
            if stderr_old.contains("HeadlessException") {
                warn!("Forge: 自动安装失败 (HeadlessException)，切换到策略 C (手动解压)");
                manual_install_old_forge(&installer_path, &game_dir, &forge_version).await?;
            } else {
                // 失败清理
                file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
                return Err(LauncherError::Custom(format!("Forge 安装失败: {}", stderr_old)));
            }
        }
    } else if !output.status.success() {
        // 新版安装器运行出错
        file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
        return Err(LauncherError::Custom(format!("Forge 安装失败: {}", stderr)));
    }

    // 6. 清理
    if installer_path.exists() {
        let _ = fs::remove_file(&installer_path);
    }
    
    info!("Forge: 安装成功完成");
    Ok(())
}