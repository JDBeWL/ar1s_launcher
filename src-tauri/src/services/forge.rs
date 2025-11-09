use crate::errors::LauncherError;
use crate::models::ForgeVersion;
use crate::services::config;
use crate::utils::file_utils;

use reqwest::Client;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use zip::ZipArchive;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

const BMCL_API_BASE_URL: &str = "https://bmclapi2.bangbang93.com";

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
        
        println!("Forge: 下载尝试第{}次: {}", retry_count, current_url);

        // 添加重试延迟（指数退避）
        if retry_count > 1 {
            let delay_seconds = std::cmp::min(2u64.pow(retry_count as u32 - 1), 10);
            println!("Forge: 等待 {} 秒后重试", delay_seconds);
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
                            println!("Forge: 检测到重定向到: {}", redirect_url);
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
                if response.status().is_success() {
                    // 检查是否为有效的JAR文件
                    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
                        let content_type_str = content_type.to_str().unwrap_or("").to_lowercase();
                        if content_type_str.contains("text/html") || content_type_str.contains("application/json") {
                            println!("Forge: 返回了HTML/JSON内容，跳过");
                            continue;
                        }
                    }

                    // 检查文件大小
                    if let Some(content_length) = response.headers().get(reqwest::header::CONTENT_LENGTH) {
                        if let Ok(length_str) = content_length.to_str() {
                            if let Ok(file_size) = length_str.parse::<u64>() {
                                if file_size < 1024 {
                                    println!("Forge: 文件大小异常（{}字节），跳过", file_size);
                                    continue;
                                }
                                println!("Forge: 文件大小: {} 字节", file_size);
                            }
                        }
                    }

                    return Ok(response);
                } else {
                    println!("Forge: 下载失败，状态: {}", response.status());
                    
                    // 如果是最后一次重试，返回错误
                    if retry_count == max_retries {
                        return Err(LauncherError::Custom(format!(
                            "下载失败: 最终状态 {}。已尝试的URL: {}",
                            response.status(),
                            tried_urls.join(", ")
                        )));
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                println!("Forge: 网络错误: {}", error_msg);

                // 如果是最后一次重试，返回错误
                if retry_count == max_retries {
                    return Err(LauncherError::Custom(format!(
                        "下载失败: {}。已尝试的URL: {}",
                        error_msg,
                        tried_urls.join(", ")
                    )));
                }
            }
        }
    }

    Err(LauncherError::Custom("下载失败: 超过最大重试次数".to_string()))
}

/// 多源下载函数，依次尝试不同的下载源
async fn download_from_multiple_sources(
    forge_version: &ForgeVersion,
    client: &Client,
) -> Result<reqwest::Response, LauncherError> {
    let sources = vec![
        // 源1: 标准bmcl下载地址
        format!(
            "https://bmclapi2.bangbang93.com/forge/download/{}",
            forge_version.build
        ),
        // 源2: bmcl Maven镜像
        {
            let maven_path = format!(
                "{mc}-{ver}",
                mc = forge_version.mcversion,
                ver = forge_version.version
            );
            format!(
                "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{path}/forge-{path}-installer.jar",
                path = maven_path
            )
        },
        // 源3: Forge官方Maven
        {
            let maven_path = format!(
                "{mc}-{ver}",
                mc = forge_version.mcversion,
                ver = forge_version.version
            );
            format!(
                "https://maven.minecraftforge.net/net/minecraftforge/forge/{path}/forge-{path}-installer.jar",
                path = maven_path
            )
        },
        // 源4: 备用bmcl下载地址
        format!(
            "https://bmclapi2.bangbang93.com/forge/download/{}?format=jar",
            forge_version.build
        ),
    ];

    for (index, source_url) in sources.iter().enumerate() {
        println!("Forge: 尝试下载源 {}: {}", index + 1, source_url);
        
        match download_with_retry(source_url, client, 3).await {
            Ok(response) => {
                println!("Forge: 源 {} 下载成功", index + 1);
                return Ok(response);
            }
            Err(e) => {
                println!("Forge: 源 {} 下载失败: {}", index + 1, e);
                // 继续尝试下一个源
            }
        }
    }

    Err(LauncherError::Custom("所有下载源均失败".to_string()))
}

/// 自动下载 LaunchWrapper 库
async fn download_launchwrapper_library(
    libraries_dir: &PathBuf,
    mc_version: &str,
) -> Result<(), LauncherError> {
    println!("Forge: 开始下载 LaunchWrapper 库");
    
    // 根据 Minecraft 版本选择合适的 LaunchWrapper 版本
    let launchwrapper_version = if mc_version.starts_with("1.12") {
        "1.12"
    } else if mc_version.starts_with("1.11") || mc_version.starts_with("1.10") {
        "1.12" // 1.10-1.11 也使用 1.12 版本
    } else if mc_version.starts_with("1.9") || mc_version.starts_with("1.8") {
        "1.12" // 1.8-1.9 也使用 1.12 版本
    } else {
        "1.12" // 默认使用 1.12 版本
    };
    
    let launchwrapper_path = format!(
        "net/minecraft/launchwrapper/{}/launchwrapper-{}.jar",
        launchwrapper_version, launchwrapper_version
    );
    let target_path = libraries_dir.join(&launchwrapper_path);
    
    // 如果库文件已存在，跳过下载
    if target_path.exists() {
        println!("Forge: LaunchWrapper 库已存在: {}", target_path.display());
        return Ok(());
    }
    
    // 创建目录
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LauncherError::Custom(format!("创建 LaunchWrapper 目录失败: {}", e)))?;
    }
    
    // 构建下载源列表
    let sources = vec![
        format!(
            "https://bmclapi2.bangbang93.com/maven/net/minecraft/launchwrapper/{}/launchwrapper-{}.jar",
            launchwrapper_version, launchwrapper_version
        ),
        format!(
            "https://repo1.maven.org/maven2/net/minecraft/launchwrapper/{}/launchwrapper-{}.jar",
            launchwrapper_version, launchwrapper_version
        ),
        format!(
            "https://libraries.minecraft.net/net/minecraft/launchwrapper/{}/launchwrapper-{}.jar",
            launchwrapper_version, launchwrapper_version
        ),
    ];
    
    let client = Client::new();
    
    for (index, source_url) in sources.iter().enumerate() {
        println!("Forge: 尝试下载源 {}: {}", index + 1, source_url);
        
        match download_with_retry(source_url, &client, 3).await {
            Ok(response) => {
                let bytes = response.bytes().await?;
                
                // 验证 JAR 文件
                if bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                    fs::write(&target_path, &bytes)
                        .map_err(|e| LauncherError::Custom(format!("写入 LaunchWrapper 库失败: {}", e)))?;
                    println!("Forge: LaunchWrapper 库下载成功: {}", target_path.display());
                    return Ok(());
                } else {
                    println!("Forge: 源 {} 返回的文件不是有效的 JAR 格式", index + 1);
                }
            }
            Err(e) => {
                println!("Forge: 源 {} 下载失败: {}", index + 1, e);
            }
        }
    }
    
    Err(LauncherError::Custom("所有 LaunchWrapper 下载源均失败".to_string()))
}

/// 自动下载 ASM 库（Forge 字节码操作需要）
async fn download_asm_library(
    libraries_dir: &PathBuf,
    mc_version: &str,
) -> Result<(), LauncherError> {
    println!("Forge: 开始下载 ASM 库");
    
    // 根据 Minecraft 版本选择合适的 ASM 版本
    let asm_version = if mc_version.starts_with("1.7.10") {
        "5.0.3" // 1.7.10 Forge 使用 ASM 5.0.3
    } else if mc_version.starts_with("1.8") || mc_version.starts_with("1.9") || mc_version.starts_with("1.10") || mc_version.starts_with("1.11") {
        "5.0.4" // 1.8-1.11 使用 ASM 5.0.4
    } else {
        "5.2" // 1.12+ 使用更新的 ASM 版本
    };
    
    let asm_path = format!(
        "org/ow2/asm/asm-all/{}/asm-all-{}.jar",
        asm_version, asm_version
    );
    let target_path = libraries_dir.join(&asm_path);
    
    // 如果库文件已存在，跳过下载
    if target_path.exists() {
        println!("Forge: ASM 库已存在: {}", target_path.display());
        return Ok(());
    }
    
    // 创建目录
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LauncherError::Custom(format!("创建 ASM 目录失败: {}", e)))?;
    }
    
    // 构建下载源列表
    let sources = vec![
        format!(
            "https://bmclapi2.bangbang93.com/maven/org/ow2/asm/asm-all/{}/asm-all-{}.jar",
            asm_version, asm_version
        ),
        format!(
            "https://repo1.maven.org/maven2/org/ow2/asm/asm-all/{}/asm-all-{}.jar",
            asm_version, asm_version
        ),
        format!(
            "https://libraries.minecraft.net/org/ow2/asm/asm-all/{}/asm-all-{}.jar",
            asm_version, asm_version
        ),
    ];
    
    let client = Client::new();
    
    for (index, source_url) in sources.iter().enumerate() {
        println!("Forge: 尝试下载源 {}: {}", index + 1, source_url);
        
        match download_with_retry(source_url, &client, 3).await {
            Ok(response) => {
                let bytes = response.bytes().await?;
                
                // 验证 JAR 文件
                if bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                    fs::write(&target_path, &bytes)
                        .map_err(|e| LauncherError::Custom(format!("写入 ASM 库失败: {}", e)))?;
                    println!("Forge: ASM 库下载成功: {}", target_path.display());
                    return Ok(());
                } else {
                    println!("Forge: 源 {} 返回的文件不是有效的 JAR 格式", index + 1);
                }
            }
            Err(e) => {
                println!("Forge: 源 {} 下载失败: {}", index + 1, e);
            }
        }
    }
    
    Err(LauncherError::Custom("所有 ASM 下载源均失败".to_string()))
}

/// 自动下载 LZMA 库（Forge 压缩文件处理需要）
async fn download_lzma_library(
    libraries_dir: &PathBuf,
    mc_version: &str,
) -> Result<(), LauncherError> {
    println!("Forge: 开始下载 LZMA 库");

    // 根据 Minecraft 版本选择合适的 LZMA 版本
    let (lzma_path, sources) = if mc_version.starts_with("1.7.10") {
        // Forge 1.7.10 实际上需要的是 lzma:lzma:0.0.1
        let lzma_version = "0.0.1";
        let path = format!("lzma/lzma/{v}/lzma-{v}.jar", v = lzma_version);
        let urls = vec![
            format!(
                "https://bmclapi2.bangbang93.com/maven/lzma/lzma/{v}/lzma-{v}.jar",
                v = lzma_version
            ),
            format!(
                "https://files.minecraftforge.net/maven/lzma/lzma/{v}/lzma-{v}.jar",
                v = lzma_version
            ),
            format!(
                "https://repo1.maven.org/maven2/lzma/lzma/{v}/lzma-{v}.jar",
                v = lzma_version
            ),
        ];
        (path, urls)
    } else {
        // 其他版本使用 xz 库
        let lzma_version = "1.8";
        let path = format!("org/tukaani/xz/{v}/xz-{v}.jar", v = lzma_version);
        let urls = vec![
            format!(
                "https://bmclapi2.bangbang93.com/maven/org/tukaani/xz/{}/xz-{}.jar",
                lzma_version, lzma_version
            ),
            format!(
                "https://repo1.maven.org/maven2/org/tukaani/xz/{}/xz-{}.jar",
                lzma_version, lzma_version
            ),
            format!(
                "https://libraries.minecraft.net/org/tukaani/xz/{}/xz-{}.jar",
                lzma_version, lzma_version
            ),
        ];
        (path, urls)
    };

    let target_path = libraries_dir.join(&lzma_path);

    // 如果库文件已存在，跳过下载
    if target_path.exists() {
        println!("Forge: LZMA 库已存在: {}", target_path.display());
        return Ok(());
    }

    // 创建目录
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LauncherError::Custom(format!("创建 LZMA 目录失败: {}", e)))?;
    }

    let client = Client::new();

    for (index, source_url) in sources.iter().enumerate() {
        println!("Forge: 尝试下载源 {}: {}", index + 1, source_url);

        match download_with_retry(source_url, &client, 3).await {
            Ok(response) => {
                let bytes = response.bytes().await?;

                // 验证 JAR 文件
                if bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
                    fs::write(&target_path, &bytes)
                        .map_err(|e| LauncherError::Custom(format!("写入 LZMA 库失败: {}", e)))?;
                    println!("Forge: LZMA 库下载成功: {}", target_path.display());
                    return Ok(());
                } else {
                    println!("Forge: 源 {} 返回的文件不是有效的 JAR 格式", index + 1);
                }
            }
            Err(e) => {
                println!("Forge: 源 {} 下载失败: {}", index + 1, e);
            }
        }
    }

    Err(LauncherError::Custom("所有 LZMA 下载源均失败".to_string()))
}

/// 从 install_profile.json 下载单个库
async fn download_library_from_profile(
    library: &serde_json::Value,
    libraries_dir: &PathBuf,
    client: &Client,
) -> Result<(), LauncherError> {
    let name = library["name"].as_str().ok_or_else(|| {
        LauncherError::Custom("Library object missing 'name' field".to_string())
    })?;

    // 仅下载客户端需要的库
    if let Some(clientreq) = library.get("clientreq").and_then(|v| v.as_bool()) {
        if !clientreq {
            println!("Forge: Skipping server-only library: {}", name);
            return Ok(());
        }
    }

    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() != 3 {
        println!("Forge: Skipping library with non-standard name format: {}", name);
        return Ok(());
    }
    let group_id = parts[0];
    let artifact_id = parts[1];
    let version = parts[2];

    let group_path = group_id.replace('.', "/");
    let artifact_path = format!("{}/{}/{}", group_path, artifact_id, version);
    let file_name = format!("{}-{}.jar", artifact_id, version);
    let maven_path = format!("{}/{}", artifact_path, file_name);

    let target_path = libraries_dir.join(&maven_path);

    if target_path.exists() {
        // TODO: Add checksum validation if available in profile
        return Ok(());
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| LauncherError::Custom(format!("Failed to create directory for library {}: {}", name, e)))?;
    }

    let mut sources = Vec::new();
    if let Some(url) = library.get("url").and_then(|u| u.as_str()) {
        // Ensure the custom URL ends with a slash
        let base_url = if url.ends_with('/') {
            url.to_string()
        } else {
            format!("{}/", url)
        };
        sources.push(format!("{}{}", base_url, maven_path));
    }
    // Add default repositories
    sources.push(format!("{}/maven/{}", BMCL_API_BASE_URL, maven_path));
    sources.push(format!("https://libraries.minecraft.net/{}", maven_path));
    sources.push(format!("https://repo1.maven.org/maven2/{}", maven_path));
    sources.push(format!("https://files.minecraftforge.net/maven/{}", maven_path));
    
    sources.dedup();

    for source_url in sources {
        println!("Forge: Attempting to download library {}: {}", name, source_url);
        match download_with_retry(&source_url, client, 3).await {
            Ok(response) => {
                let bytes = response.bytes().await?;
                if bytes.len() > 100 { // Basic JAR check
                    fs::write(&target_path, &bytes)
                        .map_err(|e| LauncherError::Custom(format!("Failed to write library {}: {}", name, e)))?;
                    println!("Forge: Library {} downloaded successfully", name);
                    return Ok(());
                } else {
                    println!("Forge: Downloaded file for {} from {} is too small, likely not a valid JAR.", name, source_url);
                }
            }
            Err(e) => {
                println!("Forge: Failed to download {} from {}: {}", name, source_url, e);
            }
        }
    }

    Err(LauncherError::Custom(format!("Failed to download library {} from all sources", name)))
}

/// Manually installs old Forge versions by extracting and copying files
async fn manual_install_old_forge(
    installer_path: &PathBuf,
    game_dir: &PathBuf,
    forge_version: &ForgeVersion,
) -> Result<(), LauncherError> {
    println!("Forge: 开始手动安装旧版本 Forge");

    // 1. 打开安装器 JAR 文件
    let file = fs::File::open(installer_path)
        .map_err(|e| LauncherError::Custom(format!("无法打开安装器文件: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| LauncherError::Custom(format!("无法读取安装器 ZIP: {}", e)))?;

    // 2. 查找并读取 install_profile.json
    let mut install_profile_content = String::new();
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| LauncherError::Custom(format!("无法读取 ZIP 条目: {}", e)))?;
        if file.name() == "install_profile.json" {
            file.read_to_string(&mut install_profile_content)
                .map_err(|e| {
                    LauncherError::Custom(format!("无法读取 install_profile.json: {}", e))
                })?;
            break;
        }
    }

    if install_profile_content.is_empty() {
        return Err(LauncherError::Custom(
            "安装器中未找到 install_profile.json".to_string(),
        ));
    }

    println!("Forge: 找到 install_profile.json");

    // 3. 解析 install_profile.json
    let profile: serde_json::Value = serde_json::from_str(&install_profile_content)
        .map_err(|e| LauncherError::Custom(format!("解析 install_profile.json 失败: {}", e)))?;

    // 4. 从 profile 下载所有必需的库
    let libraries_dir = game_dir.join("libraries");
    let client = Client::new(); // Create a client for library downloads
    if let Some(version_info) = profile.get("versionInfo") {
        if let Some(libraries) = version_info.get("libraries").and_then(|l| l.as_array()) {
            println!("Forge: 开始从 install_profile 下载 {} 个库", libraries.len());
            for lib in libraries {
                if let Err(e) = download_library_from_profile(lib, &libraries_dir, &client).await {
                    // Log error but don't fail the installation, as the launcher might be able to download it later.
                    println!("Forge: 下载库失败: {}，但继续安装过程", e);
                }
            }
            println!("Forge: 库下载完成");
        }
    }

    // 5. 创建版本目录和 JSON 文件
    let version_id = format!("{}-forge{}", forge_version.mcversion, forge_version.version);
    let version_dir = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&version_dir)
        .map_err(|e| LauncherError::Custom(format!("创建版本目录失败: {}", e)))?;
    println!("Forge: 创建版本目录: {}", version_dir.display());

    let version_json_path = version_dir.join(format!("{}.json", version_id));
    
    // Use the versionInfo from the profile directly
    let version_info = profile.get("versionInfo").ok_or_else(|| LauncherError::Custom("install_profile.json is missing 'versionInfo'".to_string()))?;

    fs::write(
        &version_json_path,
        serde_json::to_string_pretty(&version_info).unwrap(),
    )
    .map_err(|e| LauncherError::Custom(format!("写入版本 JSON 失败: {}", e)))?;
    println!("Forge: 已创建版本 JSON: {}", version_json_path.display());

    // 6. 解压安装器中的 "maven" 目录和 universal jar
    println!("Forge: 开始从安装器中提取文件...");
    // Re-open archive for extraction
    let file = fs::File::open(installer_path).map_err(|e| LauncherError::Custom(format!("无法重新打开安装器文件: {}", e)))?;
    let mut archive = ZipArchive::new(file).map_err(|e| LauncherError::Custom(format!("无法重新读取安装器 ZIP: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| LauncherError::Custom(format!("无法读取 ZIP 条目: {}", e)))?;
        let file_name = file.name().to_string();

        // Extract files from the 'maven' directory within the installer
        if file_name.starts_with("maven/") && !file_name.ends_with('/') {
            let rel_path = file_name.strip_prefix("maven/").unwrap();
            let target_path = libraries_dir.join(rel_path);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| LauncherError::Custom(format!("创建库子目录失败: {}", e)))?;
            }

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).map_err(|e| LauncherError::Custom(format!("读取库文件失败: {}", e)))?;
            fs::write(&target_path, buffer).map_err(|e| LauncherError::Custom(format!("写入库文件失败: {}", e)))?;
        }
        // Extract the universal jar
        else if file_name.ends_with("-universal.jar") && !file_name.contains('/') {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).map_err(|e| LauncherError::Custom(format!("读取 Forge universal JAR 失败: {}", e)))?;

            // Place it in the correct maven path inside /libraries
            let forge_lib_path = format!("net/minecraftforge/forge/{mc}-{v}/forge-{mc}-{v}-universal.jar", mc=forge_version.mcversion, v=forge_version.version);
            let lib_target_path = libraries_dir.join(&forge_lib_path);

            if let Some(parent) = lib_target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| LauncherError::Custom(format!("创建 Forge 库目录失败: {}", e)))?;
            }
            fs::write(&lib_target_path, &buffer).map_err(|e| LauncherError::Custom(format!("写入 Forge 库 JAR 失败: {}", e)))?;
            println!("Forge: 已提取 Forge universal JAR: {}", lib_target_path.display());

            // Also copy it to the version directory for compatibility
            let version_jar_path = version_dir.join(format!("{}.jar", version_id));
            fs::write(&version_jar_path, &buffer).map_err(|e| LauncherError::Custom(format!("写入版本 JAR 失败: {}", e)))?;
            println!("Forge: 已复制到版本目录: {}", version_jar_path.display());
        }
    }
    
    println!("Forge: 手动安装完成");
    Ok(())
}
/// Fetches the list of available Forge versions for a given Minecraft version.
pub async fn get_forge_versions(
    minecraft_version: String,
) -> Result<Vec<ForgeVersion>, LauncherError> {
    let client = Client::new();
    let url = format!(
        "{}/forge/minecraft/{}",
        BMCL_API_BASE_URL, minecraft_version
    );

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

/// 比较两个Forge版本号，返回排序顺序
fn compare_forge_versions(a: &str, b: &str) -> std::cmp::Ordering {
    // 分割版本号为数字部分
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    
    // 比较每个部分
    for i in 0..std::cmp::max(a_parts.len(), b_parts.len()) {
        let a_part = a_parts.get(i).unwrap_or(&"0");
        let b_part = b_parts.get(i).unwrap_or(&"0");
        
        // 尝试解析为数字比较
        let a_num = a_part.parse::<u32>().unwrap_or(0);
        let b_num = b_part.parse::<u32>().unwrap_or(0);
        
        match a_num.cmp(&b_num) {
            std::cmp::Ordering::Equal => continue,
            ordering => return ordering,
        }
    }
    
    // 如果所有部分都相等，按字符串比较（处理特殊情况）
    a.cmp(b)
}

/// Installs a specific version of Forge into a given instance directory.
pub async fn install_forge(
    instance_path: PathBuf,
    forge_version: ForgeVersion,
) -> Result<(), LauncherError> {
    // 1. Get Java path and game directory from config
    let app_config = config::load_config()?;
    let java_path = app_config
        .java_path
        .ok_or_else(|| LauncherError::Custom("未设置Java路径，无法安装Forge.".to_string()))?;
    let game_dir = std::path::PathBuf::from(&app_config.game_dir);

    // 2. 准备安装器路径
    let installer_filename = format!(
        "forge-{}-{}-installer.jar",
        forge_version.mcversion, forge_version.version
    );
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(&installer_filename);

    // 3. 构建HTTP客户端
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ),
    );
    default_headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("*/*"),
    );
    default_headers.insert(
        reqwest::header::CACHE_CONTROL,
        reqwest::header::HeaderValue::from_static("no-cache"),
    );

    let client = Client::builder()
        .default_headers(default_headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| LauncherError::Custom(format!("创建HTTP客户端失败: {}", e)))?;

    // 4. 使用通用下载函数下载安装器
    println!("Forge: 开始下载安装器");
    let response = download_from_multiple_sources(&forge_version, &client).await?;

    // 5. 验证并保存安装器文件
    let installer_bytes = response.bytes().await?;

    // 验证JAR文件格式
    if installer_bytes.len() >= 4 {
        let header = &installer_bytes[0..4];
        if header != [0x50, 0x4B, 0x03, 0x04] {
            file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
            return Err(LauncherError::Custom(
                "下载Forge安装器失败: 文件不是有效的JAR/ZIP格式".to_string(),
            ));
        }
    } else {
        file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
        return Err(LauncherError::Custom(
            "下载Forge安装器失败: 文件大小过小，无法验证格式".to_string(),
        ));
    }

    fs::write(&installer_path, &installer_bytes)?;
    println!(
        "Forge: 安装器已保存到: {} ({} 字节)",
        installer_path.display(),
        installer_bytes.len()
    );

    // 3. Run the installer（设置工作目录为全局游戏目录，打印日志并捕获详细输出）
    println!("Forge: Java路径: {}", java_path);
    println!("Forge: 工作目录: {}", game_dir.display());
    println!("Forge: 安装器路径: {}", installer_path.display());
    println!("Forge: 实例目录: {}", instance_path.display());

    // 下载 LaunchWrapper 库（旧版本 Forge 需要）
    let libraries_dir = game_dir.join("libraries");
    if let Err(e) = download_launchwrapper_library(&libraries_dir, &forge_version.mcversion).await {
        println!("Forge: LaunchWrapper 库下载失败: {}，但继续安装过程", e);
        // 不中断安装，让启动器在运行时自动检测和下载
    } else {
        println!("Forge: LaunchWrapper 库下载成功");
    }

    // 下载 ASM 库（Forge 字节码操作需要）
    if let Err(e) = download_asm_library(&libraries_dir, &forge_version.mcversion).await {
        println!("Forge: ASM 库下载失败: {}，但继续安装过程", e);
        // 不中断安装，让启动器在运行时自动检测和下载
    } else {
        println!("Forge: ASM 库下载成功");
    }

    // 下载 LZMA 库（Forge 压缩文件处理需要）
    if let Err(e) = download_lzma_library(&libraries_dir, &forge_version.mcversion).await {
        println!("Forge: LZMA 库下载失败: {}，但继续安装过程", e);
        // 不中断安装，让启动器在运行时自动检测和下载
    } else {
        println!("Forge: LZMA 库下载成功");
    }

    // 在游戏目录创建占位的 launcher_profiles.json（若不存在），满足 Forge 安装器检查
    let launcher_profiles = game_dir.join("launcher_profiles.json");
    if !launcher_profiles.exists() {
        let placeholder = r#"{
  "profiles": {},
  "selectedProfile": "",
  "clientToken": "",
  "authenticationDatabase": {}
}"#;
        if let Some(parent) = launcher_profiles.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&launcher_profiles, placeholder) {
            Ok(_) => println!(
                "Forge: 已创建占位 launcher_profiles.json 于 {}",
                launcher_profiles.display()
            ),
            Err(e) => println!("Forge: 创建占位 launcher_profiles.json 失败: {}", e),
        }
    }

    // 尝试方式 A：新版本 Forge 安装器（使用 --installClient）
    println!("Forge: 尝试使用新版本安装方式（--installClient）");
    let mut cmd = Command::new(&java_path);
    cmd.current_dir(&game_dir)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient");
    
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    
    let output = cmd.output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Forge: 新版本安装器 stdout:\n{}", stdout);
    println!("Forge: 新版本安装器 stderr:\n{}", stderr);

    // 检查是否因为不支持 --installClient 参数而失败
    let is_old_installer = !output.status.success()
        && (stderr.contains("not a recognized option")
            || stderr.contains("UnrecognizedOptionException"));

    if is_old_installer {
        println!("Forge: 检测到旧版本安装器，切换到旧版本安装方式");

        // 尝试方式 B1：旧版本 Forge 安装器（使用 headless 模式 + --installClient 目录）
        println!("Forge: 尝试使用 headless 模式 + --installClient 目录参数");
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

        let mut stdout_old = String::from_utf8_lossy(&output_old.stdout);
        let mut stderr_old = String::from_utf8_lossy(&output_old.stderr);

        println!("Forge: 旧版本安装器 (尝试1) stdout:\n{}", stdout_old);
        println!("Forge: 旧版本安装器 (尝试1) stderr:\n{}", stderr_old);

        // 如果还是参数错误，尝试不带 --installClient 参数
        if !output_old.status.success()
            && (stderr_old.contains("not a recognized option")
                || stderr_old.contains("UnrecognizedOptionException"))
        {
            println!("Forge: 尝试使用 headless 模式（无参数）");
            let mut cmd_old2 = Command::new(&java_path);
            cmd_old2.current_dir(&game_dir)
                .arg("-Djava.awt.headless=true")
                .arg("-jar")
                .arg(&installer_path);
            
            #[cfg(windows)]
            cmd_old2.creation_flags(CREATE_NO_WINDOW);
            
            output_old = cmd_old2.output()?;

            stdout_old = String::from_utf8_lossy(&output_old.stdout);
            stderr_old = String::from_utf8_lossy(&output_old.stderr);

            println!("Forge: 旧版本安装器 (尝试2) stdout:\n{}", stdout_old);
            println!("Forge: 旧版本安装器 (尝试2) stderr:\n{}", stderr_old);
        }

        // 检查是否因为 HeadlessException 失败
        let is_headless_error =
            !output_old.status.success() && stderr_old.contains("HeadlessException");

        if is_headless_error {
            println!("Forge: 检测到 HeadlessException，使用手动安装方式");
            // 使用手动安装
            manual_install_old_forge(&installer_path, &game_dir, &forge_version).await?;
            println!("Forge: 手动安装成功");
        } else if !output_old.status.success() {
            // Clean up the installer file and installation artifacts before returning the error
            file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
            return Err(LauncherError::Custom(format!(
                "Forge安装失败（旧版本安装器）。\nstdout: {}\nstderr: {}",
                stdout_old, stderr_old
            )));
        } else {
            println!("Forge: 旧版本安装器执行完成");
        }
    } else if !output.status.success() {
        // 新版本安装器执行失败（非参数不支持的其他错误）
        file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
        return Err(LauncherError::Custom(format!(
            "Forge安装失败。\nstdout: {}\nstderr: {}",
            stdout, stderr
        )));
    } else {
        println!("Forge: 新版本安装器执行完成");
    }

    // 5. 安装器执行完成后，进行清理操作
    println!("Forge: 安装完成，开始清理临时文件");
    
    // 清理临时安装器文件
    if installer_path.exists() {
        match fs::remove_file(&installer_path) {
            Ok(_) => println!("Forge: 临时安装器文件清理完成"),
            Err(e) => println!("Forge: 清理安装器文件失败: {}，但安装继续", e),
        }
    } else {
        println!("Forge: 安装器文件已不存在，无需清理");
    }
    Ok(())
}
