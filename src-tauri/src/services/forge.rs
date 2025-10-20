use crate::errors::LauncherError;
use crate::models::ForgeVersion;
use crate::services::config;
use crate::utils::file_utils;

use reqwest::{Client, StatusCode};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use zip::ZipArchive;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

const BMCL_API_BASE_URL: &str = "https://bmclapi2.bangbang93.com";

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

    // 4. 创建版本目录
    let version_id = format!("{}-forge{}", forge_version.mcversion, forge_version.version);
    let version_dir = game_dir.join("versions").join(&version_id);
    fs::create_dir_all(&version_dir)
        .map_err(|e| LauncherError::Custom(format!("创建版本目录失败: {}", e)))?;
    println!("Forge: 创建版本目录: {}", version_dir.display());

    // 5. 创建版本 JSON 文件
    let version_json_path = version_dir.join(format!("{}.json", version_id));

    // 旧版本 Forge 使用 "versionInfo"，新版本可能直接在根级别
    let version_info = if let Some(info) = profile.get("versionInfo") {
        info.clone()
    } else {
        // 如果没有 versionInfo，尝试构建基本的版本信息
        println!("Forge: 未找到 versionInfo，尝试从 install_profile 构建");

        // 读取原版 Minecraft 的版本 JSON 作为基础
        let mc_version_path = game_dir
            .join("versions")
            .join(&forge_version.mcversion)
            .join(format!("{}.json", forge_version.mcversion));

        if mc_version_path.exists() {
            let mc_json_content = fs::read_to_string(&mc_version_path)
                .map_err(|e| LauncherError::Custom(format!("读取原版 MC JSON 失败: {}", e)))?;
            let mut mc_json: serde_json::Value = serde_json::from_str(&mc_json_content)
                .map_err(|e| LauncherError::Custom(format!("解析原版 MC JSON 失败: {}", e)))?;

            // 修改 ID
            if let Some(obj) = mc_json.as_object_mut() {
                obj.insert("id".to_string(), serde_json::json!(version_id));

                // 添加 Forge 库
                if let Some(install) = profile.get("install") {
                    if let Some(path) = install.get("path") {
                        let forge_lib = serde_json::json!({
                            "name": path.as_str().unwrap_or("")
                        });

                        if let Some(libs) = obj.get_mut("libraries").and_then(|l| l.as_array_mut())
                        {
                            libs.insert(0, forge_lib);
                        }
                    }
                }

                // 修改 mainClass（如果 profile 中有）
                if let Some(install) = profile.get("install") {
                    if let Some(main_class) = install.get("minecraft") {
                        obj.insert("mainClass".to_string(), main_class.clone());
                    }
                }
            }

            mc_json
        } else {
            return Err(LauncherError::Custom(format!(
                "无法找到原版 Minecraft {} 的版本文件，请先安装原版",
                forge_version.mcversion
            )));
        }
    };

    fs::write(
        &version_json_path,
        serde_json::to_string_pretty(&version_info).unwrap(),
    )
    .map_err(|e| LauncherError::Custom(format!("写入版本 JSON 失败: {}", e)))?;
    println!("Forge: 已创建版本 JSON: {}", version_json_path.display());

    // 6. 解压并复制库文件
    let libraries_dir = game_dir.join("libraries");
    fs::create_dir_all(&libraries_dir)
        .map_err(|e| LauncherError::Custom(format!("创建库目录失败: {}", e)))?;

    // 重新打开 archive 用于提取文件
    let file = fs::File::open(installer_path)
        .map_err(|e| LauncherError::Custom(format!("无法重新打开安装器文件: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| LauncherError::Custom(format!("无法重新读取安装器 ZIP: {}", e)))?;

    let mut forge_universal_found = false;

    // 提取所有必要的文件
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| LauncherError::Custom(format!("无法读取 ZIP 条目: {}", e)))?;
        let file_name = file.name().to_string();

        // 提取 maven/ 目录下的所有库文件
        if file_name.starts_with("maven/") && !file_name.ends_with('/') {
            let rel_path = file_name.strip_prefix("maven/").unwrap();
            let target_path = libraries_dir.join(rel_path);

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| LauncherError::Custom(format!("创建库子目录失败: {}", e)))?;
            }

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| LauncherError::Custom(format!("读取库文件失败: {}", e)))?;
            fs::write(&target_path, buffer)
                .map_err(|e| LauncherError::Custom(format!("写入库文件失败: {}", e)))?;

            println!("Forge: 已提取库文件: {}", rel_path);
        }
        // 提取 Forge universal JAR（旧版本 Forge 需要）
        else if file_name.contains("universal")
            && file_name.ends_with(".jar")
            && !file_name.contains("/")
        {
            forge_universal_found = true;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).map_err(|e| {
                LauncherError::Custom(format!("读取 Forge universal JAR 失败: {}", e))
            })?;

            // 1. 复制到 libraries 目录（标准 Maven 格式）
            let forge_lib_path = format!(
                "net/minecraftforge/forge/{}-{}",
                forge_version.mcversion, forge_version.version
            );
            let lib_target_path = libraries_dir.join(&forge_lib_path).join(format!(
                "forge-{}-{}.jar",
                forge_version.mcversion, forge_version.version
            ));

            if let Some(parent) = lib_target_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| LauncherError::Custom(format!("创建 Forge 库目录失败: {}", e)))?;
            }

            fs::write(&lib_target_path, &buffer)
                .map_err(|e| LauncherError::Custom(format!("写入 Forge 库 JAR 失败: {}", e)))?;
            println!("Forge: 已提取 Forge 库 JAR: {}", lib_target_path.display());

            // 2. 同时复制到版本目录（某些启动器需要）
            let version_jar_path = version_dir.join(format!("{}.jar", version_id));
            fs::write(&version_jar_path, &buffer)
                .map_err(|e| LauncherError::Custom(format!("写入版本 JAR 失败: {}", e)))?;
            println!("Forge: 已复制到版本目录: {}", version_jar_path.display());
        }
    }

    if !forge_universal_found {
        println!("Forge: 警告 - 未找到 universal JAR，这可能是新版本安装器");
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

    let versions: Vec<ForgeVersion> = response.json().await?;

    // The API returns versions in ascending order, so we reverse it to show newest first.
    Ok(versions.into_iter().rev().collect())
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

    // 2. Construct installer URL and download it
    // 使用不带时间戳的稳定下载 URL，避免镜像返回异常的 304
    let installer_url = format!(
        "https://bmclapi2.bangbang93.com/forge/download/{}",
        forge_version.build
    );
    let installer_filename = format!(
        "forge-{}-{}-installer.jar",
        forge_version.mcversion, forge_version.version
    );
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(&installer_filename);
    println!("Forge: 准备下载安装器: {}", installer_url);

    // 模拟真实浏览器请求，避免被服务器拒绝或返回502错误
    let mut default_headers = reqwest::header::HeaderMap::new();

    // 使用常见的Chrome浏览器User-Agent
    default_headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ),
    );

    // 添加完整的浏览器请求头
    default_headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"),
    );
    default_headers.insert(
        reqwest::header::ACCEPT_ENCODING,
        reqwest::header::HeaderValue::from_static("gzip, deflate, br"),
    );
    default_headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
    );
    default_headers.insert(
        "sec-ch-ua",
        reqwest::header::HeaderValue::from_static(
            "\"Chromium\";v=\"120\", \"Not?A_Brand\";v=\"99\"",
        ),
    );
    default_headers.insert(
        "sec-ch-ua-mobile",
        reqwest::header::HeaderValue::from_static("?0"),
    );
    default_headers.insert(
        "sec-ch-ua-platform",
        reqwest::header::HeaderValue::from_static("\"Windows\""),
    );
    default_headers.insert(
        reqwest::header::UPGRADE_INSECURE_REQUESTS,
        reqwest::header::HeaderValue::from_static("1"),
    );

    // 添加缓存控制头
    default_headers.insert(
        reqwest::header::CACHE_CONTROL,
        reqwest::header::HeaderValue::from_static("no-cache"),
    );
    default_headers.insert(
        reqwest::header::PRAGMA,
        reqwest::header::HeaderValue::from_static("no-cache"),
    );

    // 构建更健壮的HTTP客户端，处理TLS连接问题
    let client = Client::builder()
        .default_headers(default_headers)
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
        .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
        .https_only(false) // 允许HTTP回退
        .http1_title_case_headers() // 使用标题大小写头部
        .http1_allow_obsolete_multiline_headers_in_responses(true)
        .build()
        .map_err(|e| LauncherError::Custom(format!("创建HTTP客户端失败: {}", e)))?;

    println!("Forge: 开始下载安装器: {}", installer_url);

    // 初始请求也添加重试机制，处理网络连接问题
    let mut response;
    let mut initial_retry_count = 0;
    let max_initial_retries = 3;
    let mut tried_urls = vec![installer_url.clone()];
    let mut current_url = installer_url.clone();

    loop {
        initial_retry_count += 1;
        let result = client.get(&current_url).send().await;

        match result {
            Ok(resp) => {
                response = resp;
                println!("Forge: 安装器下载响应状态: {}", response.status());
                break;
            }
            Err(e) => {
                let error_msg = e.to_string();
                println!(
                    "Forge: 初始请求第{}次失败: {}",
                    initial_retry_count, error_msg
                );

                // 检查是否是网络连接问题
                if error_msg.contains("error sending request")
                    || error_msg.contains("connection")
                    || error_msg.contains("dns")
                {
                    println!("Forge: 检测到网络连接问题，等待后重试");

                    if initial_retry_count >= max_initial_retries {
                        println!("Forge: bmclapi连接失败，回退到MinecraftForge官网下载");
                        
                        // 回退到Forge官方Maven下载
                        let maven_path = format!(
                            "{mc}-{ver}",
                            mc = forge_version.mcversion,
                            ver = forge_version.version
                        );
                        let forge_official_url = format!(
                            "https://maven.minecraftforge.net/net/minecraftforge/forge/{path}/forge-{path}-installer.jar",
                            path = maven_path
                        );
                        
                        println!("Forge: 尝试官方源: {}", forge_official_url);
                        tried_urls.push(forge_official_url.clone());
                        
                        // 重置重试计数并切换到官方源
                        initial_retry_count = 0;
                        current_url = forge_official_url;
                        continue;
                    }

                    // 等待2秒后重试
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                } else {
                    // 其他错误直接返回
                    return Err(LauncherError::Custom(format!(
                        "下载Forge安装器失败: {}",
                        error_msg
                    )));
                }
            }
        }
    }

    // 记录详细的响应头信息用于调试
    println!("Forge: 响应头信息:");
    for (name, value) in response.headers() {
        let name_str = name.as_str();
        if let Ok(value_str) = value.to_str() {
            println!("  {}: {}", name_str, value_str);
        }
    }

    // 处理重定向和失败重试逻辑
    if response.status() == StatusCode::NOT_MODIFIED || !response.status().is_success() {
        // tried_urls已经在前面初始化，包含初始URL和可能的回退URL
        let mut current_response = response;

        // 1) 首先检查是否是重定向响应，并对重定向后的URL进行多次重试
        let mut redirect_processed = false;
        if current_response.status().is_redirection() {
            // 提前提取重定向URL，避免借用问题
            let redirect_url =
                if let Some(location) = current_response.headers().get(reqwest::header::LOCATION) {
                    if let Ok(url) = location.to_str() {
                        Some(url.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

            if let Some(redirect_url) = redirect_url {
                println!("Forge: 检测到重定向到: {}", redirect_url);
                tried_urls.push(redirect_url.clone());

                // 对重定向后的URL进行最多5次重试，添加指数退避
                for retry_count in 1..=5 {
                    println!("Forge: 重定向URL重试第{}次", retry_count);

                    // 添加重试延迟（指数退避）
                    if retry_count > 1 {
                        let delay_seconds = std::cmp::min(2u64.pow(retry_count as u32 - 1), 10);
                        println!("Forge: 等待 {} 秒后重试", delay_seconds);
                        tokio::time::sleep(std::time::Duration::from_secs(delay_seconds)).await;
                    }

                    let result = client
                        .get(&redirect_url)
                        .header(reqwest::header::CACHE_CONTROL, "no-cache, no-store")
                        .header(reqwest::header::PRAGMA, "no-cache")
                        .send()
                        .await;

                    match result {
                        Ok(redirect_response) => {
                            if redirect_response.status().is_success() {
                                current_response = redirect_response;
                                redirect_processed = true;
                                println!("Forge: 重定向下载成功（第{}次重试）", retry_count);
                                break;
                            } else {
                                let status = redirect_response.status();
                                println!(
                                    "Forge: 重定向URL第{}次重试失败，状态: {}",
                                    retry_count, status
                                );
                                current_response = redirect_response;

                                // 如果是502错误，特别处理
                                if status == StatusCode::BAD_GATEWAY {
                                    println!(
                                        "Forge: 检测到502 Bad Gateway错误，可能需要等待服务器恢复"
                                    );
                                }

                                // 如果是最后一次重试仍然失败，继续尝试其他源
                                if retry_count == 5 {
                                    println!("Forge: 重定向URL重试5次均失败，尝试其他下载源");
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            println!(
                                "Forge: 重定向URL第{}次重试网络错误: {}",
                                retry_count, error_msg
                            );

                            // 检查是否是连接被重置的错误
                            if error_msg.contains("forcibly closed by the remote host")
                                || error_msg.contains("connection reset")
                                || error_msg.contains("tls strategy failed")
                            {
                                println!("Forge: 检测到连接被重置或TLS失败，可能是服务器限制，等待后重试");
                            }

                            // 如果是最后一次重试仍然失败，继续尝试其他源
                            if retry_count == 5 {
                                println!("Forge: 重定向URL重试5次均失败，尝试其他下载源");
                            }
                        }
                    }
                }
            }
        }

        // 2) 如果重定向失败或不是重定向，尝试多源重试
        if !redirect_processed && !current_response.status().is_success() {
            let sources = vec![
                // 源1: 标准bmcl下载地址（带重试）
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
                // 源4: 备用bmcl下载地址（不带时间戳）
                format!(
                    "https://bmclapi2.bangbang93.com/forge/download/{}?format=jar",
                    forge_version.build
                ),
            ];

            for (index, source_url) in sources.iter().enumerate() {
                if tried_urls.contains(source_url) {
                    continue; // 跳过已经尝试过的URL
                }

                println!("Forge: 尝试源 {}: {}", index + 1, source_url);
                tried_urls.push(source_url.clone());

                // 对每个源进行最多3次重试，添加指数退避
                let mut retry_success = false;
                for retry_count in 1..=3 {
                    println!("Forge: 源 {} 第{}次重试", index + 1, retry_count);

                    // 添加重试延迟（指数退避）
                    if retry_count > 1 {
                        let delay_seconds = std::cmp::min(2u64.pow(retry_count as u32 - 1), 8);
                        println!("Forge: 等待 {} 秒后重试", delay_seconds);
                        tokio::time::sleep(std::time::Duration::from_secs(delay_seconds)).await;
                    }

                    let result = client
                        .get(source_url)
                        .header(reqwest::header::CACHE_CONTROL, "no-cache, no-store")
                        .header(reqwest::header::PRAGMA, "no-cache")
                        .send()
                        .await;

                    match result {
                        Ok(retry_response) => {
                            // 验证响应内容是否为有效的JAR文件
                            if retry_response.status().is_success() {
                                // 检查Content-Type确保不是HTML页面
                                if let Some(content_type) =
                                    retry_response.headers().get(reqwest::header::CONTENT_TYPE)
                                {
                                    let content_type_str =
                                        content_type.to_str().unwrap_or("").to_lowercase();
                                    if content_type_str.contains("text/html")
                                        || content_type_str.contains("application/json")
                                    {
                                        println!(
                                            "Forge: 源 {} 返回了HTML/JSON内容，跳过",
                                            index + 1
                                        );
                                        continue; // 继续下一次重试
                                    }
                                }

                                // 检查Content-Length确保文件大小合理
                                if let Some(content_length) = retry_response
                                    .headers()
                                    .get(reqwest::header::CONTENT_LENGTH)
                                {
                                    if let Ok(length_str) = content_length.to_str() {
                                        if let Ok(file_size) = length_str.parse::<u64>() {
                                            if file_size < 1024 {
                                                println!(
                                                    "Forge: 源 {} 文件大小异常（{}字节），跳过",
                                                    index + 1,
                                                    file_size
                                                );
                                                continue; // 继续下一次重试
                                            }
                                        }
                                    }
                                }

                                current_response = retry_response;
                                retry_success = true;
                                println!(
                                    "Forge: 源 {} 下载成功（第{}次重试）",
                                    index + 1,
                                    retry_count
                                );
                                break;
                            } else {
                                let status = retry_response.status();
                                println!(
                                    "Forge: 源 {} 第{}次重试失败，状态: {}",
                                    index + 1,
                                    retry_count,
                                    status
                                );
                                current_response = retry_response;

                                // 如果是502错误，特别处理
                                if status == StatusCode::BAD_GATEWAY {
                                    println!(
                                        "Forge: 检测到502 Bad Gateway错误，可能需要等待服务器恢复"
                                    );
                                }

                                // 如果是最后一次重试仍然失败，继续尝试下一个源
                                if retry_count == 3 {
                                    println!("Forge: 源 {} 重试3次均失败，尝试下一个源", index + 1);
                                }
                            }
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            println!(
                                "Forge: 源 {} 第{}次重试网络错误: {}",
                                index + 1,
                                retry_count,
                                error_msg
                            );

                            // 检查是否是连接被重置的错误
                            if error_msg.contains("forcibly closed by the remote host")
                                || error_msg.contains("connection reset")
                                || error_msg.contains("tls strategy failed")
                            {
                                println!("Forge: 检测到连接被重置或TLS失败，可能是服务器限制");
                            }

                            // 如果是最后一次重试仍然失败，继续尝试下一个源
                            if retry_count == 3 {
                                println!("Forge: 源 {} 重试3次均失败，尝试下一个源", index + 1);
                            }
                        }
                    }
                }

                if retry_success {
                    break;
                }
            }
        }

        // 3) 如果所有源都失败，返回详细错误信息
        if !current_response.status().is_success() {
            file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
            return Err(LauncherError::Custom(format!(
                "下载Forge安装器失败: 最终状态 {}。已尝试的URL: {}",
                current_response.status(),
                tried_urls.join(", ")
            )));
        }

        // 更新最终的response变量
        response = current_response;
    }

    // 验证下载的文件内容
    if !response.status().is_success() {
        file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
        return Err(LauncherError::Custom(format!(
            "下载Forge安装器失败: 服务器返回错误状态 {}。已尝试 bmcl 下载与 Maven 源。",
            response.status()
        )));
    }

    // 检查响应头确保是有效的JAR文件
    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
        let content_type_str = content_type.to_str().unwrap_or("").to_lowercase();
        if content_type_str.contains("text/html") || content_type_str.contains("application/json") {
            file_utils::cleanup_forge_installation(&instance_path, &game_dir, &forge_version, &installer_path);
            return Err(LauncherError::Custom(format!(
                "下载Forge安装器失败: 期望获取JAR文件，但服务器返回了{}内容。URL: {}",
                content_type_str, current_url
            )));
        }
    }

    // 检查文件大小
    if let Some(content_length) = response.headers().get(reqwest::header::CONTENT_LENGTH) {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(file_size) = length_str.parse::<u64>() {
                if file_size < 1024 {
                    file_utils::cleanup_forge_installation(
                        &instance_path,
                        &game_dir,
                        &forge_version,
                        &installer_path,
                    );
                    return Err(LauncherError::Custom(format!(
                        "下载Forge安装器失败: 文件大小异常（{}字节），可能下载了错误文件",
                        file_size
                    )));
                }
                println!("Forge: 安装器文件大小: {} 字节", file_size);
            }
        }
    }

    let installer_bytes = response.bytes().await?;

    // 验证下载的字节是否为有效的JAR文件（检查ZIP文件头）
    if installer_bytes.len() >= 4 {
        let header = &installer_bytes[0..4];
        if header != [0x50, 0x4B, 0x03, 0x04] {
            // ZIP文件头
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
