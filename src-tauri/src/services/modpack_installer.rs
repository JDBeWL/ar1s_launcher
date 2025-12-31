use crate::errors::LauncherError;
use crate::models::modpack::*;
use crate::services::{config, download, loaders, modrinth};
use crate::utils::file_utils::{self, validate_instance_name_or_error};
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

/// 全局取消标志
static MODPACK_CANCEL_FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

/// 获取或初始化取消标志
fn get_cancel_flag() -> Arc<AtomicBool> {
    MODPACK_CANCEL_FLAG
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

/// 重置取消标志
pub fn reset_modpack_cancel_flag() {
    // 确保标志已初始化，然后重置为 false
    get_cancel_flag().store(false, Ordering::SeqCst);
}

/// 设置取消标志
pub fn set_modpack_cancel_flag() {
    get_cancel_flag().store(true, Ordering::SeqCst);
}

/// 检查是否已取消
fn is_cancelled() -> bool {
    get_cancel_flag().load(Ordering::SeqCst)
}

/// 检查取消状态，如果已取消则返回错误
fn check_cancelled() -> Result<(), LauncherError> {
    if is_cancelled() {
        Err(LauncherError::Custom("安装已取消".to_string()))
    } else {
        Ok(())
    }
}

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
        // 重置取消标志
        reset_modpack_cancel_flag();
        
        // 验证实例名称
        validate_instance_name_or_error(&options.instance_name)?;
        
        let config = config::load_config()?;
        let game_dir = PathBuf::from(&config.game_dir);
        let instance_dir = game_dir.join("versions").join(&options.instance_name);
        let temp_dir = game_dir.join("temp");
        let extract_dir = temp_dir.join(format!("{}_extract", &options.instance_name));

        // 1. 检查实例是否已存在
        if instance_dir.exists() {
            return Err(LauncherError::Custom(format!(
                "名为 '{}' 的实例已存在，请使用其他名称",
                options.instance_name
            )));
        }

        // 执行安装，如果失败或取消则清理
        let result = self.do_install_modrinth_modpack(&options, window, &game_dir, &instance_dir, &temp_dir, &extract_dir).await;
        
        // 如果安装失败或被取消，清理已创建的目录
        if result.is_err() {
            info!("安装失败或被取消，清理已创建的文件...");
            
            // 清理实例目录
            if instance_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&instance_dir) {
                    warn!("清理实例目录失败: {}", e);
                } else {
                    info!("已清理实例目录: {}", instance_dir.display());
                }
            }
            
            // 清理解压目录
            if extract_dir.exists() {
                let _ = fs::remove_dir_all(&extract_dir);
            }
        }
        
        result
    }
    
    /// 执行实际的整合包安装逻辑
    async fn do_install_modrinth_modpack(
        &self,
        options: &ModpackInstallOptions,
        window: &tauri::Window,
        game_dir: &PathBuf,
        instance_dir: &PathBuf,
        temp_dir: &PathBuf,
        extract_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
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
        check_cancelled()?;

        send_progress(10, "获取整合包信息...", false);
        check_cancelled()?;

        // 2. 获取整合包详细信息
        let modpack = self
            .modrinth_service
            .get_modpack(&options.modpack_id)
            .await
            .map_err(|e| LauncherError::Custom(format!("获取整合包信息失败: {}", e)))?;

        send_progress(15, "获取整合包版本...", false);
        check_cancelled()?;

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
        check_cancelled()?;

        // 4. 下载整合包文件
        let primary_file = selected_version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| selected_version.files.first())
            .ok_or_else(|| LauncherError::Custom("整合包没有可用的文件".to_string()))?;

        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)?;
        }

        let modpack_file_path = temp_dir.join(&primary_file.filename);

        self.modrinth_service
            .download_modpack_file(&primary_file.url, &modpack_file_path)
            .await
            .map_err(|e| LauncherError::Custom(format!("下载整合包文件失败: {}", e)))?;

        send_progress(35, "解压整合包...", false);
        check_cancelled()?;

        // 5. 解压整合包
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        fs::create_dir_all(&extract_dir)?;

        self.extract_modpack(&modpack_file_path, &extract_dir)
            .await
            .map_err(|e| LauncherError::Custom(format!("解压整合包失败: {}", e)))?;

        send_progress(45, "处理整合包配置...", false);
        check_cancelled()?;

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
        check_cancelled()?;

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
            check_cancelled()?;
            self.download_modpack_files(&index.files, &instance_dir, window)
                .await?;
        }

        send_progress(75, "安装游戏版本...", false);
        check_cancelled()?;

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
        check_cancelled()?;

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
            // 检查是否已取消
            check_cancelled()?;
            
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
                // 每次下载前检查取消状态
                if is_cancelled() {
                    return Err(LauncherError::Custom("安装已取消".to_string()));
                }
                
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

        // 安装加载器（使用统一的 loaders 模块）
        if let Some(forge_version) = &deps.forge {
            info!("安装 Forge {}", forge_version);
            loaders::install_loader(
                &loaders::LoaderType::Forge {
                    mc_version: mc_version.clone(),
                    loader_version: forge_version.clone(),
                },
                instance_name,
                game_dir,
            ).await?;
        } else if let Some(fabric_version) = deps.fabric_loader.as_ref().or(deps.fabric.as_ref()) {
            info!("安装 Fabric {}", fabric_version);
            loaders::install_loader(
                &loaders::LoaderType::Fabric {
                    mc_version: mc_version.clone(),
                    loader_version: fabric_version.clone(),
                },
                instance_name,
                game_dir,
            ).await?;
        } else if let Some(quilt_version) = deps.quilt_loader.as_ref().or(deps.quilt.as_ref()) {
            info!("安装 Quilt {}", quilt_version);
            loaders::install_loader(
                &loaders::LoaderType::Quilt {
                    mc_version: mc_version.clone(),
                    loader_version: quilt_version.clone(),
                },
                instance_name,
                game_dir,
            ).await?;
        } else if let Some(neoforge_version) = &deps.neoforge {
            info!("安装 NeoForge {}", neoforge_version);
            loaders::install_loader(
                &loaders::LoaderType::NeoForge {
                    mc_version: mc_version.clone(),
                    loader_version: neoforge_version.clone(),
                },
                instance_name,
                game_dir,
            ).await?;
        } else {
            // 纯净版，创建版本 JSON
            self.create_vanilla_version_json(mc_version, instance_name, game_dir)?;
        }

        Ok(())
    }

    /// 创建指向加载器版本的版本 JSON
    fn create_loader_version_json(
        &self,
        instance_name: &str,
        inherits_from: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        let version_dir = game_dir.join("versions").join(instance_name);
        fs::create_dir_all(&version_dir)?;

        let json_path = version_dir.join(format!("{}.json", instance_name));
        
        // 如果已经存在，不覆盖
        if json_path.exists() {
            return Ok(());
        }

        let version_json = serde_json::json!({
            "id": instance_name,
            "inheritsFrom": inherits_from,
            "type": "release"
        });

        fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;
        info!("创建版本 JSON: {}", json_path.display());

        Ok(())
    }

    /// 创建纯净版版本 JSON
    fn create_vanilla_version_json(
        &self,
        mc_version: &str,
        instance_name: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        self.create_loader_version_json(instance_name, mc_version, game_dir)
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
            let file_name = file.name().to_string();
            
            // 安全检查：防止路径遍历攻击
            // 检查是否包含 ".." 或绝对路径
            if file_name.contains("..") || file_name.starts_with('/') || file_name.starts_with('\\') {
                log::warn!("跳过可疑的 zip 条目: {}", file_name);
                continue;
            }
            
            // 在 Windows 上也检查驱动器路径 (如 C:)
            #[cfg(windows)]
            if file_name.len() >= 2 && file_name.chars().nth(1) == Some(':') {
                log::warn!("跳过可疑的 zip 条目 (绝对路径): {}", file_name);
                continue;
            }
            
            let outpath = extract_dir.join(&file_name);
            
            // 确保解压路径在目标目录内
            let canonical_extract = extract_dir.canonicalize().unwrap_or_else(|_| extract_dir.clone());
            if let Ok(canonical_out) = outpath.canonicalize() {
                if !canonical_out.starts_with(&canonical_extract) {
                    log::warn!("跳过路径遍历尝试: {} -> {}", file_name, canonical_out.display());
                    continue;
                }
            }

            if file_name.ends_with('/') {
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
