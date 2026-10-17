//! NeoForge 加载器安装
//!
//! 下载 installer jar，取其中的 version.json 与内嵌库落 libraries，1.20.2 起版本号不带 MC 前缀

use crate::errors::LauncherError;
use crate::services::http_client;
use log::{info, warn};
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::Path;

const NEOFORGE_MAVEN_URL: &str = "https://maven.neoforged.net/releases";
const BMCLAPI_NEOFORGE_URL: &str = "https://bmclapi2.bangbang93.com/neoforge";

pub async fn install_neoforge(
    mc_version: &str,
    neoforge_version: &str,
    instance_name: &str,
    game_dir: &Path,
) -> Result<(), LauncherError> {
    info!(
        "安装 NeoForge: MC {} + NeoForge {} -> {}",
        mc_version, neoforge_version, instance_name
    );

    let client = http_client::get_client();

    // 1.20.2 起 NeoForge 版本号不再带 mc 前缀，旧版才是 `mc-ver-loader-ver` 格式
    let full_version = if neoforge_version.contains('.') && !neoforge_version.contains('-') {
        neoforge_version.to_string()
    } else if neoforge_version.contains('-') {
        neoforge_version.to_string()
    } else {
        format!("{}-{}", mc_version, neoforge_version)
    };

    let temp_dir = game_dir.join("temp");
    fs::create_dir_all(&temp_dir)?;
    let installer_path = temp_dir.join(format!("neoforge-{}-installer.jar", full_version));

    let bmclapi_url = format!(
        "{}/version/{}/download/installer.jar",
        BMCLAPI_NEOFORGE_URL, full_version
    );
    let official_url = format!(
        "{}/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        NEOFORGE_MAVEN_URL, full_version, full_version
    );

    let mut downloaded = false;

    info!("尝试从 BMCLAPI 下载 NeoForge installer");
    if let Ok(response) = client.get(&bmclapi_url).send().await {
        if response.status().is_success() {
            if let Ok(bytes) = response.bytes().await {
                if bytes.len() > 1024 {
                    fs::write(&installer_path, &bytes)?;
                    downloaded = true;
                    info!("从 BMCLAPI 下载成功");
                }
            }
        }
    }

    if !downloaded {
        info!("尝试从官方源下载 NeoForge installer: {}", official_url);
        let response =
            client.get(&official_url).send().await.map_err(|e| {
                LauncherError::Custom(format!("下载 NeoForge installer 失败: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(LauncherError::Custom(format!(
                "下载 NeoForge installer 失败: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| LauncherError::Custom(format!("读取 NeoForge installer 失败: {}", e)))?;
        fs::write(&installer_path, &bytes)?;
    }

    let file = fs::File::open(&installer_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut version_json_content: Option<String> = None;
    let libraries_dir = game_dir.join("libraries");

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name()?.to_string();

        if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
            log::warn!("跳过可疑的 zip 条目: {}", name);
            continue;
        }

        if name == "version.json" {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            version_json_content = Some(content);
        } else if name.starts_with("maven/") && !name.ends_with('/') {
            let rel_path = &name[6..];
            if rel_path.contains("..") {
                log::warn!("跳过可疑的 maven 路径: {}", name);
                continue;
            }
            let outpath = libraries_dir.join(rel_path);
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    let version_json_str = version_json_content.ok_or_else(|| {
        LauncherError::Custom("NeoForge installer 中未找到 version.json".to_string())
    })?;

    let mut version_json: Value = serde_json::from_str(&version_json_str)
        .map_err(|e| LauncherError::Custom(format!("解析 NeoForge JSON 失败: {}", e)))?;

    if let Some(obj) = version_json.as_object_mut() {
        obj.insert("id".to_string(), serde_json::json!(instance_name));
    }

    let version_dir = game_dir.join("versions").join(instance_name);
    fs::create_dir_all(&version_dir)?;

    let json_path = version_dir.join(format!("{}.json", instance_name));
    fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

    info!("NeoForge 版本 JSON 已创建: {}", json_path.display());

    if installer_path.exists() {
        let _ = fs::remove_file(&installer_path);
    }

    Ok(())
}

pub async fn get_neoforge_versions(
    mc_version: &str,
) -> Result<Vec<NeoForgeVersion>, LauncherError> {
    let client = http_client::get_client();

    let bmclapi_url = format!("{}/list/{}", BMCLAPI_NEOFORGE_URL, mc_version);

    if let Ok(response) = client.get(&bmclapi_url).send().await {
        if response.status().is_success() {
            if let Ok(versions) = response.json::<Vec<BmclapiNeoForgeVersion>>().await {
                return Ok(versions
                    .into_iter()
                    .map(|v| NeoForgeVersion {
                        version: v.version,
                        mc_version: v.mc_version,
                    })
                    .collect());
            }
        }
    }

    warn!("BMCLAPI 获取 NeoForge 版本失败，返回空列表");
    Ok(vec![])
}

#[derive(serde::Deserialize)]
struct BmclapiNeoForgeVersion {
    version: String,
    #[serde(rename = "mcversion")]
    mc_version: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NeoForgeVersion {
    pub version: String,
    pub mc_version: String,
}
