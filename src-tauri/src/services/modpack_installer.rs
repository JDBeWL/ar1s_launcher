//! 整合包安装模块
//!
//! 下载解压 Modrinth 整合包，按 modrinth.index.json 并发补下模组与依赖加载器，失败或取消时回滚实例

use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::models::modpack::*;
use crate::services::launcher::isolation;
use crate::services::{config, download, http_client, loaders, modrinth};
use crate::utils::file_utils::{self, validate_instance_name_or_error};
use log::{error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

static MODPACK_CANCEL_FLAG: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

const MAX_EXTRACTED_BYTES: u64 = 8 * 1024 * 1024 * 1024;

fn get_cancel_flag() -> Arc<AtomicBool> {
    MODPACK_CANCEL_FLAG
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

pub fn reset_modpack_cancel_flag() {
    get_cancel_flag().store(false, Ordering::SeqCst);
}

pub fn set_modpack_cancel_flag() {
    get_cancel_flag().store(true, Ordering::SeqCst);
    // 原版下载走独立的取消标志；不联动的话取消要等整轮下载跑完才生效
    crate::services::download::batch::set_cancel_flag();
}

fn is_cancelled() -> bool {
    get_cancel_flag().load(Ordering::SeqCst)
}

fn check_cancelled() -> Result<(), LauncherError> {
    if is_cancelled() {
        Err(LauncherError::Cancelled)
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
    http_client: &'static Client,
}

static INSTALLER: std::sync::LazyLock<ModpackInstaller> =
    std::sync::LazyLock::new(ModpackInstaller::new);

pub fn get_installer() -> &'static ModpackInstaller {
    &INSTALLER
}

impl ModpackInstaller {
    fn new() -> Self {
        Self {
            modrinth_service: modrinth::ModrinthService::new(),
            http_client: http_client::get_client(),
        }
    }

    pub async fn install_modrinth_modpack(
        &self,
        options: ModpackInstallOptions,
        window: &tauri::Window,
    ) -> Result<(), LauncherError> {
        reset_modpack_cancel_flag();

        validate_instance_name_or_error(&options.instance_name)?;

        let config = config::load_config()?;
        let game_dir = PathBuf::from(&config.game_dir);
        let instance_dir = game_dir.join("versions").join(&options.instance_name);
        // 版本信息写在实例目录，模组/配置/存档按隔离开关落到实例目录或共享根目录
        let content_root = isolation::content_root(&config, &game_dir, &options.instance_name);
        if !config.version_isolation {
            warn!(
                "版本隔离已关闭：整合包内容会写入共享目录 {}",
                content_root.display()
            );
        }
        let temp_dir = game_dir.join("temp");
        let extract_dir = temp_dir.join(format!("{}_extract", &options.instance_name));

        // 与实例创建/删除/重命名共用一把锁；失败清理会删实例目录，不能和另一实例互相踩
        let _install_lock = crate::utils::file_lock::acquire_async(
            &game_dir.join(".download_state/instances.lock"),
            std::time::Duration::from_secs(2),
        )
        .await?;

        if instance_dir.exists() {
            return Err(LauncherError::Custom(format!(
                "名为 '{}' 的实例已存在，请使用其他名称",
                options.instance_name
            )));
        }

        let result = self
            .do_install_modrinth_modpack(
                &options,
                window,
                &game_dir,
                &instance_dir,
                &content_root,
                &temp_dir,
                &extract_dir,
            )
            .await;

        if result.is_err() {
            info!("安装失败或被取消，清理已创建的文件...");

            if instance_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&instance_dir) {
                    warn!("清理实例目录失败: {}", e);
                } else {
                    info!("已清理实例目录: {}", instance_dir.display());
                }
            }

            if !config.version_isolation && content_root != instance_dir {
                // 关闭隔离时内容是共享的，删掉就等于删别人的文件，只能提示手动处理
                warn!(
                    "版本隔离已关闭，本次写入共享目录 {} 的文件不会被自动清理",
                    content_root.display()
                );
            }

            if extract_dir.exists() {
                let _ = fs::remove_dir_all(&extract_dir);
            }
        }

        result
    }

    async fn do_install_modrinth_modpack(
        &self,
        options: &ModpackInstallOptions,
        window: &tauri::Window,
        game_dir: &PathBuf,
        instance_dir: &PathBuf,
        content_root: &PathBuf,
        temp_dir: &PathBuf,
        extract_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
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

        let modpack = self
            .modrinth_service
            .get_modpack(&options.modpack_id)
            .await
            .map_err(|e| LauncherError::Custom(format!("获取整合包信息失败: {}", e)))?;

        send_progress(15, "获取整合包版本...", false);
        check_cancelled()?;

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

        let primary_file = selected_version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| selected_version.files.first())
            .ok_or_else(|| LauncherError::Custom("整合包没有可用的文件".to_string()))?;

        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)?;
        }

        file_utils::validate_file_name_or_error(&primary_file.filename)?;
        let modpack_file_path = temp_dir.join(&primary_file.filename);
        let sha1 = (!primary_file.hashes.sha1.trim().is_empty())
            .then_some(primary_file.hashes.sha1.as_str());
        let cancel_flag = get_cancel_flag();

        self.modrinth_service
            .download_modpack_file(
                &primary_file.url,
                &modpack_file_path,
                primary_file.size,
                sha1,
                &cancel_flag,
            )
            .await
            .map_err(|e| {
                if e.is_cancelled() {
                    LauncherError::Cancelled
                } else {
                    LauncherError::Custom(format!("下载整合包文件失败: {}", e))
                }
            })?;

        send_progress(35, "解压整合包...", false);
        check_cancelled()?;

        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        fs::create_dir_all(&extract_dir)?;

        self.extract_modpack(&modpack_file_path, &extract_dir)
            .await
            .map_err(|e| LauncherError::Custom(format!("解压整合包失败: {}", e)))?;

        send_progress(45, "处理整合包配置...", false);
        check_cancelled()?;

        let index_path = extract_dir.join("modrinth.index.json");
        let modrinth_index = if index_path.exists() {
            let content = fs::read_to_string(&index_path)?;
            Some(
                serde_json::from_str::<ModrinthIndex>(&content).map_err(|e| {
                    LauncherError::Custom(format!("解析 modrinth.index.json 失败: {}", e))
                })?,
            )
        } else {
            None
        };

        fs::create_dir_all(&instance_dir)?;
        if content_root != instance_dir {
            fs::create_dir_all(content_root)?;
        }

        send_progress(50, "复制整合包文件...", false);
        check_cancelled()?;

        let overrides_dir = extract_dir.join("overrides");
        if overrides_dir.exists() {
            info!("复制 overrides 目录到实例");
            file_utils::copy_dir_all(&overrides_dir, content_root)?;
        }

        // 某些整合包用 client-overrides 而非 overrides
        let client_overrides_dir = extract_dir.join("client-overrides");
        if client_overrides_dir.exists() {
            info!("复制 client-overrides 目录到实例");
            file_utils::copy_dir_all(&client_overrides_dir, content_root)?;
        }

        if let Some(ref index) = modrinth_index {
            send_progress(55, "下载模组文件...", false);
            check_cancelled()?;
            self.download_modpack_files(&index.files, content_root, window)
                .await?;
        }

        send_progress(75, "安装游戏版本...", false);
        check_cancelled()?;

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

        let mc_version = modrinth_index
            .as_ref()
            .map(|i| i.dependencies.minecraft.clone())
            .or_else(|| selected_version.game_versions.first().cloned())
            .unwrap_or_default();

        let loader_type = if modrinth_index
            .as_ref()
            .map(|i| i.dependencies.forge.is_some())
            .unwrap_or(false)
        {
            Some("forge")
        } else if modrinth_index
            .as_ref()
            .map(|i| i.dependencies.fabric_loader.is_some() || i.dependencies.fabric.is_some())
            .unwrap_or(false)
        {
            Some("fabric")
        } else if modrinth_index
            .as_ref()
            .map(|i| i.dependencies.quilt_loader.is_some() || i.dependencies.quilt.is_some())
            .unwrap_or(false)
        {
            Some("quilt")
        } else if modrinth_index
            .as_ref()
            .map(|i| i.dependencies.neoforge.is_some())
            .unwrap_or(false)
        {
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
            "icon_url": modpack.icon_url.clone(),
            "loaders": selected_version.loaders.clone(),
            "created": chrono::Utc::now().to_rfc3339(),
        });

        let config_path = instance_dir.join("instance.json");
        fs::write(config_path, serde_json::to_string_pretty(&instance_config)?)?;

        if modpack_file_path.exists() {
            let _ = fs::remove_file(&modpack_file_path);
        }
        if extract_dir.exists() {
            let _ = fs::remove_dir_all(&extract_dir);
        }

        send_progress(100, "整合包安装完成！", false);
        info!("整合包 {} 安装完成", options.instance_name);

        if let Ok(game_dir_str) = config::get_game_dir() {
            let _ = window.emit("game-dir-changed", &game_dir_str);
        }

        Ok(())
    }

    /// 并发下载整合包文件，复用 download_file 的断点续传
    async fn download_modpack_files(
        &self,
        files: &[ModrinthIndexFile],
        instance_dir: &PathBuf,
        window: &tauri::Window,
    ) -> Result<(), LauncherError> {
        let total_files = files.len();
        info!("开始下载 {} 个文件", total_files);

        if total_files == 0 {
            return Ok(());
        }

        // 清单里的 path 由远端给；出现遍历路径说明包本身有问题，整包拒绝而不是装一半
        if let Some(bad) = files
            .iter()
            .find(|file| !file_utils::is_safe_relative_path(&file.path))
        {
            return Err(LauncherError::Custom(format!(
                "整合包清单包含非法文件路径，已拒绝安装: {}",
                bad.path
            )));
        }

        let dl_config = config::load_config()?;
        let max_concurrent = (dl_config.download_threads as usize).max(1);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        let completed_count = Arc::new(AtomicUsize::new(0));
        let failed_files = Arc::new(Mutex::new(Vec::<String>::new()));
        let client = self.http_client;

        // download_file 靠这两个标志在流式读取中途响应取消
        let running = Arc::new(AtomicBool::new(true));
        let cancelled = get_cancel_flag();
        // 进度按文件数计算，字节计数只是 download_file 的出参
        let bytes_downloaded = Arc::new(AtomicU64::new(0));
        let bytes_since_last = Arc::new(AtomicU64::new(0));

        let mut handles = Vec::with_capacity(total_files);

        for file in files {
            check_cancelled()?;

            let sem = semaphore.clone();
            let completed = completed_count.clone();
            let window_clone = window.clone();
            let file_path = file.path.clone();
            let total = total_files;
            let failed = failed_files.clone();
            let running = running.clone();
            let cancelled = cancelled.clone();
            let bytes_downloaded = bytes_downloaded.clone();
            let bytes_since_last = bytes_since_last.clone();
            // Modrinth 每个文件给多个镜像：第一个当主源、第二个当备源
            let job = DownloadJob {
                url: file.downloads.first().cloned().unwrap_or_default(),
                fallback_url: file.downloads.get(1).cloned(),
                path: instance_dir.join(&file.path),
                size: file.file_size.unwrap_or(0),
                // index.json 允许 sha1 为空串，为空时不校验，否则好文件会被判坏
                hash: (!file.hashes.sha1.is_empty()).then(|| file.hashes.sha1.clone()),
            };

            let handle = tokio::spawn(async move {
                let _permit = match sem.acquire().await {
                    Ok(p) => p,
                    Err(_) => return,
                };

                if is_cancelled() {
                    return;
                }

                let current = completed.fetch_add(1, Ordering::Relaxed) + 1;
                let progress = 55 + ((current as f32 / total as f32) * 20.0) as u8;
                let _ = window_clone.emit(
                    "modpack-install-progress",
                    ModpackInstallProgress {
                        progress,
                        message: format!("下载文件 ({}/{}): {}", current, total, file_path),
                        indeterminate: false,
                    },
                );

                // 重试时 .part 已经落了半截，download_file 会带 Range 续传而不是从头再下
                let mut last_error: Option<LauncherError> = None;
                for attempt in 0..3 {
                    if is_cancelled() {
                        return;
                    }
                    match download::download_file(
                        client,
                        &job,
                        &job.url,
                        &running,
                        &cancelled,
                        &bytes_downloaded,
                        &bytes_since_last,
                    )
                    .await
                    {
                        Ok(()) => {
                            last_error = None;
                            break;
                        }
                        Err(e) => {
                            if e.is_cancelled() {
                                return;
                            }
                            warn!("下载失败 {} (第 {} 次尝试): {}", file_path, attempt + 1, e);
                            last_error = Some(e);
                            if attempt < 2 {
                                tokio::time::sleep(std::time::Duration::from_secs(1 << attempt))
                                    .await;
                            }
                        }
                    }
                }

                if let Some(e) = last_error {
                    error!("无法下载文件 {}: {}", file_path, e);
                    failed.lock().unwrap().push(file_path);
                }
            });

            handles.push(handle);
        }

        let mut task_panics = 0usize;
        for handle in handles {
            if let Err(e) = handle.await {
                task_panics += 1;
                error!("整合包下载任务异常终止: {}", e);
            }
        }

        check_cancelled()?;

        if task_panics > 0 {
            return Err(LauncherError::Custom(format!(
                "{} 个下载任务异常终止，整合包可能不完整",
                task_panics
            )));
        }

        let failed = failed_files.lock().unwrap();
        if !failed.is_empty() {
            const MAX_SHOWN: usize = 5;
            let shown = failed
                .iter()
                .take(MAX_SHOWN)
                .cloned()
                .collect::<Vec<_>>()
                .join("、");
            let suffix = if failed.len() > MAX_SHOWN {
                format!(" 等 {} 个文件", failed.len())
            } else {
                String::new()
            };
            return Err(LauncherError::Custom(format!(
                "{} 个文件下载失败，整合包可能不完整：{}{}",
                failed.len(),
                shown,
                suffix
            )));
        }

        Ok(())
    }

    async fn install_game_and_loader(
        &self,
        deps: &ModrinthDependencies,
        instance_name: &str,
        game_dir: &PathBuf,
        window: &tauri::Window,
    ) -> Result<(), LauncherError> {
        let mc_version = &deps.minecraft;
        info!("安装 Minecraft {}", mc_version);
        let cancel = get_cancel_flag();

        let config = config::load_config()?;
        let download_result = download::process_and_download_version(
            mc_version.clone(),
            config.download_mirror.clone(),
            window,
        )
        .await;
        // 下载被取消时错误文案来自下载模块，这里统一成安装取消
        check_cancelled()?;
        download_result?;

        if let Some(forge_version) = &deps.forge {
            info!("安装 Forge {}", forge_version);
            let result = loaders::install_loader(
                &loaders::LoaderType::Forge {
                    mc_version: mc_version.clone(),
                    loader_version: forge_version.clone(),
                },
                instance_name,
                game_dir,
                Some(cancel.as_ref()),
            )
            .await;
            check_cancelled()?;
            result?;
        } else if let Some(fabric_version) = deps.fabric_loader.as_ref().or(deps.fabric.as_ref()) {
            info!("安装 Fabric {}", fabric_version);
            let result = loaders::install_loader(
                &loaders::LoaderType::Fabric {
                    mc_version: mc_version.clone(),
                    loader_version: fabric_version.clone(),
                },
                instance_name,
                game_dir,
                Some(cancel.as_ref()),
            )
            .await;
            check_cancelled()?;
            result?;
        } else if let Some(quilt_version) = deps.quilt_loader.as_ref().or(deps.quilt.as_ref()) {
            info!("安装 Quilt {}", quilt_version);
            let result = loaders::install_loader(
                &loaders::LoaderType::Quilt {
                    mc_version: mc_version.clone(),
                    loader_version: quilt_version.clone(),
                },
                instance_name,
                game_dir,
                Some(cancel.as_ref()),
            )
            .await;
            check_cancelled()?;
            result?;
        } else if let Some(neoforge_version) = &deps.neoforge {
            info!("安装 NeoForge {}", neoforge_version);
            let result = loaders::install_loader(
                &loaders::LoaderType::NeoForge {
                    mc_version: mc_version.clone(),
                    loader_version: neoforge_version.clone(),
                },
                instance_name,
                game_dir,
                Some(cancel.as_ref()),
            )
            .await;
            check_cancelled()?;
            result?;
        } else {
            // 无加载器依赖时手动补一份版本 JSON
            self.create_vanilla_version_json(mc_version, instance_name, game_dir)?;
        }

        Ok(())
    }

    fn create_loader_version_json(
        &self,
        instance_name: &str,
        inherits_from: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        let version_dir = game_dir.join("versions").join(instance_name);
        fs::create_dir_all(&version_dir)?;

        let json_path = version_dir.join(format!("{}.json", instance_name));

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

    fn create_vanilla_version_json(
        &self,
        mc_version: &str,
        instance_name: &str,
        game_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        self.create_loader_version_json(instance_name, mc_version, game_dir)
    }

    /// 解压整合包，防止 zip 路径遍历与 zip bomb
    async fn extract_modpack(
        &self,
        modpack_file_path: &PathBuf,
        extract_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        extract_zip_limited(modpack_file_path, extract_dir, MAX_EXTRACTED_BYTES)
    }

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
            .search_modpacks(
                query,
                game_versions,
                loaders,
                categories,
                limit,
                offset,
                sort_by,
            )
            .await
    }

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

fn extract_zip_limited(
    modpack_file_path: &std::path::Path,
    extract_dir: &std::path::Path,
    max_bytes: u64,
) -> Result<(), LauncherError> {
    fs::create_dir_all(extract_dir)?;

    // 先按 central directory 声明的未压缩大小做快速上限检查
    let file = fs::File::open(modpack_file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut declared_total = 0u64;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name()?.to_string();
        if name.ends_with('/') || !file_utils::is_safe_relative_path(&name) {
            continue;
        }
        declared_total = declared_total.saturating_add(entry.size());
        if declared_total > max_bytes {
            return Err(LauncherError::Custom(format!(
                "整合包解压后超过 {} 字节上限，已中止",
                max_bytes
            )));
        }
    }
    drop(archive);

    let file = fs::File::open(modpack_file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut actual_total = 0u64;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let file_name = entry.name()?.to_string();

        if !file_utils::is_safe_relative_path(&file_name) {
            log::warn!("跳过可疑的 zip 条目: {}", file_name);
            continue;
        }

        let outpath = extract_dir.join(&file_name);

        if file_name.ends_with('/') {
            fs::create_dir_all(&outpath)?;
            continue;
        }

        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut outfile = fs::File::create(&outpath)?;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            actual_total = actual_total.saturating_add(read as u64);
            if actual_total > max_bytes {
                return Err(LauncherError::Custom(format!(
                    "整合包解压后超过 {} 字节上限，已中止",
                    max_bytes
                )));
            }
            outfile.write_all(&buffer[..read])?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::extract_zip_limited;
    use std::fs;
    use std::io::Write;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    fn write_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        for (name, content) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn test_extract_zip_enforces_uncompressed_limit() {
        let dir = tempfile::TempDir::new().unwrap();
        let zip_path = dir.path().join("pack.zip");
        let extract_dir = dir.path().join("extract");
        fs::create_dir_all(&extract_dir).unwrap();
        write_zip(&zip_path, &[("a.txt", b"0123456789")]);

        let error = extract_zip_limited(&zip_path, &extract_dir, 5).unwrap_err();
        assert!(error.to_string().contains("上限"));

        assert!(extract_zip_limited(&zip_path, &extract_dir, 10).is_ok());
    }

    #[test]
    fn test_extract_zip_skips_path_traversal() {
        let dir = tempfile::TempDir::new().unwrap();
        let zip_path = dir.path().join("pack.zip");
        let extract_dir = dir.path().join("extract");
        fs::create_dir_all(&extract_dir).unwrap();
        write_zip(&zip_path, &[("ok.txt", b"ok"), ("../evil.txt", b"evil")]);

        extract_zip_limited(&zip_path, &extract_dir, 1024).unwrap();

        assert!(extract_dir.join("ok.txt").exists());
        assert!(!dir.path().join("evil.txt").exists());
    }
}
