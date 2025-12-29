use crate::errors::LauncherError;
use crate::models::ForgeVersion;
use crate::services::config;

use log::{debug, error, info, warn};
use reqwest::Client;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// 常用镜像源常量
const BMCL_API_BASE_URL: &str = "https://bmclapi2.bangbang93.com";
const BMCL_LIBRARIES_URL: &str = "https://bmclapi2.bangbang93.com/libraries";
const MAVEN_FORGE: &str = "https://maven.minecraftforge.net";
const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";
const MAVEN_MINECRAFT: &str = "https://libraries.minecraft.net";

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

        if retry_count > 1 {
            let delay_seconds = std::cmp::min(2u64.pow(retry_count as u32 - 1), 10);
            debug!("Forge: 等待 {} 秒后重试", delay_seconds);
            tokio::time::sleep(std::time::Duration::from_secs(delay_seconds)).await;
        }

        let result = client
            .get(&current_url)
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header(reqwest::header::ACCEPT, "*/*")
            .header(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
            .header(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, br")
            .header(reqwest::header::CONNECTION, "keep-alive")
            .send()
            .await;

        match result {
            Ok(response) => {
                if response.status().is_redirection() {
                    if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
                        if let Ok(redirect_url) = location.to_str() {
                            if !tried_urls.contains(&redirect_url.to_string()) {
                                current_url = redirect_url.to_string();
                                tried_urls.push(current_url.clone());
                                retry_count = 0;
                                continue;
                            }
                        }
                    }
                }

                if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_MODIFIED {
                    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
                        let ct = content_type.to_str().unwrap_or("").to_lowercase();
                        if ct.contains("text/html") {
                            warn!("Forge: 返回了HTML内容，跳过: {}", current_url);
                            continue;
                        }
                    }
                    return Ok(response);
                } else {
                    warn!("Forge: 下载失败，状态: {}", response.status());
                }
            }
            Err(e) => warn!("Forge: 网络错误: {}", e),
        }
    }

    Err(LauncherError::Custom(format!(
        "下载失败: 超过最大重试次数。尝试过的URL: {:?}",
        tried_urls
    )))
}

/// 通用库下载辅助函数
async fn download_library(
    libraries_dir: &Path,
    rel_path: &str,
    sources: Vec<String>,
    lib_name: &str,
) -> Result<(), LauncherError> {
    let target_path = libraries_dir.join(rel_path);

    if target_path.exists() {
        debug!("Forge: {} 库已存在", lib_name);
        return Ok(());
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LauncherError::Custom(format!("创建目录失败: {}", e)))?;
    }

    let client = Client::new();
    for source_url in &sources {
        debug!("Forge: 尝试下载 {}: {}", lib_name, source_url);
        if let Ok(response) = download_with_retry(source_url, &client, 3).await {
            if let Ok(bytes) = response.bytes().await {
                if bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                    fs::write(&target_path, &bytes)
                        .map_err(|e| LauncherError::Custom(format!("写入失败: {}", e)))?;
                    info!("Forge: {} 下载成功", lib_name);
                    return Ok(());
                }
            }
        }
    }

    Err(LauncherError::Custom(format!("所有 {} 下载源均失败", lib_name)))
}

/// 下载 LaunchWrapper 库 (旧版 Forge 需要)
async fn download_launchwrapper_library(libraries_dir: &Path, _mc_version: &str) -> Result<(), LauncherError> {
    let path = "net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar";
    let sources = vec![
        format!("{}/{}", BMCL_LIBRARIES_URL, path),
        format!("{}/{}", MAVEN_MINECRAFT, path),
    ];
    download_library(libraries_dir, path, sources, "LaunchWrapper").await
}

/// 下载 ASM 库
async fn download_asm_library(libraries_dir: &Path, mc_version: &str) -> Result<(), LauncherError> {
    let version = if mc_version.starts_with("1.7") { "5.0.3" } 
                  else if mc_version.starts_with("1.8") || mc_version.starts_with("1.9") 
                       || mc_version.starts_with("1.10") || mc_version.starts_with("1.11") { "5.0.4" }
                  else { "5.2" };
    
    let path = format!("org/ow2/asm/asm-all/{}/asm-all-{}.jar", version, version);
    let sources = vec![
        format!("{}/{}", BMCL_LIBRARIES_URL, path),
        format!("{}/{}", MAVEN_CENTRAL, path),
    ];
    download_library(libraries_dir, &path, sources, "ASM").await
}

/// 下载 LZMA/XZ 库
async fn download_lzma_library(libraries_dir: &Path, mc_version: &str) -> Result<(), LauncherError> {
    let path = if mc_version.starts_with("1.7") {
        "lzma/lzma/0.0.1/lzma-0.0.1.jar".to_string()
    } else {
        "org/tukaani/xz/1.8/xz-1.8.jar".to_string()
    };
    let sources = vec![
        format!("{}/{}", BMCL_LIBRARIES_URL, path),
        format!("{}/{}", MAVEN_CENTRAL, path),
    ];
    download_library(libraries_dir, &path, sources, "LZMA/XZ").await
}

/// 从 Maven 坐标解析路径
fn maven_to_path(name: &str, classifier: Option<&str>, extension: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 { return None; }
    
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
    library: &serde_json::Value,
    libraries_dir: &Path,
    client: &Client,
) -> Result<(), LauncherError> {
    let name = match library["name"].as_str() {
        Some(n) => n,
        None => return Ok(()),
    };

    // 跳过服务端专用库
    if let Some(false) = library.get("clientreq").and_then(|v| v.as_bool()) {
        return Ok(());
    }

    // 优先使用 downloads.artifact 中的路径和 URL
    if let Some(artifact) = library.get("downloads").and_then(|d| d.get("artifact")) {
        if let Some(path) = artifact.get("path").and_then(|p| p.as_str()) {
            let target_path = libraries_dir.join(path);
            if target_path.exists() { return Ok(()); }
            
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            
            let mut sources = Vec::new();
            
            // 使用 artifact 中的 URL
            if let Some(url) = artifact.get("url").and_then(|u| u.as_str()) {
                // BMCLAPI 镜像优先
                let mirrored_url = url
                    .replace("https://libraries.minecraft.net", BMCL_LIBRARIES_URL)
                    .replace("https://maven.minecraftforge.net", &format!("{}/maven", BMCL_API_BASE_URL))
                    .replace("https://maven.neoforged.net/releases", &format!("{}/maven", BMCL_API_BASE_URL));
                if mirrored_url != url {
                    sources.push(mirrored_url);
                }
                sources.push(url.to_string());
            }
            
            // 添加备用源
            sources.push(format!("{}/{}", BMCL_LIBRARIES_URL, path));
            sources.push(format!("{}/{}", MAVEN_FORGE, path));
            sources.push(format!("{}/{}", MAVEN_CENTRAL, path));
            
            for url in &sources {
                if let Ok(resp) = download_with_retry(url, client, 2).await {
                    if let Ok(bytes) = resp.bytes().await {
                        if bytes.len() > 100 {
                            fs::write(&target_path, &bytes).ok();
                            debug!("Forge: 库下载成功 (artifact): {}", name);
                            return Ok(());
                        }
                    }
                }
            }
            
            warn!("Forge: 无法下载库 (artifact): {}", name);
            return Ok(());
        }
    }

    // 回退到从 name 构建路径
    let maven_path = match maven_to_path(name, None, "jar") {
        Some(p) => p,
        None => return Ok(()),
    };

    let target_path = libraries_dir.join(&maven_path);
    if target_path.exists() { return Ok(()); }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).ok();
    }

    let mut sources = Vec::new();
    
    // 优先使用 profile 中指定的 URL
    if let Some(url) = library.get("url").and_then(|u| u.as_str()) {
        let base = if url.ends_with('/') { url.to_string() } else { format!("{}/", url) };
        sources.push(format!("{}{}", base, maven_path));
    }
    
    sources.push(format!("{}/{}", BMCL_LIBRARIES_URL, maven_path));
    sources.push(format!("{}/{}", MAVEN_FORGE, maven_path));
    sources.push(format!("{}/{}", MAVEN_CENTRAL, maven_path));

    for url in &sources {
        if let Ok(resp) = download_with_retry(url, client, 2).await {
            if let Ok(bytes) = resp.bytes().await {
                if bytes.len() > 100 {
                    fs::write(&target_path, &bytes).ok();
                    debug!("Forge: 库下载成功: {}", name);
                    return Ok(());
                }
            }
        }
    }

    warn!("Forge: 无法下载库: {}", name);
    Ok(())
}

/// 从 install_profile 下载所有库 (新版 Forge)
async fn download_libraries_from_new_profile(
    profile: &serde_json::Value,
    libraries_dir: &Path,
    client: &Client,
) -> Result<(), LauncherError> {
    // 新版 Forge 的库在顶层 libraries 数组
    if let Some(libs) = profile.get("libraries").and_then(|l| l.as_array()) {
        info!("Forge: 下载 {} 个库文件", libs.len());
        for lib in libs {
            download_library_from_profile(lib, libraries_dir, client).await?;
        }
    }
    Ok(())
}


/// 执行新版 Forge 的 processors (1.13+)
async fn run_forge_processors(
    profile: &serde_json::Value,
    game_dir: &Path,
    java_path: &str,
    mc_version: &str,
    forge_version: &str,
) -> Result<(), LauncherError> {
    let processors = match profile.get("processors").and_then(|p| p.as_array()) {
        Some(p) => p,
        None => {
            debug!("Forge: 没有 processors 需要执行");
            return Ok(());
        }
    };

    let libraries_dir = game_dir.join("libraries");
    let data = profile.get("data").and_then(|d| d.as_object());

    info!("Forge: 执行 {} 个 processors", processors.len());

    for (idx, processor) in processors.iter().enumerate() {
        // 检查是否只在服务端运行
        if let Some(sides) = processor.get("sides").and_then(|s| s.as_array()) {
            let is_client = sides.iter().any(|s| s.as_str() == Some("client"));
            if !is_client {
                debug!("Forge: 跳过服务端 processor {}", idx);
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

        // 构建 classpath
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

        // 构建参数
        let mut args: Vec<String> = Vec::new();
        if let Some(proc_args) = processor.get("args").and_then(|a| a.as_array()) {
            for arg in proc_args {
                if let Some(arg_str) = arg.as_str() {
                    let resolved = resolve_processor_arg(arg_str, data, game_dir, &libraries_dir, mc_version, forge_version);
                    args.push(resolved);
                }
            }
        }

        // 获取主类
        let main_class = get_jar_main_class(&jar_path)?;

        info!("Forge: 执行 processor {}/{}: {}", idx + 1, processors.len(), main_class);
        debug!("Forge: Classpath: {:?}", classpath);
        debug!("Forge: Args: {:?}", args);

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

        let output = cmd.output()
            .map_err(|e| LauncherError::Custom(format!("执行 processor 失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!("Forge: Processor {} 失败:\nstdout: {}\nstderr: {}", idx, stdout, stderr);
            // 某些 processor 失败可能不是致命的，继续执行
        } else {
            debug!("Forge: Processor {} 完成", idx + 1);
        }
    }

    Ok(())
}

/// 解析 processor 参数中的变量
fn resolve_processor_arg(
    arg: &str,
    data: Option<&serde_json::Map<String, serde_json::Value>>,
    game_dir: &Path,
    libraries_dir: &Path,
    mc_version: &str,
    forge_version: &str,
) -> String {
    let mut result = arg.to_string();

    // 处理 {KEY} 格式的数据引用
    if arg.starts_with('{') && arg.ends_with('}') {
        let key = &arg[1..arg.len()-1];
        if let Some(data_map) = data {
            if let Some(value) = data_map.get(key) {
                if let Some(val_obj) = value.as_object() {
                    if let Some(client_val) = val_obj.get("client").and_then(|v| v.as_str()) {
                        result = resolve_data_value(client_val, libraries_dir, game_dir, mc_version, forge_version);
                    }
                } else if let Some(val_str) = value.as_str() {
                    result = resolve_data_value(val_str, libraries_dir, game_dir, mc_version, forge_version);
                }
            }
        }
    }
    // 处理 [artifact] 格式的库引用
    else if arg.starts_with('[') && arg.ends_with(']') {
        let artifact = &arg[1..arg.len()-1];
        if let Some(path) = maven_to_path(artifact, None, "jar") {
            result = libraries_dir.join(path).to_string_lossy().to_string();
        }
    }

    result
}

/// 解析数据值中的路径
fn resolve_data_value(value: &str, libraries_dir: &Path, game_dir: &Path, mc_version: &str, forge_version: &str) -> String {
    if value.starts_with('[') && value.ends_with(']') {
        let artifact = &value[1..value.len()-1];
        if let Some(path) = maven_to_path(artifact, None, "jar") {
            return libraries_dir.join(path).to_string_lossy().to_string();
        }
    }
    
    if value.starts_with('/') {
        // 相对于 game_dir 的路径
        return game_dir.join(&value[1..]).to_string_lossy().to_string();
    }

    // 替换常见变量
    value
        .replace("{MINECRAFT_VERSION}", mc_version)
        .replace("{FORGE_VERSION}", forge_version)
        .replace("{ROOT}", &game_dir.to_string_lossy())
        .replace("{LIBRARY_DIR}", &libraries_dir.to_string_lossy())
}

/// 从 JAR 文件获取主类
fn get_jar_main_class(jar_path: &Path) -> Result<String, LauncherError> {
    let file = fs::File::open(jar_path)
        .map_err(|e| LauncherError::Custom(format!("无法打开 JAR: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| LauncherError::Custom(format!("无法读取 JAR: {}", e)))?;

    let mut manifest = match archive.by_name("META-INF/MANIFEST.MF") {
        Ok(f) => f,
        Err(_) => return Err(LauncherError::Custom("JAR 中没有 MANIFEST.MF".to_string())),
    };

    let mut content = String::new();
    manifest.read_to_string(&mut content)
        .map_err(|e| LauncherError::Custom(format!("读取 MANIFEST 失败: {}", e)))?;

    for line in content.lines() {
        if line.starts_with("Main-Class:") {
            return Ok(line.trim_start_matches("Main-Class:").trim().to_string());
        }
    }

    Err(LauncherError::Custom("MANIFEST 中没有 Main-Class".to_string()))
}


/// 手动安装旧版本 Forge (1.12.2 及以下)
async fn manual_install_old_forge(
    installer_path: &Path,
    game_dir: &Path,
    forge_version: &ForgeVersion,
) -> Result<(), LauncherError> {
    info!("Forge: 开始手动安装旧版本 Forge");

    let file = fs::File::open(installer_path)
        .map_err(|e| LauncherError::Custom(format!("无法打开安装器: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| LauncherError::Custom(format!("无法读取安装器: {}", e)))?;

    // 读取 install_profile.json
    let profile: serde_json::Value = {
        let mut content = String::new();
        archive.by_name("install_profile.json")
            .map_err(|_| LauncherError::Custom("安装器中未找到 install_profile.json".to_string()))?
            .read_to_string(&mut content)
            .map_err(|e| LauncherError::Custom(format!("读取失败: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| LauncherError::Custom(format!("解析失败: {}", e)))?
    };

    let libraries_dir = game_dir.join("libraries");
    let client = Client::new();

    // 下载库文件
    if let Some(libs) = profile.get("versionInfo").and_then(|v| v.get("libraries")).and_then(|l| l.as_array()) {
        info!("Forge: 下载 {} 个库", libs.len());
        for lib in libs {
            let _ = download_library_from_profile(lib, &libraries_dir, &client).await;
        }
    }

    // 创建版本目录和 JSON
    let version_id = get_forge_version_id(&forge_version.mcversion, &forge_version.version);
    let version_dir = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&version_dir)
        .map_err(|e| LauncherError::Custom(format!("创建版本目录失败: {}", e)))?;

    let mut version_info = profile.get("versionInfo")
        .ok_or_else(|| LauncherError::Custom("缺少 versionInfo".to_string()))?
        .clone();

    // 旧版 Forge (1.7.x, 1.9.x, 1.10) 使用 mc-forge-mc 格式
    let needs_old_format = forge_version.mcversion.starts_with("1.7") 
        || forge_version.mcversion.starts_with("1.9")
        || forge_version.mcversion == "1.10";

    if let serde_json::Value::Object(ref mut obj) = version_info {
        obj.insert("id".to_string(), serde_json::json!(version_id));
        obj.insert("inheritsFrom".to_string(), serde_json::json!(forge_version.mcversion));
        obj.insert("jar".to_string(), serde_json::json!(forge_version.mcversion));
        obj.insert("mainClass".to_string(), serde_json::json!("net.minecraft.launchwrapper.Launch"));

        // 修复库路径中的 Forge 版本格式
        if needs_old_format {
            if let Some(libs) = obj.get_mut("libraries").and_then(|l| l.as_array_mut()) {
                for lib in libs.iter_mut() {
                    if let Some(name) = lib.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()) {
                        // 修复 net.minecraftforge:forge:mc-version 格式
                        if name.contains("net.minecraftforge:forge:") {
                            let old_name = format!("net.minecraftforge:forge:{}-{}", 
                                forge_version.mcversion, forge_version.version);
                            let new_name = format!("net.minecraftforge:forge:{}-{}-{}", 
                                forge_version.mcversion, forge_version.version, forge_version.mcversion);
                            if name == old_name {
                                if let Some(lib_obj) = lib.as_object_mut() {
                                    lib_obj.insert("name".to_string(), serde_json::json!(new_name));
                                    info!("Forge: 修复库路径 {} -> {}", old_name, new_name);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 处理启动参数
        let mut args = String::new();
        if let Some(serde_json::Value::String(s)) = obj.get("minecraftArguments") {
            args = s.clone();
        }
        
        // 根据版本确定正确的 FMLTweaker 类路径
        // 1.7.x 使用 cpw.mods.fml.common.launcher.FMLTweaker
        // 1.8+ 使用 net.minecraftforge.fml.common.launcher.FMLTweaker
        let tweaker_class = if forge_version.mcversion.starts_with("1.7") {
            "cpw.mods.fml.common.launcher.FMLTweaker"
        } else {
            "net.minecraftforge.fml.common.launcher.FMLTweaker"
        };
        
        // 确保 FMLTweaker 存在
        if !args.contains("FMLTweaker") {
            args = format!("--tweakClass {} {}", tweaker_class, args);
        }
        obj.insert("minecraftArguments".to_string(), serde_json::json!(args.trim()));
    }

    let version_json_path = version_dir.join(format!("{}.json", version_id));
    fs::write(&version_json_path, serde_json::to_string_pretty(&version_info)
        .map_err(|e| LauncherError::Custom(format!("序列化失败: {}", e)))?)
        .map_err(|e| LauncherError::Custom(format!("写入版本 JSON 失败: {}", e)))?;

    // 重新打开 ZIP 提取文件
    let file = fs::File::open(installer_path)
        .map_err(|e| LauncherError::Custom(format!("重新打开安装器失败: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| LauncherError::Custom(format!("重新读取安装器失败: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let file_name = file.name().to_string();

        // 提取 maven 目录下的库
        if file_name.starts_with("maven/") && !file_name.ends_with('/') {
            if let Some(rel_path) = file_name.strip_prefix("maven/") {
                let target = libraries_dir.join(rel_path);
                if let Some(p) = target.parent() { fs::create_dir_all(p).ok(); }
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    fs::write(&target, &buf).ok();
                }
            }
        }
        // 提取 universal.jar
        else if file_name.ends_with("-universal.jar") && !file_name.contains('/') {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                // 旧版 Forge (1.7.x, 1.9.x, 1.10) 使用 mc-forge-mc 格式
                let needs_old_format = forge_version.mcversion.starts_with("1.7") 
                    || forge_version.mcversion.starts_with("1.9")
                    || forge_version.mcversion == "1.10";
                
                let forge_lib = if needs_old_format {
                    format!(
                        "net/minecraftforge/forge/{mc}-{v}-{mc}/forge-{mc}-{v}-{mc}-universal.jar",
                        mc = forge_version.mcversion, v = forge_version.version
                    )
                } else {
                    format!(
                        "net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-universal.jar",
                        mc = forge_version.mcversion, v = forge_version.version
                    )
                };
                let lib_target = libraries_dir.join(&forge_lib);
                if let Some(p) = lib_target.parent() { fs::create_dir_all(p).ok(); }
                fs::write(&lib_target, &buf).ok();
                info!("Forge: 已提取 Universal JAR 到 {}", forge_lib);
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

    let file = fs::File::open(installer_path)
        .map_err(|e| LauncherError::Custom(format!("无法打开安装器: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| LauncherError::Custom(format!("无法读取安装器: {}", e)))?;

    // 读取 install_profile.json
    let profile: serde_json::Value = {
        let mut content = String::new();
        archive.by_name("install_profile.json")
            .map_err(|_| LauncherError::Custom("未找到 install_profile.json".to_string()))?
            .read_to_string(&mut content)
            .map_err(|e| LauncherError::Custom(format!("读取失败: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| LauncherError::Custom(format!("解析失败: {}", e)))?
    };

    // 读取 version.json (新版 Forge 的版本信息)
    let version_json: serde_json::Value = {
        let mut content = String::new();
        if let Ok(mut f) = archive.by_name("version.json") {
            f.read_to_string(&mut content).ok();
        }
        if content.is_empty() {
            // 某些版本可能没有单独的 version.json
            serde_json::json!({})
        } else {
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        }
    };

    let libraries_dir = game_dir.join("libraries");
    let client = Client::new();

    // 下载 install_profile.json 中的库
    download_libraries_from_new_profile(&profile, &libraries_dir, &client).await?;

    // 下载 version.json 中的库（这些是运行时需要的库）
    if let Some(libs) = version_json.get("libraries").and_then(|l| l.as_array()) {
        info!("Forge: 下载 version.json 中的 {} 个库文件", libs.len());
        for lib in libs {
            if let Err(e) = download_library_from_profile(lib, &libraries_dir, &client).await {
                warn!("Forge: 下载库失败: {}", e);
            }
        }
    }

    // 提取 maven 目录中的文件
    let file = fs::File::open(installer_path)
        .map_err(|e| LauncherError::Custom(format!("重新打开失败: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| LauncherError::Custom(format!("重新读取失败: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let name = file.name().to_string();

        if name.starts_with("maven/") && !name.ends_with('/') {
            if let Some(rel) = name.strip_prefix("maven/") {
                let target = libraries_dir.join(rel);
                if let Some(p) = target.parent() { fs::create_dir_all(p).ok(); }
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    fs::write(&target, &buf).ok();
                }
            }
        }
        // 提取 data 目录中的文件 (BINPATCH 等)
        else if name.starts_with("data/") && !name.ends_with('/') {
            if let Some(rel) = name.strip_prefix("data/") {
                let target = libraries_dir.join("net/minecraftforge/forge")
                    .join(format!("{}-{}", forge_version.mcversion, forge_version.version))
                    .join(rel);
                if let Some(p) = target.parent() { fs::create_dir_all(p).ok(); }
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    fs::write(&target, &buf).ok();
                }
            }
        }
    }

    // 执行 processors
    run_forge_processors(&profile, game_dir, java_path, &forge_version.mcversion, &forge_version.version).await?;

    // 创建版本 JSON
    let version_id = get_forge_version_id(&forge_version.mcversion, &forge_version.version);
    let version_dir = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&version_dir)
        .map_err(|e| LauncherError::Custom(format!("创建版本目录失败: {}", e)))?;

    let mut final_version = if version_json.is_object() && !version_json.as_object().unwrap().is_empty() {
        version_json.clone()
    } else {
        serde_json::json!({})
    };

    if let serde_json::Value::Object(ref mut obj) = final_version {
        obj.insert("id".to_string(), serde_json::json!(version_id));
        obj.insert("inheritsFrom".to_string(), serde_json::json!(forge_version.mcversion));
        
        // 新版 Forge 使用 ModLauncher
        if !obj.contains_key("mainClass") {
            obj.insert("mainClass".to_string(), serde_json::json!("cpw.mods.modlauncher.Launcher"));
        }
    }

    let version_json_path = version_dir.join(format!("{}.json", version_id));
    fs::write(&version_json_path, serde_json::to_string_pretty(&final_version)
        .map_err(|e| LauncherError::Custom(format!("序列化失败: {}", e)))?)
        .map_err(|e| LauncherError::Custom(format!("写入版本 JSON 失败: {}", e)))?;

    info!("Forge: 新版手动安装完成");
    Ok(())
}


/// 获取 Forge 版本列表
pub async fn get_forge_versions(minecraft_version: String) -> Result<Vec<ForgeVersion>, LauncherError> {
    let client = Client::new();
    let url = format!("{}/forge/minecraft/{}", BMCL_API_BASE_URL, minecraft_version);

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

/// 安装 Forge
pub async fn install_forge(
    _instance_path: PathBuf,
    forge_version: ForgeVersion,
) -> Result<(), LauncherError> {
    let app_config = config::load_config()?;
    let java_path = app_config.java_path
        .ok_or_else(|| LauncherError::Custom("未设置 Java 路径".to_string()))?;
    let game_dir = PathBuf::from(&app_config.game_dir);

    info!("Forge: 安装 MC {} + Forge {}", forge_version.mcversion, forge_version.version);

    // 下载安装器
    let installer_filename = format!("forge-{}-{}-installer.jar", forge_version.mcversion, forge_version.version);
    let installer_path = std::env::temp_dir().join(&installer_filename);

    // 判断是否需要使用旧版 URL 格式 (1.7.x, 1.9.x 需要 mc-forge-mc 格式)
    let needs_old_format = forge_version.mcversion.starts_with("1.7") 
        || forge_version.mcversion.starts_with("1.9")
        || forge_version.mcversion == "1.10";
    
    let sources = if needs_old_format {
        // 旧版格式: forge-1.7.10-10.13.4.1614-1.7.10-installer.jar
        // BMCLAPI 优先
        vec![
            format!("{}/net/minecraftforge/forge/{mc}-{v}-{mc}/forge-{mc}-{v}-{mc}-installer.jar",
                BMCL_LIBRARIES_URL, mc=forge_version.mcversion, v=forge_version.version),
            format!("{}/net/minecraftforge/forge/{mc}-{v}-{mc}/forge-{mc}-{v}-{mc}-installer.jar",
                MAVEN_FORGE, mc=forge_version.mcversion, v=forge_version.version),
            // 备用：尝试标准格式
            format!("{}/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
                BMCL_LIBRARIES_URL, mc=forge_version.mcversion, v=forge_version.version),
            format!("{}/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
                MAVEN_FORGE, mc=forge_version.mcversion, v=forge_version.version),
        ]
    } else {
        // 标准格式: forge-1.12.2-14.23.5.2860-installer.jar
        // BMCLAPI 优先
        vec![
            format!("{}/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
                BMCL_LIBRARIES_URL, mc=forge_version.mcversion, v=forge_version.version),
            format!("{}/net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-installer.jar",
                MAVEN_FORGE, mc=forge_version.mcversion, v=forge_version.version),
        ]
    };

    let client = Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut downloaded = false;
    for url in &sources {
        info!("Forge: 尝试下载: {}", url);
        if let Ok(resp) = download_with_retry(url, &client, 3).await {
            if let Ok(bytes) = resp.bytes().await {
                if bytes.len() > 1024 && bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
                    fs::write(&installer_path, &bytes)
                        .map_err(|e| LauncherError::Custom(format!("写入安装器失败: {}", e)))?;
                    downloaded = true;
                    break;
                }
            }
        }
    }

    if !downloaded {
        return Err(LauncherError::Custom("安装器下载失败".to_string()));
    }
    info!("Forge: 安装器已下载");

    // 预下载必要库 (旧版 Forge)
    if !is_new_forge(&forge_version.mcversion) {
        let libs_dir = game_dir.join("libraries");
        let _ = download_launchwrapper_library(&libs_dir, &forge_version.mcversion).await;
        let _ = download_asm_library(&libs_dir, &forge_version.mcversion).await;
        let _ = download_lzma_library(&libs_dir, &forge_version.mcversion).await;
    }

    // 准备 launcher_profiles.json
    let profiles_path = game_dir.join("launcher_profiles.json");
    if !profiles_path.exists() {
        fs::write(&profiles_path, r#"{"profiles":{}}"#).ok();
    }

    // 尝试使用官方安装器
    info!("Forge: 尝试官方安装器");
    let install_result = run_official_installer(&installer_path, &game_dir, &java_path).await;

    match install_result {
        Ok(()) => {
            info!("Forge: 官方安装器成功");
        }
        Err(e) => {
            warn!("Forge: 官方安装器失败: {}, 尝试手动安装", e);
            
            if is_new_forge(&forge_version.mcversion) {
                manual_install_new_forge(&installer_path, &game_dir, &forge_version, &java_path).await?;
            } else {
                manual_install_old_forge(&installer_path, &game_dir, &forge_version).await?;
            }
        }
    }

    // 清理
    if installer_path.exists() {
        fs::remove_file(&installer_path).ok();
    }

    info!("Forge: 安装完成");
    Ok(())
}

/// 运行官方安装器
async fn run_official_installer(
    installer_path: &Path,
    game_dir: &Path,
    java_path: &str,
) -> Result<(), LauncherError> {
    // 策略 1: --installClient (新版安装器)
    let mut cmd = Command::new(java_path);
    cmd.current_dir(game_dir)
        .arg("-jar")
        .arg(installer_path)
        .arg("--installClient");

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output()
        .map_err(|e| LauncherError::Custom(format!("执行安装器失败: {}", e)))?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // 检查是否是参数不支持的旧版安装器
    if stderr.contains("not a recognized option") || stderr.contains("UnrecognizedOptionException") {
        // 策略 2: headless 模式 (旧版安装器)
        let mut cmd2 = Command::new(java_path);
        cmd2.current_dir(game_dir)
            .arg("-Djava.awt.headless=true")
            .arg("-jar")
            .arg(installer_path);

        #[cfg(windows)]
        cmd2.creation_flags(CREATE_NO_WINDOW);

        let output2 = cmd2.output()
            .map_err(|e| LauncherError::Custom(format!("执行安装器失败: {}", e)))?;

        if !output2.status.success() {
            let stderr2 = String::from_utf8_lossy(&output2.stderr);
            if stderr2.contains("HeadlessException") {
                return Err(LauncherError::Custom("安装器需要 GUI，切换到手动安装".to_string()));
            }
            return Err(LauncherError::Custom(format!("安装器失败: {}", stderr2)));
        }
    } else if !output.status.success() {
        return Err(LauncherError::Custom(format!("安装器失败: {}", stderr)));
    }

    Ok(())
}
