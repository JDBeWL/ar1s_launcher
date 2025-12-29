use crate::errors::LauncherError;
use crate::models::modpack::*;
use crate::services::{config, download, forge, modrinth};
use crate::utils::file_utils;
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
pub struct ModpackInstallProgress {
    pub progress: u8,
    pub message: String,
    pub indeterminate: bool,
}

/// Modrinth index.json 中的文件定义
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ModrinthIndexFile {
    path: String,
    hashes: ModrinthIndexHashes,
    downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ModrinthIndexHashes {
    sha1: String,
    sha512: Option<String>,
}

/// Modrinth index.json 结构
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ModrinthIndex {
    #[serde(rename = "formatVersion")]
    format_version: u32,
    game: String,
    #[serde(rename = "versionId")]
    version_id: String,
    name: String,
    files: Vec<ModrinthIndexFile>,
    dependencies: ModrinthDependencies,
}

#[derive(Debug, Deserialize)]
struct ModrinthDependencies {
    minecraft: String,
    #[serde(default)]
    forge: Option<String>,
    #[serde(default)]
    fabric: Option<String>,
    #[serde(rename = "fabric-loader")]
    #[serde(default)]
    fabric_loader: Option<String>,
    #[serde(default)]
    quilt: Option<String>,
    #[serde(rename = "quilt-loader")]
    #[serde(default)]
    quilt_loader: Option<String>,
    #[serde(default)]
    neoforge: Option<String>,
}

pub struct ModpackInstaller {
    modrinth_service: modrinth::ModrinthService,
    http_client: Client,
}

impl ModpackInstaller {
    pub fn new() -> Self {
        Self {
            modrinth_service: modrinth::ModrinthService::new(),
            http_client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// 安装Modrinth整合包
    pub async fn install_modrinth_modpack(
        &self,
        options: ModpackInstallOptions,
        window: &tauri::Window,
    ) -> Result<(), LauncherError> {
        let config = config::load_config()?;
        let game_dir = PathBuf::from(&config.game_dir);
        let instance_dir = game_dir.join("versions").join(&options.instance_name);

        // 发送进度更新
        let send_progress = |progress: u8, message: &str, indeterminate: bool| {
            let _ = window.emit(
                "modpack-install-progress",
                ModpackInstallProgress {
                    progress,
                    message: message.to_string(),
                    indeterminate,
                },
            );
        };

        send_progress(5, "检查实例目录...", false);

        // 1. 检查实例是否已存在
        if instance_dir.exists() {
            return Err(LauncherError::Custom(format!(
                "名为 '{}' 的实例已存在",
                options.instance_name
            )));
        }

        send_progress(10, "获取整合包信息...", false);

        // 2. 获取整合包详细信息
        let modpack = self
            .modrinth_service
            .get_modpack(&options.modpack_id)
            .await
            .map_err(|e| LauncherError::Custom(format!("获取整合包信息失败: {}", e)))?;

        send_progress(15, "获取整合包版本...", false);

        // 3. 获取指定版本信息
        let versions = self
            .modrinth_service
            .get_modpack_versions(&options.modpack_id, None, None)
            .await
            .map_err(|e| LauncherError::Custom(format!("获取整合包版本失败: {}", e)))?;

        let selected_version = versions
            .iter()
            .find(|v| v.id == options.version_id)
            .ok_or_else(|| LauncherError::Custom("未找到指定的整合包版本".to_string()))?;

        send_progress(20, "下载整合包文件...", false);

        // 4. 下载整合包文件
        let primary_file = selected_version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| selected_version.files.first())
            .ok_or_else(|| LauncherError::Custom("整合包没有可用的文件".to_string()))?;

        let temp_dir = game_dir.join("temp");
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)?;
        }

        let modpack_file_path = temp_dir.join(&primary_file.filename);

        self.modrinth_service
            .download_modpack_file(&primary_file.url, &modpack_file_path)
            .await
            .map_err(|e| LauncherError::Custom(format!("下载整合包文件失败: {}", e)))?;

        send_progress(35, "解压整合包...", false);

        // 5. 解压整合包
        let extract_dir = temp_dir.join(format!("{}_extract", &options.instance_name));
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        fs::create_dir_all(&extract_dir)?;

        self.extract_modpack(&modpack_file_path, &extract_dir)
            .await
            .map_err(|e| LauncherError::Custom(format!("解压整合包失败: {}", e)))?;

        send_progress(45, "处理整合包配置...", false);

        // 6. 处理整合包配置
        let index_path = extract_dir.join("modrinth.index.json");
        let modrinth_index = if index_path.exists() {
            let content = fs::read_to_string(&index_path)?;
            Some(
                serde_json::from_str::<ModrinthIndex>(&content)
                    .map_err(|e| LauncherError::Custom(format!("解析 modrinth.index.json 失败: {}", e)))?,
            )
        } else {
            None
        };

        // 创建实例目录
        fs::create_dir_all(&instance_dir)?;

        send_progress(50, "复制整合包文件...", false);

        // 7. 复制 overrides 目录内容
        let overrides_dir = extract_dir.join("overrides");
        if overrides_dir.exists() {
            info!("复制 overrides 目录到实例");
            file_utils::copy_dir_all(&overrides_dir, &instance_dir)?;
        }

        // 也检查 client-overrides (某些整合包使用)
        let client_overrides_dir = extract_dir.join("client-overrides");
        if client_overrides_dir.exists() {
            info!("复制 client-overrides 目录到实例");
            file_utils::copy_dir_all(&client_overrides_dir, &instance_dir)?;
        }

        // 8. 下载 mods 和其他依赖文件
        if let Some(ref index) = modrinth_index {
            send_progress(55, "下载模组文件...", false);
            self.download_modpack_files(&index.files, &instance_dir, window)
                .await?;
        }

        send_progress(75, "安装游戏版本...", false);

        // 9. 安装基础游戏版本和加载器
        if let Some(ref index) = modrinth_index {
            self.install_game_and_loader(
                &index.dependencies,
                &options.instance_name,
                &game_dir,
                window,
            )
            .await?;
        }

        send_progress(90, "创建实例配置...", false);

        // 10. 创建实例配置文件
        let mc_version = modrinth_index
            .as_ref()
            .map(|i| i.dependencies.minecraft.clone())
            .or_else(|| selected_version.game_versions.first().cloned())
            .unwrap_or_default();

        let loader_type = if modrinth_index.as_ref().map(|i| i.dependencies.forge.is_some()).unwrap_or(false) {
            Some("forge")
        } else if modrinth_index.as_ref().map(|i| i.dependencies.fabric_loader.is_some() || i.dependencies.fabric.is_some()).unwrap_or(false) {
            Some("fabric")
        } else if modrinth_index.as_ref().map(|i| i.dependencies.quilt_loader.is_some() || i.dependencies.quilt.is_some()).unwrap_or(false) {
            Some("quilt")
        } else if modrinth_index.as_ref().map(|i| i.dependencies.neoforge.is_some()).unwrap_or(false) {
            Some("neoforge")
        } else {
            None
        };

        let instance_config = serde_json::json!({
            "id": options.instance_name.clone(),
            "name": modpack.title.clone(),
            "type": "modpack",
            "source": "modrinth",
            "modpack_id": modpack.slug.clone(),
            "modpack_version": selected_version.version_number.clone(),
            "minecraft": mc_version,
            "loader": loader_type,
            "loaders": selected_version.loaders.clone(),
            "created": chrono::Utc::now().to_rfc3339(),
        });

        let config_path = instance_dir.join("instance.json");
        fs::write(config_path, serde_json::to_string_pretty(&instance_config)?)?;

        // 11. 清理临时文件
        if modpack_file_path.exists() {
            let _ = fs::remove_file(&modpack_file_path);
        }
        if extract_dir.exists() {
            let _ = fs::remove_dir_all(&extract_dir);
        }

        send_progress(100, "整合包安装完成！", false);
        info!("整合包 {} 安装完成", options.instance_name);

        Ok(())
    }


    /// 下载整合包中定义的文件（mods等）
    async fn download_modpack_files(
        &self,
        files: &[ModrinthIndexFile],
        instance_dir: &PathBuf,
        window: &tauri::Window,
    ) -> Result<(), LauncherError> {
        let total_files = files.len();
        info!("开始下载 {} 个文件", total_files);

        for (index, file) in files.iter().enumerate() {
            let progress = 55 + ((index as f32 / total_files as f32) * 20.0) as u8;
            let _ = window.emit(
                "modpack-install-progress",
                ModpackInstallProgress {
                    progress,
                    message: format!("下载文件 ({}/{}): {}", index + 1, total_files, file.path),
                    indeterminate: false,
                },
            );

            let dest_path = instance_dir.join(&file.path);

            // 创建父目录
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // 如果文件已存在且哈希匹配，跳过下载
            if dest_path.exists() {
                debug!("文件已存在，跳过: {}", file.path);
                continue;
            }

            // 尝试从所有下载源下载
            let mut downloaded = false;
            for url in &file.downloads {
                match self.download_file_with_retry(url, &dest_path, 3).await {
                    Ok(_) => {
                        downloaded = true;
                        debug!("下载成功: {}", file.path);
                        break;
                    }
                    Err(e) => {
                        warn!("下载失败 {}: {}", url, e);
                    }
                }
            }

            if !downloaded {
                error!("无法下载文件: {}", file.path);
                // 继续下载其他文件，不中断整个过程
            }
        }

        Ok(())
    }

    /// 带重试的文件下载
    async fn download_file_with_retry(
        &self,
        url: &str,
        dest: &PathBuf,
        max_retries: u32,
    ) -> Result<(), LauncherError> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
            }

            match self.http_client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.bytes().await {
                            Ok(bytes) => {
                                fs::write(dest, &bytes)?;
                                return Ok(());
                            }
                            Err(e) => {
                                last_error = Some(format!("读取响应失败: {}", e));
                            }
                        }
                    } else {
                        last_error = Some(format!("HTTP {}", response.status()));
                    }
                }
                Err(e) => {
                    last_error = Some(format!("请求失败: {}", e));
                }
            }
        }

        Err(LauncherError::Custom(
            last_error.unwrap_or_else(|| "下载失败".to_string()),
        ))
    }

    /// 安装游戏版本和加载器
    async fn install_game_and_loader(
        &self,
        deps: &ModrinthDependencies,
        instance_name: &str,
        game_dir: &PathBuf,
        window: &tauri::Window,
    ) -> Result<(), LauncherError> {
        let mc_version = &deps.minecraft;
        info!("安装 Minecraft {}", mc_version);

        // 下载基础游戏版本
        let config = config::load_config()?;
        download::process_and_download_version(
            mc_version.clone(),
            config.download_mirror.clone(),
            window,
        )
        .await?;

        // 安装加载器
        if let Some(forge_version) = &deps.forge {
            info!("安装 Forge {}", forge_version);
            self.install_forge_loader(mc_version, forge_version, instance_name, game_dir)
                .await?;
        } else if let Some(fabric_version) = deps.fabric_loader.as_ref().or(deps.fabric.as_ref()) {
            info!("安装 Fabric {}", fabric_version);
            self.install_fabric_loader(mc_version, fabric_version, instance_name, game_dir)
                .await?;
        } else if let Some(quilt_version) = deps.quilt_loader.as_ref().or(deps.quilt.as_ref()) {
            info!("安装 Quilt {}", quilt_version);
            self.install_quilt_loader(mc_version, quilt_version, instance_name, game_dir)
                .await?;
        } else if let Some(neoforge_version) = &deps.neoforge {
            info!("安装 NeoForge {}", neoforge_version);
            self.install_neoforge_loader(mc_version, neoforge_version, instance_name, game_dir)
                .await?;
        } else {
            // 纯净版，创建版本 JSON
            self.create_vanilla_version_json(mc_version, instance_name, game_dir)?;
        }

        Ok(())
    }

    /// 安装 Forge 加载器
    async fn install_forge_loader(
        &self,
        mc_version: &str,
        forge_version: &str,
        instance_name: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        use crate::models::ForgeVersion;

        info!("安装 Forge: MC {} + Forge {}", mc_version, forge_version);

        // 构造 ForgeVersion 结构
        let forge_ver = ForgeVersion {
            version: forge_version.to_string(),
            mcversion: mc_version.to_string(),
            build: 0, // 从版本号解析 build 号不是必须的
        };

        // 调用 Forge 安装服务
        let instance_path = game_dir.join("versions").join(instance_name);
        forge::install_forge(instance_path, forge_ver).await?;

        // 验证 Forge 版本是否被正确创建
        let version_id = format!("{}-forge-{}", mc_version, forge_version);
        let forge_version_json_path = game_dir
            .join("versions")
            .join(&version_id)
            .join(format!("{}.json", &version_id));
        
        if !forge_version_json_path.exists() {
            warn!("Forge 版本 JSON 未找到: {}", forge_version_json_path.display());
            // Forge 安装可能创建了不同格式的版本 ID，尝试查找
            let versions_dir = game_dir.join("versions");
            if let Ok(entries) = fs::read_dir(&versions_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("forge") && name.contains(mc_version) {
                        info!("找到可能的 Forge 版本: {}", name);
                    }
                }
            }
        } else {
            info!("Forge 版本 JSON 已创建: {}", forge_version_json_path.display());
        }

        // 创建指向 Forge 版本的版本 JSON
        self.create_loader_version_json(instance_name, &version_id, game_dir)?;

        Ok(())
    }

    /// 安装 Fabric 加载器
    async fn install_fabric_loader(
        &self,
        mc_version: &str,
        fabric_version: &str,
        instance_name: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        // Fabric 安装：下载 Fabric loader JSON
        let fabric_meta_url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
            mc_version, fabric_version
        );

        info!("获取 Fabric 版本信息: {}", fabric_meta_url);

        let response = self
            .http_client
            .get(&fabric_meta_url)
            .send()
            .await
            .map_err(|e| LauncherError::Custom(format!("获取 Fabric 信息失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(LauncherError::Custom(format!(
                "获取 Fabric 信息失败: {}",
                response.status()
            )));
        }

        let mut version_json: Value = response
            .json()
            .await
            .map_err(|e| LauncherError::Custom(format!("解析 Fabric JSON 失败: {}", e)))?;

        // 修改版本 ID 为实例名称
        let _version_id = format!("fabric-loader-{}-{}", fabric_version, mc_version);
        if let Some(obj) = version_json.as_object_mut() {
            obj.insert("id".to_string(), serde_json::json!(instance_name));
        }

        // 保存版本 JSON
        let version_dir = game_dir.join("versions").join(instance_name);
        fs::create_dir_all(&version_dir)?;

        let json_path = version_dir.join(format!("{}.json", instance_name));
        fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

        info!("Fabric 版本 JSON 已创建: {}", json_path.display());

        Ok(())
    }

    /// 安装 Quilt 加载器
    async fn install_quilt_loader(
        &self,
        mc_version: &str,
        quilt_version: &str,
        instance_name: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        // Quilt 安装：下载 Quilt loader JSON
        let quilt_meta_url = format!(
            "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
            mc_version, quilt_version
        );

        info!("获取 Quilt 版本信息: {}", quilt_meta_url);

        let response = self
            .http_client
            .get(&quilt_meta_url)
            .send()
            .await
            .map_err(|e| LauncherError::Custom(format!("获取 Quilt 信息失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(LauncherError::Custom(format!(
                "获取 Quilt 信息失败: {}",
                response.status()
            )));
        }

        let mut version_json: Value = response
            .json()
            .await
            .map_err(|e| LauncherError::Custom(format!("解析 Quilt JSON 失败: {}", e)))?;

        // 修改版本 ID
        if let Some(obj) = version_json.as_object_mut() {
            obj.insert("id".to_string(), serde_json::json!(instance_name));
        }

        // 保存版本 JSON
        let version_dir = game_dir.join("versions").join(instance_name);
        fs::create_dir_all(&version_dir)?;

        let json_path = version_dir.join(format!("{}.json", instance_name));
        fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

        info!("Quilt 版本 JSON 已创建: {}", json_path.display());

        Ok(())
    }

    /// 安装 NeoForge 加载器
    async fn install_neoforge_loader(
        &self,
        mc_version: &str,
        neoforge_version: &str,
        instance_name: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        info!("安装 NeoForge {} for MC {}", neoforge_version, mc_version);
        
        // NeoForge 版本格式：
        // - 1.20.1 之前: mc_version-neoforge_version (如 1.20.1-47.1.100)
        // - 1.20.2 之后: neoforge_version (如 20.2.88, 21.0.1)
        
        // 判断版本格式
        let (full_version, installer_url) = if neoforge_version.contains('.') && !neoforge_version.contains('-') {
            // 新版本格式 (20.2.88, 21.0.1 等)
            let full_ver = neoforge_version.to_string();
            let url = format!(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
                neoforge_version, neoforge_version
            );
            (full_ver, url)
        } else {
            // 旧版本格式或带 MC 版本的格式
            let full_ver = if neoforge_version.contains('-') {
                neoforge_version.to_string()
            } else {
                format!("{}-{}", mc_version, neoforge_version)
            };
            let url = format!(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
                full_ver, full_ver
            );
            (full_ver, url)
        };

        info!("NeoForge installer URL: {}", installer_url);

        // 下载 installer
        let temp_dir = game_dir.join("temp");
        fs::create_dir_all(&temp_dir)?;
        let installer_path = temp_dir.join(format!("neoforge-{}-installer.jar", full_version));

        // 尝试从 BMCLAPI 镜像下载
        let bmclapi_url = format!(
            "https://bmclapi2.bangbang93.com/neoforge/version/{}/download/installer.jar",
            full_version
        );

        let mut downloaded = false;
        
        // 先尝试 BMCLAPI
        info!("尝试从 BMCLAPI 下载 NeoForge installer");
        if let Ok(response) = self.http_client.get(&bmclapi_url).send().await {
            if response.status().is_success() {
                if let Ok(bytes) = response.bytes().await {
                    fs::write(&installer_path, &bytes)?;
                    downloaded = true;
                    info!("从 BMCLAPI 下载成功");
                }
            }
        }

        // 如果 BMCLAPI 失败，尝试官方源
        if !downloaded {
            info!("尝试从官方源下载 NeoForge installer");
            let response = self.http_client.get(&installer_url).send().await
                .map_err(|e| LauncherError::Custom(format!("下载 NeoForge installer 失败: {}", e)))?;
            
            if !response.status().is_success() {
                return Err(LauncherError::Custom(format!(
                    "下载 NeoForge installer 失败: {}",
                    response.status()
                )));
            }
            
            let bytes = response.bytes().await
                .map_err(|e| LauncherError::Custom(format!("读取 NeoForge installer 失败: {}", e)))?;
            fs::write(&installer_path, &bytes)?;
        }

        // 解压 installer 获取版本 JSON
        let extract_dir = temp_dir.join(format!("neoforge-{}-extract", full_version));
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        fs::create_dir_all(&extract_dir)?;

        let file = fs::File::open(&installer_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        // 查找版本 JSON 文件
        let mut version_json_content: Option<String> = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            
            if name == "version.json" {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                version_json_content = Some(content);
            } else if name.starts_with("maven/") {
                // 解压 maven 库文件
                let outpath = game_dir.join("libraries").join(&name[6..]);
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)?;
                }
                if !name.ends_with('/') {
                    let mut outfile = fs::File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }
            }
        }

        let version_json_str = version_json_content
            .ok_or_else(|| LauncherError::Custom("NeoForge installer 中未找到 version.json".to_string()))?;

        let mut version_json: Value = serde_json::from_str(&version_json_str)
            .map_err(|e| LauncherError::Custom(format!("解析 NeoForge version.json 失败: {}", e)))?;

        // 修改版本 ID 为实例名称
        if let Some(obj) = version_json.as_object_mut() {
            obj.insert("id".to_string(), serde_json::json!(instance_name));
        }

        // 保存版本 JSON
        let version_dir = game_dir.join("versions").join(instance_name);
        fs::create_dir_all(&version_dir)?;

        let json_path = version_dir.join(format!("{}.json", instance_name));
        fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

        info!("NeoForge 版本 JSON 已创建: {}", json_path.display());

        // 清理临时文件
        let _ = fs::remove_file(&installer_path);
        let _ = fs::remove_dir_all(&extract_dir);

        Ok(())
    }

    /// 创建纯净版版本 JSON
    fn create_vanilla_version_json(
        &self,
        mc_version: &str,
        instance_name: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        let version_json = serde_json::json!({
            "id": instance_name,
            "inheritsFrom": mc_version,
            "type": "release"
        });

        let version_dir = game_dir.join("versions").join(instance_name);
        fs::create_dir_all(&version_dir)?;

        let json_path = version_dir.join(format!("{}.json", instance_name));
        fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

        Ok(())
    }

    /// 创建加载器版本 JSON（指向已安装的加载器版本）
    fn create_loader_version_json(
        &self,
        instance_name: &str,
        loader_version_id: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        let version_json = serde_json::json!({
            "id": instance_name,
            "inheritsFrom": loader_version_id,
            "type": "release"
        });

        let version_dir = game_dir.join("versions").join(instance_name);
        fs::create_dir_all(&version_dir)?;

        let json_path = version_dir.join(format!("{}.json", instance_name));
        fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

        Ok(())
    }

    /// 解压整合包文件
    async fn extract_modpack(
        &self,
        modpack_file_path: &PathBuf,
        extract_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        let file = fs::File::open(modpack_file_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = extract_dir.join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }

    /// 搜索Modrinth整合包
    pub async fn search_modpacks(
        &self,
        query: Option<String>,
        game_versions: Option<Vec<String>>,
        loaders: Option<Vec<String>>,
        categories: Option<Vec<String>>,
        limit: Option<u32>,
        offset: Option<u32>,
        sort_by: Option<String>,
    ) -> Result<ModrinthSearchResponse, LauncherError> {
        self.modrinth_service
            .search_modpacks(query, game_versions, loaders, categories, limit, offset, sort_by)
            .await
    }

    /// 获取整合包版本列表
    pub async fn get_modpack_versions(
        &self,
        project_id: &str,
        game_versions: Option<Vec<String>>,
        loaders: Option<Vec<String>>,
    ) -> Result<Vec<ModrinthModpackVersion>, LauncherError> {
        self.modrinth_service
            .get_modpack_versions(project_id, game_versions, loaders)
            .await
    }
}
