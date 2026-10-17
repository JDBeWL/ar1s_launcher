//! Quilt 加载器安装
//!
//! 从 meta.quiltmc.org v3 取 profile JSON，改写 id 为实例名后写入 versions 目录；
//! 安装前校验加载器自带的 ASM 能不能读游戏类的 class 版本

use crate::errors::LauncherError;
use crate::services::http_client;
use crate::services::launcher::java_requirement;
use log::{info, warn};
use serde_json::Value;
use std::fs;
use std::path::Path;

const QUILT_META_URL: &str = "https://meta.quiltmc.org/v3";

pub async fn install_quilt(
    mc_version: &str,
    quilt_version: &str,
    instance_name: &str,
    game_dir: &Path,
) -> Result<(), LauncherError> {
    info!(
        "安装 Quilt: MC {} + Quilt {} -> {}",
        mc_version, quilt_version, instance_name
    );

    let client = http_client::get_client();

    let profile_url = format!(
        "{}/versions/loader/{}/{}/profile/json",
        QUILT_META_URL, mc_version, quilt_version
    );

    info!("获取 Quilt 版本信息: {}", profile_url);

    let response = client
        .get(&profile_url)
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

    check_asm_support(&version_json, mc_version, quilt_version, game_dir).await?;

    if let Some(obj) = version_json.as_object_mut() {
        obj.insert("id".to_string(), serde_json::json!(instance_name));
    }

    let version_dir = game_dir.join("versions").join(instance_name);
    fs::create_dir_all(&version_dir)?;

    let json_path = version_dir.join(format!("{}.json", instance_name));
    fs::write(&json_path, serde_json::to_string_pretty(&version_json)?)?;

    info!("Quilt 版本 JSON 已创建: {}", json_path.display());

    Ok(())
}

pub async fn get_quilt_versions(
    mc_version: &str,
) -> Result<Vec<QuiltLoaderVersion>, LauncherError> {
    let client = http_client::get_client();
    let url = format!("{}/versions/loader/{}", QUILT_META_URL, mc_version);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("获取 Quilt 版本列表失败: {}", e)))?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取 Quilt 版本列表失败: {}",
            response.status()
        )));
    }

    let versions: Vec<QuiltLoaderInfo> = response
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析 Quilt 版本列表失败: {}", e)))?;

    let mut versions: Vec<QuiltLoaderVersion> = versions
        .into_iter()
        .map(|v| QuiltLoaderVersion {
            stable: is_stable(&v.loader.version),
            version: v.loader.version,
        })
        .collect();

    // 接口按「兼容性登记顺序」返回，第一条可能是几年前的老版本；
    // 调用方默认取第一条，所以这里必须自己按版本号倒序
    versions.sort_by(|a, b| version_key(&b.version).cmp(&version_key(&a.version)));
    versions.dedup_by(|a, b| a.version == b.version);

    Ok(versions)
}

pub async fn get_quilt_game_versions() -> Result<Vec<String>, LauncherError> {
    let client = http_client::get_client();
    let url = format!("{}/versions/game", QUILT_META_URL);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| LauncherError::Custom(format!("获取游戏版本列表失败: {}", e)))?;

    if !response.status().is_success() {
        return Err(LauncherError::Custom(format!(
            "获取游戏版本列表失败: {}",
            response.status()
        )));
    }

    let versions: Vec<QuiltGameVersion> = response
        .json()
        .await
        .map_err(|e| LauncherError::Custom(format!("解析游戏版本列表失败: {}", e)))?;

    Ok(versions.into_iter().map(|v| v.version).collect())
}

#[derive(serde::Deserialize)]
struct QuiltLoaderInfo {
    loader: QuiltLoader,
}

#[derive(serde::Deserialize)]
struct QuiltLoader {
    version: String,
}

#[derive(serde::Deserialize)]
struct QuiltGameVersion {
    version: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QuiltLoaderVersion {
    pub version: String,
    /// 不带 `-beta` 之类的后缀就算正式版
    pub stable: bool,
}

/// 版本号比较键：`0.31.0-beta.4` → (0, 31, 0, 0, 4)，正式版稳定位排 1
fn version_key(raw: &str) -> (u32, u32, u32, u8, u32) {
    let (core, pre) = match raw.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (raw, None),
    };

    let mut parts = core.split('.').map(|part| part.parse::<u32>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);

    match pre {
        // 同一 x.y.z 下正式版高于预发布版
        None => (major, minor, patch, 1, 0),
        Some(pre) => {
            let build = pre
                .rsplit('.')
                .next()
                .and_then(|num| num.parse::<u32>().ok())
                .unwrap_or(0);
            (major, minor, patch, 0, build)
        }
    }
}

fn is_stable(version: &str) -> bool {
    !version.contains('-')
}

/// profile 里的 ASM 版本，坐标是 `org.ow2.asm:asm:x.y.z`
fn profile_asm_version(profile: &Value) -> Option<String> {
    profile
        .get("libraries")?
        .as_array()?
        .iter()
        .find_map(|library| {
            let name = library.get("name")?.as_str()?;
            let mut parts = name.split(':');
            let group = parts.next()?;
            let artifact = parts.next()?;
            if group == "org.ow2.asm" && artifact == "asm" {
                parts.next().map(str::to_string)
            } else {
                None
            }
        })
}

/// ASM 9.N 认到 class 主版本 60+N：9.5 → Java 21 (65)，9.9 → Java 25 (69)
fn asm_class_ceiling(asm_version: &str) -> Option<u32> {
    let mut parts = asm_version.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    // 只认识 9.x 这套编号，换编号方式时不做判断，交给运行时
    (major == 9).then_some(60 + minor)
}

/// 基础版本声明的 Java；JSON 读不到时按 MC 版本号推断
fn base_java_version(game_dir: &Path, mc_version: &str) -> Option<u32> {
    let json_path = game_dir
        .join("versions")
        .join(mc_version)
        .join(format!("{}.json", mc_version));

    fs::read_to_string(json_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|json| java_requirement::declared_major(&json))
        .or_else(|| java_requirement::infer_java_for_version(mc_version))
}

/// ASM 读不了这个游戏时返回 (asm 版本, 能读到的 class 版本, 需要的 class 版本)
fn asm_gap(profile: &Value, game_dir: &Path, mc_version: &str) -> Option<(String, u32, u32)> {
    let asm = profile_asm_version(profile)?;
    let ceiling = asm_class_ceiling(&asm)?;
    let java = base_java_version(game_dir, mc_version)?;
    let required = 44 + java;

    (ceiling < required).then_some((asm, ceiling, required))
}

/// profile 里的 Quilt 加载器版本，坐标是 `org.quiltmc:quilt-loader:x.y.z`
fn profile_loader_version(profile: &Value) -> Option<String> {
    profile
        .get("libraries")?
        .as_array()?
        .iter()
        .find_map(|library| {
            let name = library.get("name")?.as_str()?;
            name.strip_prefix("org.quiltmc:quilt-loader:")
                .map(str::to_string)
        })
}

/// 老 Quilt 读不了新游戏的类时就地升到最新版：这种实例必然秒崩，
/// 升级只会从不能用到能用；没坏就完全不发请求、不改文件
pub async fn repair_if_unusable(
    instance_name: &str,
    game_dir: &Path,
    window: &tauri::Window,
) -> Result<Option<String>, LauncherError> {
    let json_path = game_dir
        .join("versions")
        .join(instance_name)
        .join(format!("{}.json", instance_name));

    let Ok(text) = fs::read_to_string(&json_path) else {
        return Ok(None);
    };
    let Ok(profile) = serde_json::from_str::<Value>(&text) else {
        return Ok(None);
    };

    if !profile["libraries"].as_array().is_some_and(|libs| {
        libs.iter().any(|lib| {
            lib["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("org.quiltmc"))
        })
    }) {
        return Ok(None);
    }

    let Some(mc_version) = profile["inheritsFrom"].as_str() else {
        return Ok(None);
    };
    if asm_gap(&profile, game_dir, mc_version).is_none() {
        return Ok(None);
    }

    let old = profile_loader_version(&profile).unwrap_or_else(|| "未知版本".to_string());
    let newest = get_quilt_versions(mc_version)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            LauncherError::Custom(format!("Quilt 没有为 {} 提供任何加载器版本", mc_version))
        })?;

    if newest.version == old {
        return Ok(Some(format!(
            "Quilt {} 已经是 {} 上的最新版，但仍读不了这个游戏的类",
            old, mc_version
        )));
    }

    install_quilt(mc_version, &newest.version, instance_name, game_dir).await?;

    // 新版多出来的库要立刻补齐，否则升级反而变成「文件缺失」
    let upgraded = fs::read_to_string(&json_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    if let Some(json) = upgraded {
        crate::services::download::ensure_declared_libraries(&json, window).await?;
    }

    let message = format!(
        "Quilt {} 自带的 ASM 读不了 Java {} 的类，已自动升级到 {}",
        old,
        base_java_version(game_dir, mc_version).unwrap_or(0),
        newest.version
    );
    warn!("{}", message);
    Ok(Some(message))
}

/// Quilt 用自带的 ASM 读游戏类的元数据，ASM 比游戏新就会在 GameTransformer 里秒崩
async fn check_asm_support(
    profile: &Value,
    mc_version: &str,
    quilt_version: &str,
    game_dir: &Path,
) -> Result<(), LauncherError> {
    let Some((asm, ceiling, required)) = asm_gap(profile, game_dir, mc_version) else {
        return Ok(());
    };
    let java = required - 44;

    let newest = get_quilt_versions(mc_version)
        .await
        .ok()
        .and_then(|list| list.into_iter().next());

    let hint = match newest {
        Some(found) if found.version != quilt_version => {
            format!("换成 {} 或更新版本再装。", found.version)
        }
        Some(_) => {
            "Quilt 目前还没有能读这个 Java 版本的加载器，建议改用 Fabric 或 NeoForge。".to_string()
        }
        None => "请换一个更新的 Quilt 版本再试。".to_string(),
    };

    let message = format!(
        "Quilt {} 自带的 ASM {} 最高只认到 Java {}（class {}），而 {} 的类是 Java {}（class {}）编译的，装完必然秒崩。{}",
        quilt_version,
        asm,
        ceiling - 44,
        ceiling,
        mc_version,
        java,
        required,
        hint
    );
    warn!("{}", message);
    Err(LauncherError::Custom(message))
}

#[cfg(test)]
mod tests {
    use super::{
        asm_class_ceiling, asm_gap, is_stable, profile_asm_version, profile_loader_version,
        version_key,
    };
    use serde_json::json;
    use std::fs;

    #[test]
    fn test_versions_sort_newest_first_and_stable_beats_prerelease() {
        let mut versions = vec![
            "0.20.0-beta.9",
            "0.30.1",
            "0.31.0-beta.4",
            "0.20.0-beta.10",
            "0.26.4",
        ];
        versions.sort_by(|a, b| version_key(b).cmp(&version_key(a)));

        assert_eq!(
            versions,
            vec![
                "0.31.0-beta.4",
                "0.30.1",
                "0.26.4",
                "0.20.0-beta.10",
                "0.20.0-beta.9"
            ]
        );
    }

    #[test]
    fn test_stable_flag() {
        assert!(is_stable("0.30.1"));
        assert!(!is_stable("0.31.0-beta.4"));
    }

    #[test]
    fn test_asm_version_is_read_from_profile_libraries() {
        let profile = json!({
            "libraries": [
                { "name": "net.fabricmc:tiny-mappings-parser:0.3.0" },
                { "name": "org.ow2.asm:asm:9.5" },
                { "name": "org.ow2.asm:asm-analysis:9.5" }
            ]
        });

        assert_eq!(profile_asm_version(&profile).as_deref(), Some("9.5"));
    }

    #[test]
    fn test_asm_ceiling_matches_jdk_support() {
        // 9.5 是 Java 21 时代，读不了 26.3 的 major 69
        assert_eq!(asm_class_ceiling("9.5"), Some(65));
        assert_eq!(asm_class_ceiling("9.9"), Some(69));
        assert_eq!(asm_class_ceiling("9.10.1"), Some(70));
    }

    #[test]
    fn test_unknown_asm_numbering_is_not_judged() {
        assert_eq!(asm_class_ceiling("10.0"), None);
        assert_eq!(asm_class_ceiling("9"), None);
    }

    fn game_dir_with_java(java: u32) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let version_dir = dir.path().join("versions").join("26.3");
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(
            version_dir.join("26.3.json"),
            format!(
                r#"{{"id":"26.3","javaVersion":{{"component":"jre","majorVersion":{}}}}}"#,
                java
            ),
        )
        .unwrap();
        dir
    }

    /// 用户实际遇到的那次：26.3 的类是 major 69，Quilt 0.20.0-beta.9 的 ASM 9.5 只到 65
    #[test]
    fn test_old_quilt_is_flagged_for_java_25_game() {
        let dir = game_dir_with_java(25);

        let old = json!({ "libraries": [
            { "name": "org.ow2.asm:asm:9.5" },
            { "name": "org.quiltmc:quilt-loader:0.20.0-beta.9" }
        ] });
        let new = json!({ "libraries": [
            { "name": "org.ow2.asm:asm:9.10.1" },
            { "name": "org.quiltmc:quilt-loader:0.30.1" }
        ] });

        let gap = asm_gap(&old, dir.path(), "26.3").expect("9.5 必须被判为读不了 class 69");
        assert_eq!(gap, ("9.5".to_string(), 65, 69));
        assert!(asm_gap(&new, dir.path(), "26.3").is_none());
    }

    #[test]
    fn test_old_quilt_is_fine_on_java_8_game() {
        let dir = game_dir_with_java(8);
        let old = json!({ "libraries": [{ "name": "org.ow2.asm:asm:9.5" }] });

        // class 52 远在 ASM 9.5 的能力范围内，不该被误判
        assert!(asm_gap(&old, dir.path(), "26.3").is_none());
    }

    #[test]
    fn test_loader_version_is_read_from_profile() {
        let profile = json!({ "libraries": [
            { "name": "org.ow2.asm:asm:9.5" },
            { "name": "org.quiltmc:quilt-loader:0.20.0-beta.9" }
        ] });

        assert_eq!(
            profile_loader_version(&profile).as_deref(),
            Some("0.20.0-beta.9")
        );
    }
}
