//! 整合包导出
//!
//! 把一个实例打包成 Modrinth 格式的 .mrpack：识别得出的模组按 Modrinth 版本写进
//! `modrinth.index.json` 的 files（安装时走官方 CDN），其余内容（config、资源包、光影、
//! 存档、识别不出出处的 jar）原样放进 overrides。这样导出的包既能被别的启动器装，
//! 也能在本启动器里通过「从文件导入」再装回来。

use crate::errors::LauncherError;
use crate::models::modpack::{ModpackExportOptions, ModpackExportResult, ModrinthModpackVersion};
use crate::services::launcher::isolation;
use crate::services::modpack_format::{
    self, INDEX_FILE, ModrinthDependencies, ModrinthFileEnv, ModrinthIndex, ModrinthIndexFile,
    ModrinthIndexHashes, OVERRIDES_DIR,
};
use crate::services::{config, instance, modrinth};
use crate::utils::file_utils;
use log::{info, warn};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// 只进 overrides 的内容目录（相对实例内容根）
const OVERRIDE_DIRS: &[&str] = &[
    "config",
    "defaultconfigs",
    "kubejs",
    "scripts",
    "resourcepacks",
    "shaderpacks",
    "datapacks",
    "saves",
    "local",
    "structures",
    "openloader",
];

/// 明显是运行期产物，打包没有意义还会让包变得巨大
const SKIP_DIRS: &[&str] = &[
    "logs",
    "crash-reports",
    "temp",
    ".ar1s",
    ".cache",
    ".download_state",
    "screenshots",
    "backups",
];

/// 这些文件每次都重写，保留会把上一个玩家的状态带给别人
const SKIP_FILES: &[&str] = &[
    "options.txt",
    "optionsof.txt",
    "optionsshaders.txt",
    "servers.dat",
    "servers.dat_old",
    "usercache.json",
    "usernamecache.json",
    "realms_persistence.json",
    "instance.json",
    "debug-profile.json",
];

/// 单次导出的体积上限，超过就拒绝而不是把磁盘写满
const MAX_OVERRIDES_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// 一个待打包的文件：要么是 Modrinth 上的（写链接），要么是本地独有的（写 overrides）
struct PlannedFile {
    /// index 里的相对路径，形如 mods/xxx.jar
    relative: String,
    source: PathBuf,
    size: u64,
    /// Some 表示能在 Modrinth 反查到，走链接；None 表示只能塞进 overrides
    linked: Option<LinkedFile>,
}

struct LinkedFile {
    sha1: String,
    sha512: Option<String>,
    /// 直接取 Modrinth 版本里给的地址，不自己拼 CDN 路径
    urls: Vec<String>,
    env: Option<ModrinthFileEnv>,
}

pub async fn export_modpack(
    instance_name: String,
    options: ModpackExportOptions,
) -> Result<ModpackExportResult, LauncherError> {
    let name = options.name.trim().to_string();
    let version = options.version.trim().to_string();
    if name.is_empty() {
        return Err(LauncherError::Custom("整合包名称不能为空".to_string()));
    }
    if version.is_empty() {
        return Err(LauncherError::Custom("整合包版本号不能为空".to_string()));
    }

    let config = config::load_config()?;
    let game_dir = PathBuf::from(&config.game_dir);
    let content_root = isolation::content_root(&config, &game_dir, &instance_name);
    let instance_dir = file_utils::instance_dir(&game_dir.join("versions"), &instance_name)?;
    if !instance_dir.is_dir() {
        return Err(LauncherError::Custom(format!(
            "实例 '{}' 不存在",
            instance_name
        )));
    }

    let target = validate_target(&options.target_path)?;

    // 实例的 MC 版本与加载器来自 instance.json / 版本 JSON，和实例列表用的是同一套推断
    let info = instance::get_instances()
        .await?
        .into_iter()
        .find(|info| info.name == instance_name)
        .ok_or_else(|| LauncherError::Custom(format!("实例 '{}' 不存在", instance_name)))?;

    let minecraft = info
        .game_version
        .filter(|value| !value.is_empty())
        .ok_or_else(|| LauncherError::Custom("无法确定实例的 Minecraft 版本".to_string()))?;
    let loader_key = match info.loader_type.as_deref() {
        Some("Forge") => Some(("forge", "Forge")),
        Some("Fabric") => Some(("fabric", "Fabric")),
        Some("Quilt") => Some(("quilt", "Quilt")),
        Some("NeoForge") => Some(("neoforge", "NeoForge")),
        _ => None,
    };

    let loader_version = match (loader_key, info.loader_version.clone()) {
        (Some((_, _)), Some(version)) if !version.trim().is_empty() => Some(version),
        (Some((_, label)), _) => {
            return Err(LauncherError::Custom(format!(
                "实例没有记录 {} 的版本号，无法导出为整合包",
                label
            )));
        }
        (None, _) => None,
    };

    // 没有加载器却带着 jar：包装出来是一个跑不起来的原版实例，不如现在就拒绝
    if loader_key.is_none() && has_enabled_mods(&content_root) {
        return Err(LauncherError::Custom(
            "实例里装了模组但识别不出加载器，无法导出为整合包".to_string(),
        ));
    }
    let linked_versions = resolve_modrinth_versions(&content_root, options.include_saves).await?;
    let sides = resolve_project_sides(&linked_versions).await?;

    let mut seen_paths = HashSet::new();
    let mut plan = Vec::new();
    collect_planned_files(
        &content_root,
        &content_root,
        options.include_saves,
        &linked_versions,
        &sides,
        &mut seen_paths,
        &mut plan,
    )?;

    let total_bytes = plan.iter().map(|file| file.size).sum::<u64>();
    if total_bytes > MAX_OVERRIDES_BYTES {
        return Err(LauncherError::Custom(format!(
            "要打包的内容超过 {} 字节上限，请先清理实例或去掉存档",
            MAX_OVERRIDES_BYTES
        )));
    }

    let dependencies =
        build_dependencies(&minecraft, loader_key.map(|(key, _)| key), loader_version);
    let index_files: Vec<ModrinthIndexFile> = plan
        .iter()
        .filter_map(|file| {
            let linked = file.linked.as_ref()?;
            Some(ModrinthIndexFile {
                path: file.relative.clone(),
                hashes: ModrinthIndexHashes {
                    sha1: linked.sha1.clone(),
                    sha512: linked.sha512.clone(),
                },
                downloads: linked.urls.clone(),
                file_size: Some(file.size),
                env: linked.env.clone(),
            })
        })
        .collect();

    let version_id = format!("{}-{}", sanitize_version_id(&name), version);
    let index = ModrinthIndex {
        format_version: modpack_format::SUPPORTED_FORMAT_VERSION,
        game: "minecraft".to_string(),
        version_id: version_id.clone(),
        name: name.clone(),
        summary: options
            .summary
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty()),
        files: index_files.clone(),
        dependencies,
    };

    let override_files: Vec<&PlannedFile> =
        plan.iter().filter(|file| file.linked.is_none()).collect();
    let linked_count = index_files.len();
    let override_count = override_files.len();

    let target_clone = target.clone();
    let name_clone = name.clone();
    let version_clone = version_id.clone();
    let index_text = serde_json::to_string_pretty(&index)?;
    let overrides: Vec<(String, PathBuf)> = override_files
        .iter()
        .map(|file| (file.relative.clone(), file.source.clone()))
        .collect();

    tokio::task::spawn_blocking(move || -> Result<(), LauncherError> {
        write_mrpack(
            &target_clone,
            &name_clone,
            &version_clone,
            &index_text,
            &overrides,
        )
    })
    .await
    .map_err(|e| LauncherError::Custom(format!("写入整合包失败: {}", e)))??;

    info!(
        "实例 {} 已导出为 {}（{} 个链接文件，{} 个 override 文件）",
        instance_name,
        target.display(),
        linked_count,
        override_count
    );

    Ok(ModpackExportResult {
        path: target.to_string_lossy().to_string(),
        name,
        version_id,
        linked_files: linked_count,
        override_files: override_count,
        total_bytes,
    })
}

/// 实例内容目录下所有 Modrinth 认得的文件：sha1 -> 版本
async fn resolve_modrinth_versions(
    content_root: &Path,
    include_saves: bool,
) -> Result<HashMap<String, ModrinthModpackVersion>, LauncherError> {
    let roots = scan_roots(content_root, include_saves);
    let mut candidates = Vec::new();

    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !is_archivable_file(&path) {
                continue;
            }
            candidates.push(path);
        }
    }

    if candidates.is_empty() {
        return Ok(HashMap::new());
    }

    let hashes = tokio::task::spawn_blocking(move || {
        let mut hashes = Vec::new();
        for path in candidates {
            match file_utils::sha1_file(&path) {
                Ok(hash) => hashes.push(hash),
                Err(error) => warn!("计算 {} 的 sha1 失败: {}", path.display(), error),
            }
        }
        hashes
    })
    .await
    .map_err(|e| LauncherError::Custom(format!("计算文件哈希失败: {}", e)))?;

    match modrinth::ModrinthService::new()
        .get_versions_by_hashes(hashes)
        .await
    {
        Ok(versions) => Ok(versions),
        // 查不到就整包走 overrides，功能上仍然能导入，只是包会大一些
        Err(error) => {
            warn!(
                "查询 Modrinth 版本失败，本次全部按 overrides 打包: {}",
                error
            );
            Ok(HashMap::new())
        }
    }
}

/// 每个项目只查一次端侧支持情况
async fn resolve_project_sides(
    versions: &HashMap<String, ModrinthModpackVersion>,
) -> Result<HashMap<String, modrinth::ProjectSides>, LauncherError> {
    let project_ids: Vec<String> = versions
        .values()
        .map(|version| version.project_id.clone())
        .filter(|id| !id.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    match modrinth::ModrinthService::new()
        .get_project_sides(&project_ids)
        .await
    {
        Ok(sides) => Ok(sides),
        Err(error) => {
            warn!("查询项目端侧信息失败，env 字段将省略: {}", error);
            Ok(HashMap::new())
        }
    }
}

/// mods 目录里还有启用的 jar，说明这个实例不是纯原版
fn has_enabled_mods(content_root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(content_root.join("mods")) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .to_ascii_lowercase()
            .ends_with(".jar")
    })
}

fn scan_roots(content_root: &Path, include_saves: bool) -> Vec<PathBuf> {
    let mut roots = vec![content_root.join("mods")];
    if include_saves {
        roots.push(content_root.join("saves"));
    }
    roots
}

/// 只有 jar/zip 这类 Modrinth 会收录的文件才值得去反查
fn is_archivable_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| {
            let ext = ext.to_string_lossy().to_ascii_lowercase();
            ext == "jar" || ext == "zip"
        })
        .unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn collect_planned_files(
    root: &Path,
    current: &Path,
    include_saves: bool,
    linked_versions: &HashMap<String, ModrinthModpackVersion>,
    sides: &HashMap<String, modrinth::ProjectSides>,
    seen_paths: &mut HashSet<String>,
    plan: &mut Vec<PlannedFile>,
) -> Result<(), LauncherError> {
    let Ok(entries) = fs::read_dir(current) else {
        return Ok(());
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let at_root = current == root;

        if path.is_dir() {
            let lower = name.to_ascii_lowercase();
            if SKIP_DIRS.contains(&lower.as_str()) {
                continue;
            }
            if lower == "saves" && !include_saves {
                continue;
            }
            // 白名单只在顶层生效：进了 config/ 之后里面的子目录都要原样保留
            if at_root {
                let allowed = OVERRIDE_DIRS.contains(&lower.as_str()) || lower == "mods";
                if !allowed {
                    continue;
                }
            }
            collect_planned_files(
                root,
                &path,
                include_saves,
                linked_versions,
                sides,
                seen_paths,
                plan,
            )?;
            continue;
        }

        if !path.is_file() || SKIP_FILES.contains(&name.as_str()) {
            continue;
        }

        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        if !file_utils::is_safe_relative_path(&relative) {
            warn!("跳过路径不合法的文件: {}", relative);
            continue;
        }
        if !seen_paths.insert(relative.clone()) {
            continue;
        }

        let size = path.metadata().map(|meta| meta.len()).unwrap_or(0);
        let linked = if is_archivable_file(&path) {
            match file_utils::sha1_file(&path) {
                Ok(hash) => linked_versions
                    .get(&hash.to_ascii_lowercase())
                    .and_then(|version| build_linked_file(version, &hash, sides)),
                Err(error) => {
                    warn!("计算 {} 的 sha1 失败: {}", relative, error);
                    None
                }
            }
        } else {
            None
        };

        plan.push(PlannedFile {
            relative,
            source: path,
            size,
            linked,
        });
    }

    Ok(())
}

fn build_linked_file(
    version: &ModrinthModpackVersion,
    sha1: &str,
    sides: &HashMap<String, modrinth::ProjectSides>,
) -> Option<LinkedFile> {
    let file = version
        .files
        .iter()
        .find(|file| file.hashes.sha1.eq_ignore_ascii_case(sha1))
        .or_else(|| version.files.iter().find(|file| file.primary))?;

    // 没有下载地址就没法走链接，只能塞进 overrides，否则装的人会拿到一个空 URL
    if file.url.trim().is_empty() {
        return None;
    }

    let env = sides
        .get(&version.project_id)
        .filter(|project| project.client.is_some() || project.server.is_some())
        .map(|project| ModrinthFileEnv {
            client: project.client.clone(),
            server: project.server.clone(),
        });

    Some(LinkedFile {
        sha1: file.hashes.sha1.clone(),
        // index 允许 sha512 缺省，拿不到就不写，免得写了错的值让别人的安装一路失败
        sha512: Some(file.hashes.sha512.clone()).filter(|hash| !hash.trim().is_empty()),
        urls: vec![file.url.clone()],
        env,
    })
}

fn build_dependencies(
    minecraft: &str,
    loader_key: Option<&str>,
    loader_version: Option<String>,
) -> ModrinthDependencies {
    let mut dependencies = ModrinthDependencies {
        minecraft: minecraft.to_string(),
        ..ModrinthDependencies::default()
    };

    match (loader_key, loader_version) {
        (Some("forge"), Some(version)) => dependencies.forge = Some(version),
        (Some("fabric"), Some(version)) => dependencies.fabric_loader = Some(version),
        (Some("quilt"), Some(version)) => dependencies.quilt_loader = Some(version),
        (Some("neoforge"), Some(version)) => dependencies.neoforge = Some(version),
        _ => {}
    }

    dependencies
}

fn validate_target(raw: &str) -> Result<PathBuf, LauncherError> {
    let path = PathBuf::from(raw.trim());
    if raw.trim().is_empty() {
        return Err(LauncherError::Custom("整合包保存路径不能为空".to_string()));
    }
    if !path
        .extension()
        .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("mrpack"))
    {
        return Err(LauncherError::Custom(
            "整合包必须保存为 .mrpack 文件".to_string(),
        ));
    }
    if path.is_dir() {
        return Err(LauncherError::Custom("整合包路径是一个目录".to_string()));
    }
    match path.parent() {
        Some(parent) if parent.is_dir() => Ok(path),
        _ => Err(LauncherError::Custom("整合包保存目录不存在".to_string())),
    }
}

/// index 的 versionId 只用来展示，把名字里的杂字符换掉即可
fn sanitize_version_id(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned
        .trim_matches(|c: char| c == '-' || c == '.')
        .to_string();
    if trimmed.is_empty() {
        "modpack".to_string()
    } else {
        trimmed
    }
}

fn write_mrpack(
    target: &Path,
    name: &str,
    version_id: &str,
    index_text: &str,
    overrides: &[(String, PathBuf)],
) -> Result<(), LauncherError> {
    let temp = target.with_extension("mrpack.part");
    let result = (|| -> Result<(), LauncherError> {
        let file = File::create(&temp)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file(INDEX_FILE, options)?;
        zip.write_all(index_text.as_bytes())?;

        // 规范要求的额外说明文件，别的启动器会读它做展示
        zip.start_file("README.md", options)?;
        zip.write_all(
            format!("# {name}\n\nModrinth 整合包，由 Ar1s Launcher 导出。\n\n版本: {version_id}\n")
                .as_bytes(),
        )?;

        for (relative, source) in overrides {
            let entry = format!("{}/{}", OVERRIDES_DIR, relative);
            zip.start_file(entry, options)?;
            let bytes = fs::read(source)?;
            zip.write_all(&bytes)?;
        }

        let file = zip.finish()?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }

    // Windows 的 rename 不覆盖已存在文件；保存对话框已经确认过覆盖，这里先删旧包
    if target.exists()
        && let Err(error) = fs::remove_file(target)
    {
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&temp, target) {
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_dependencies, collect_planned_files, sanitize_version_id, validate_target,
        write_mrpack,
    };
    use crate::services::modpack_format;
    use crate::services::modrinth::ProjectSides;
    use std::collections::{HashMap, HashSet};
    use std::fs;

    /// 导出的包必须能被安装路径原样读回来，否则「导出 → 导入」是死路
    #[test]
    fn test_exported_mrpack_round_trips_through_the_installer_reader() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("pack.mrpack");
        let source = dir.path().join("sodium.toml");
        fs::write(&source, b"options").unwrap();

        let index = serde_json::json!({
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "My-Pack-1.0.0",
            "name": "My Pack",
            "summary": "测试包",
            "files": [{
                "path": "mods/sodium.jar",
                "hashes": { "sha1": "abc" },
                "downloads": ["https://cdn.modrinth.com/data/x/versions/y/sodium.jar"],
                "fileSize": 42
            }],
            "dependencies": { "minecraft": "1.20.1", "fabric-loader": "0.15.0" }
        });
        let index_text = serde_json::to_string_pretty(&index).unwrap();

        write_mrpack(
            &target,
            "My Pack",
            "My-Pack-1.0.0",
            &index_text,
            &[("config/sodium.toml".to_string(), source)],
        )
        .unwrap();

        let parsed = modpack_format::read_index_from_archive(&target).unwrap();
        assert_eq!(parsed.name, "My Pack");
        assert_eq!(parsed.dependencies.minecraft, "1.20.1");
        assert_eq!(parsed.dependencies.loader_slug(), Some("fabric"));
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].path, "mods/sodium.jar");

        // overrides 里的文件要真的进了包，安装时才能铺到实例里
        let extract = dir.path().join("extract");
        modpack_format::extract_archive(&target, &extract, 1024 * 1024).unwrap();
        assert!(extract.join("overrides/config/sodium.toml").is_file());
    }

    #[test]
    fn test_sanitize_version_id_replaces_unsafe_chars() {
        assert_eq!(sanitize_version_id("My Pack"), "My-Pack");
        assert_eq!(sanitize_version_id(".."), "modpack");
        assert_eq!(sanitize_version_id("整合包"), "modpack");
        assert_eq!(sanitize_version_id("pack-1.0"), "pack-1.0");
    }

    #[test]
    fn test_build_dependencies_only_sets_one_loader() {
        let deps = build_dependencies("1.20.1", Some("fabric"), Some("0.15.0".to_string()));
        assert_eq!(deps.minecraft, "1.20.1");
        assert_eq!(deps.fabric_loader.as_deref(), Some("0.15.0"));
        assert!(deps.forge.is_none());

        let vanilla = build_dependencies("1.20.1", None, None);
        assert!(vanilla.loader_slug().is_none());
    }

    #[test]
    fn test_validate_target_requires_mrpack_extension() {
        let dir = tempfile::TempDir::new().unwrap();
        let ok = dir.path().join("pack.mrpack");
        assert!(validate_target(&ok.to_string_lossy()).is_ok());

        let bad = dir.path().join("pack.zip");
        assert!(validate_target(&bad.to_string_lossy()).is_err());

        assert!(validate_target("").is_err());
    }

    #[test]
    fn test_collect_skips_runtime_artifacts_and_private_state() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("mods")).unwrap();
        fs::create_dir_all(root.join("logs")).unwrap();
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("saves")).unwrap();

        fs::write(root.join("mods/sodium.jar"), b"jar").unwrap();
        fs::write(root.join("mods/unknown.jar"), b"unknown").unwrap();
        fs::write(root.join("logs/latest.log"), b"log").unwrap();
        fs::write(root.join("config/sodium.toml"), b"cfg").unwrap();
        fs::create_dir_all(root.join("saves/world")).unwrap();
        fs::write(root.join("saves/world/level.dat"), b"world").unwrap();
        fs::write(root.join("options.txt"), b"options").unwrap();
        fs::write(root.join("servers.dat"), b"servers").unwrap();

        let mut seen = HashSet::new();
        let mut plan = Vec::new();
        collect_planned_files(
            root,
            root,
            false,
            &HashMap::new(),
            &HashMap::<String, ProjectSides>::new(),
            &mut seen,
            &mut plan,
        )
        .unwrap();

        let mut paths: Vec<&str> = plan.iter().map(|file| file.relative.as_str()).collect();
        paths.sort();

        assert_eq!(
            paths,
            vec!["config/sodium.toml", "mods/sodium.jar", "mods/unknown.jar"]
        );
        // 没有 Modrinth 反查结果时一切都进 overrides
        assert!(plan.iter().all(|file| file.linked.is_none()));
    }

    #[test]
    fn test_collect_includes_saves_only_when_requested() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("saves/world")).unwrap();
        fs::write(root.join("saves/world/level.dat"), b"world").unwrap();

        let mut seen = HashSet::new();
        let mut plan = Vec::new();
        collect_planned_files(
            root,
            root,
            true,
            &HashMap::new(),
            &HashMap::<String, ProjectSides>::new(),
            &mut seen,
            &mut plan,
        )
        .unwrap();

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].relative, "saves/world/level.dat");
    }
}
