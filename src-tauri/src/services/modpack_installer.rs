use crate::errors::LauncherError;
use crate::models::modpack::*;
use crate::services::{config, download, modrinth};
use crate::utils::file_utils;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tauri::Emitter;



#[derive(Clone, serde::Serialize)]
pub struct ModpackInstallProgress {
    pub progress: u8,
    pub message: String,
    pub indeterminate: bool,
}

pub struct ModpackInstaller {
    modrinth_service: modrinth::ModrinthService,
}

impl ModpackInstaller {
    pub fn new() -> Self {
        Self {
            modrinth_service: modrinth::ModrinthService::new(),
        }
    }

    /// 安装Modrinth整合包
    pub async fn install_modrinth_modpack(
        &self,
        options: ModpackInstallOptions,
        window: &tauri::Window,
    ) -> Result<(), LauncherError> {
        let config = config::load_config()?;
        let game_dir = PathBuf::from(config.game_dir);
        let instance_dir = game_dir.join("versions").join(&options.instance_name);

        // 发送进度更新
        let send_progress = |progress: u8, message: &str, indeterminate: bool| {
            let _ = window.emit("modpack-install-progress", ModpackInstallProgress {
                progress,
                message: message.to_string(),
                indeterminate,
            });
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
        let modpack = self.modrinth_service
            .get_modpack(&options.modpack_id)
            .await
            .map_err(|e| LauncherError::Custom(format!("获取整合包信息失败: {}", e)))?;

        send_progress(20, "获取整合包版本...", false);

        // 3. 获取指定版本信息
        let versions = self.modrinth_service
            .get_modpack_versions(&options.modpack_id, None, None)
            .await
            .map_err(|e| LauncherError::Custom(format!("获取整合包版本失败: {}", e)))?;

        let selected_version = versions
            .iter()
            .find(|v| v.id == options.version_id)
            .ok_or_else(|| LauncherError::Custom("未找到指定的整合包版本".to_string()))?;

        send_progress(30, "下载整合包文件...", false);

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

        send_progress(50, "解压整合包...", false);

        // 5. 解压整合包
        let extract_dir = temp_dir.join(&options.instance_name);
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir)?;
        }
        fs::create_dir_all(&extract_dir)?;

        self.extract_modpack(&modpack_file_path, &extract_dir)
            .await
            .map_err(|e| LauncherError::Custom(format!("解压整合包失败: {}", e)))?;

        send_progress(70, "处理整合包配置...", false);

        // 6. 处理整合包配置
        self.process_modpack_config(&extract_dir, &instance_dir, &modpack, selected_version)
            .await
            .map_err(|e| LauncherError::Custom(format!("处理整合包配置失败: {}", e)))?;

        send_progress(90, "下载游戏文件...", true);

        // 7. 下载游戏文件
        self.download_game_files(&instance_dir, window)
            .await
            .map_err(|e| LauncherError::Custom(format!("下载游戏文件失败: {}", e)))?;

        // 8. 清理临时文件
        if modpack_file_path.exists() {
            let _ = fs::remove_file(&modpack_file_path);
        }
        if extract_dir.exists() {
            let _ = fs::remove_dir_all(&extract_dir);
        }

        send_progress(100, "整合包安装完成！", false);

        Ok(())
    }

    /// 解压整合包文件
    async fn extract_modpack(
        &self,
        modpack_file_path: &PathBuf,
        extract_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        let file_extension = modpack_file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match file_extension {
            "zip" => {
                // 使用标准库解压ZIP文件
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
            }
            "mrpack" => {
                // Modrinth包格式，也是ZIP格式
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
            }
            _ => {
                return Err(LauncherError::Custom(format!(
                    "不支持的整合包格式: {}",
                    file_extension
                )));
            }
        }

        Ok(())
    }

    /// 处理整合包配置
    async fn process_modpack_config(
        &self,
        extract_dir: &PathBuf,
        instance_dir: &PathBuf,
        modpack: &ModrinthModpack,
        version: &ModrinthModpackVersion,
    ) -> Result<(), LauncherError> {
        // 1. 创建实例目录
        fs::create_dir_all(instance_dir)?;

        // 2. 检查并处理modrinth.index.json
        let index_path = extract_dir.join("modrinth.index.json");
        if index_path.exists() {
            // 处理Modrinth格式的整合包
            self.process_modrinth_format(extract_dir, instance_dir, &index_path).await?;
        } else {
            // 处理传统格式的整合包（直接复制文件）
            self.process_legacy_format(extract_dir, instance_dir).await?;
        }

        // 3. 创建实例配置文件
        let instance_config = serde_json::json!({
            "id": modpack.slug.clone(),
            "name": modpack.title.clone(),
            "type": "modpack",
            "source": "modrinth",
            "modpack_id": modpack.slug.clone(),
            "modpack_version": version.version_number.clone(),
            "minecraft": version.game_versions.first().cloned().unwrap_or_default(),
            "loaders": version.loaders.clone(),
            "created": chrono::Utc::now().to_rfc3339(),
        });

        let config_path = instance_dir.join("instance.json");
        fs::write(config_path, serde_json::to_string_pretty(&instance_config)?)?;

        Ok(())
    }

    /// 处理Modrinth格式的整合包
    async fn process_modrinth_format(
        &self,
        extract_dir: &PathBuf,
        instance_dir: &PathBuf,
        index_path: &PathBuf,
    ) -> Result<(), LauncherError> {
        let index_content = fs::read_to_string(index_path)?;
        let index: Value = serde_json::from_str(&index_content)?;

        // 首先复制已解压的文件（排除 modrinth.index.json）
        for entry in fs::read_dir(extract_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            
            // 跳过 modrinth.index.json 文件
            if file_name == "modrinth.index.json" {
                continue;
            }
            
            let dest_path = instance_dir.join(&file_name);
            
            if path.is_dir() {
                file_utils::copy_dir_all(&path, &dest_path)?;
            } else {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&path, &dest_path)?;
            }
        }

        // 处理依赖文件（从 modrinth.index.json 中定义的文件）
        if let Some(files) = index["files"].as_array() {
            for file_info in files {
                if let (Some(path), Some(downloads)) = (
                    file_info["path"].as_str(),
                    file_info["downloads"].as_array(),
                ) {
                    if let Some(download_url) = downloads.first().and_then(|d| d.as_str()) {
                        let dest_path = instance_dir.join(path);
                        
                        if let Some(parent) = dest_path.parent() {
                            fs::create_dir_all(parent)?;
                        }

                        // 下载文件
                        self.modrinth_service
                            .download_modpack_file(download_url, &dest_path)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 处理传统格式的整合包
    async fn process_legacy_format(
        &self,
        extract_dir: &PathBuf,
        instance_dir: &PathBuf,
    ) -> Result<(), LauncherError> {
        // 直接复制所有文件到实例目录
        file_utils::copy_dir_all(extract_dir, instance_dir)?;

        Ok(())
    }

    /// 下载游戏文件
    async fn download_game_files(
        &self,
        instance_dir: &PathBuf,
        window: &tauri::Window,
    ) -> Result<(), LauncherError> {
        // 检查是否存在版本JSON文件
        let version_json_path = instance_dir.join("version.json");
        if version_json_path.exists() {
            let json_content = fs::read_to_string(&version_json_path)?;
            let version_json: Value = serde_json::from_str(&json_content)?;

            // 使用现有的下载服务下载游戏文件
            if let Some(id) = version_json["id"].as_str() {
                let config = config::load_config()?;
                download::process_and_download_version(
                    id.to_string(),
                    config.download_mirror.clone(),
                    window,
                ).await?;
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