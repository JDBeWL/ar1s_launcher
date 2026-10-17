//! Java 需求判定模块
//!
//! 下限取 Mojang 版本 JSON 声明的 javaVersion，mods 目录实测抬高下限；
//! 只有 LaunchWrapper 时代的老 Forge 另设硬上限（等于声明值），其余加载器不设上限

use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// 声明缺失时的兜底；Mojang 对每个版本都写了 javaVersion，正常不会走到
pub const FALLBACK_JAVA: u32 = 8;

/// 允许向下搜索 inheritsFrom 的层数，实例 → 加载器 → 原版
const INHERITS_DEPTH: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderKind {
    Vanilla,
    Forge,
    /// LaunchWrapper 时代的 Forge（1.12 及更早）：类加载与反射依赖被 Java 9+ 的模块系统切断
    ForgeLegacy,
    NeoForge,
    Fabric,
    Quilt,
}

impl LoaderKind {
    pub fn label(self) -> &'static str {
        match self {
            LoaderKind::Vanilla => "原版",
            LoaderKind::Forge => "Forge",
            LoaderKind::ForgeLegacy => "Forge（LaunchWrapper 时代）",
            LoaderKind::NeoForge => "NeoForge",
            LoaderKind::Fabric => "Fabric",
            LoaderKind::Quilt => "Quilt",
        }
    }

    /// 上限只给旧版 Forge：它的声明值就是实际能用的最高 JDK，不是最低要求。
    /// modlauncher 之后的 Forge（1.13+）能往上走（1.16.5 支持 Java 8 与 11），NeoForge 更没有历史包袱。
    fn caps_at_declared(self) -> bool {
        matches!(self, LoaderKind::ForgeLegacy)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaRequirement {
    /// Mojang 声明的大版本；自动匹配优先命中它
    pub target: u32,
    /// 可能被 mods 的 class 版本抬高
    pub min: u32,
    /// None 表示没有已知的上限
    pub hard_max: Option<u32>,
    pub reasons: Vec<String>,
}

impl JavaRequirement {
    /// min > hard_max 时没有任何 Java 能满足，只能靠报告说明
    pub fn unsatisfiable(&self) -> bool {
        self.hard_max.is_some_and(|max| self.min > max)
    }
}

#[derive(Debug, Clone)]
pub struct ModProbe {
    pub file: String,
    pub java: u32,
}

#[derive(Debug, Clone)]
pub struct InstanceProfile {
    pub requirement: JavaRequirement,
    pub loader: LoaderKind,
    pub mods: Vec<ModProbe>,
    /// 版本 JSON 里真实写了 javaVersion 才有值；兜底成的 8 不算声明
    pub declared_java: Option<u32>,
    /// 版本 JSON 读不到时为 false，此时不做硬拦
    pub resolved: bool,
    pub version_id: String,
    pub game_dir: PathBuf,
}

pub fn class_file_to_java(major: u32) -> Option<u32> {
    // class 48 = Java 4，往后每加 1 对应一个新大版本
    if (48..=90).contains(&major) {
        Some(major - 44)
    } else {
        None
    }
}

pub fn declared_major(json: &Value) -> Option<u32> {
    json.get("javaVersion")?
        .get("majorVersion")?
        .as_u64()
        .map(|v| v as u32)
}

fn has_library(json: &Value, group: &str) -> bool {
    json["libraries"].as_array().is_some_and(|libs| {
        libs.iter().any(|lib| {
            lib["name"]
                .as_str()
                .or_else(|| lib["group"].as_str())
                .is_some_and(|name| name.starts_with(group))
        })
    })
}

/// Forge 还要再分一代：老 Forge 靠 LaunchWrapper 启动，实例 JSON 里会留 `net.minecraft.launchwrapper`
/// —— 要么在 libraries 里，要么直接就是 mainClass。两个信号都认，避免被裁过 libraries 的整合包漏判
fn forge_kind(json: &Value) -> LoaderKind {
    let legacy_by_lib = has_library(json, "net.minecraft.launchwrapper");
    let legacy_by_main = json["mainClass"]
        .as_str()
        .is_some_and(|main| main.starts_with("net.minecraft.launchwrapper"));

    if legacy_by_lib || legacy_by_main {
        LoaderKind::ForgeLegacy
    } else {
        LoaderKind::Forge
    }
}

/// 优先看 libraries（最可靠），退回版本 id 里的加载器字样
pub fn detect_loader(json: &Value, version_id: &str) -> LoaderKind {
    if has_library(json, "net.neoforged") {
        return LoaderKind::NeoForge;
    }
    if has_library(json, "net.minecraftforge") {
        return forge_kind(json);
    }
    if has_library(json, "net.fabricmc") {
        return LoaderKind::Fabric;
    }
    if has_library(json, "org.quiltmc") {
        return LoaderKind::Quilt;
    }

    let id = version_id.to_lowercase();
    if id.contains("neoforge") {
        LoaderKind::NeoForge
    } else if id.contains("forge") {
        LoaderKind::Forge
    } else if id.contains("fabric") {
        LoaderKind::Fabric
    } else if id.contains("quilt") {
        LoaderKind::Quilt
    } else {
        LoaderKind::Vanilla
    }
}

/// 读 jar 里第一个 .class 的主版本号
pub fn jar_java_version(jar: &Path) -> Option<u32> {
    let file = fs::File::open(jar).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        let Ok(name) = entry.name() else { continue };
        if !name.ends_with(".class") {
            continue;
        }

        let mut head = [0u8; 8];
        if entry.read_exact(&mut head).is_err() {
            continue;
        }
        if head[..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
            continue;
        }

        return class_file_to_java(u16::from_be_bytes([head[6], head[7]]) as u32);
    }

    None
}

/// 禁用中的 *.jar.disabled 不算
pub fn scan_mods(mods_dir: &Path) -> Vec<ModProbe> {
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return Vec::new();
    };

    let mut probes = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.ends_with(".jar") || name.ends_with(".jar.disabled") {
            continue;
        }

        if let Some(java) = jar_java_version(&entry.path()) {
            probes.push(ModProbe {
                file: entry.file_name().to_string_lossy().into_owned(),
                java,
            });
        }
    }

    probes.sort_by(|a, b| b.java.cmp(&a.java).then_with(|| a.file.cmp(&b.file)));
    probes
}

pub fn build_requirement(
    declared: Option<u32>,
    loader: LoaderKind,
    mods: &[ModProbe],
) -> JavaRequirement {
    let target = declared.unwrap_or(FALLBACK_JAVA);
    let mut reasons = Vec::new();

    match declared {
        Some(v) => reasons.push(format!("Mojang 声明 javaVersion.majorVersion = {}", v)),
        None => reasons.push(format!(
            "版本 JSON 没有 javaVersion 声明，按 Java {} 兜底",
            FALLBACK_JAVA
        )),
    }

    let hard_max = if loader.caps_at_declared() {
        reasons.push(format!(
            "{} 实例不高于声明的 Java {} 运行：旧类加载与反射依赖被 Java 9+ 的模块系统切断",
            loader.label(),
            target
        ));
        Some(target)
    } else {
        reasons.push(format!("{} 实例不设上限，较新的 JDK 可用", loader.label()));
        None
    };

    let mut min = target;
    if let Some(highest) = mods.first()
        && highest.java > min
    {
        reasons.push(format!(
            "模组 {} 由 Java {} 编译（class 主版本要求），下限抬到 {}",
            highest.file, highest.java, highest.java
        ));
        min = highest.java;
    }

    JavaRequirement {
        target,
        min,
        hard_max,
        reasons,
    }
}

pub fn java_allowed(req: &JavaRequirement, java: u32) -> bool {
    java >= req.min && req.hard_max.is_none_or(|max| java <= max)
}

fn read_version_json(game_dir: &Path, version_id: &str) -> Option<Value> {
    let path = game_dir
        .join("versions")
        .join(version_id)
        .join(format!("{}.json", version_id));
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 沿 inheritsFrom 找第一个带 javaVersion 声明的 JSON
fn resolve_declared(game_dir: &Path, version_id: &str) -> Option<(u32, String)> {
    let mut current = version_id.to_string();

    for _ in 0..=INHERITS_DEPTH {
        let json = read_version_json(game_dir, &current)?;
        if let Some(major) = declared_major(&json) {
            return Some((major, current));
        }

        let parent = json["inheritsFrom"].as_str()?;
        current = parent.to_string();
    }

    None
}

/// 开了版本隔离时 mods 在实例目录里，否则落在游戏根目录
pub fn mods_dir(game_dir: &Path, version_id: &str) -> PathBuf {
    let isolated = game_dir.join("versions").join(version_id).join("mods");
    if isolated.is_dir() {
        isolated
    } else {
        game_dir.join("mods")
    }
}

/// 诊断报告要按 mod id 反指到具体 jar，这里只取名字不做 class 探测
pub fn list_mod_files(mods_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.to_lowercase().ends_with(".jar").then_some(name)
        })
        .collect()
}

pub fn analyze(version_id: &str, game_dir: &Path) -> InstanceProfile {
    let instance_json = read_version_json(game_dir, version_id);
    let loader = instance_json
        .as_ref()
        .map(|json| detect_loader(json, version_id))
        .unwrap_or(LoaderKind::Vanilla);

    let declared = resolve_declared(game_dir, version_id).map(|pair| pair.0);

    let mods = scan_mods(&mods_dir(game_dir, version_id));

    InstanceProfile {
        requirement: build_requirement(declared, loader, &mods),
        loader,
        mods,
        declared_java: declared,
        resolved: instance_json.is_some(),
        version_id: version_id.to_string(),
        game_dir: game_dir.to_path_buf(),
    }
}

/// 给详情对话框看的纯文本报告；同时会被写进日志
pub fn build_report(
    profile: &InstanceProfile,
    current: Option<(&str, u32)>,
    candidates: &[(String, u32)],
) -> String {
    let req = &profile.requirement;
    let mut lines: Vec<String> = Vec::new();

    lines.push(format!("实例：{}", profile.version_id));
    lines.push(format!("加载器：{}", profile.loader.label()));
    lines.push(format!(
        "Java 要求：{}（优先 {}）{}",
        req.min,
        req.target,
        match req.hard_max {
            Some(max) => format!("，不得高于 {}", max),
            None => "，无上限".to_string(),
        }
    ));

    if let Some((path, ver)) = current {
        lines.push(format!("当前选择：Java {} · {}", ver, path));
    } else {
        lines.push("当前选择：未确定".to_string());
    }

    lines.push(String::new());
    lines.push("判定依据：".to_string());
    for reason in &req.reasons {
        lines.push(format!("  · {}", reason));
    }

    if !profile.mods.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "模组 class 版本（前 {} 项）：",
            profile.mods.len().min(8)
        ));
        for probe in profile.mods.iter().take(8) {
            lines.push(format!("  · {} → Java {}", probe.file, probe.java));
        }
    }

    if req.unsatisfiable() {
        lines.push(String::new());
        lines.push(format!(
            "无法同时满足：{} 上限是 Java {}，但模组要求 Java {}。请把模组换成匹配版本，或用对应版本重建实例。",
            profile.loader.label(),
            req.hard_max.unwrap_or(0),
            req.min
        ));
    }

    lines.push(String::new());
    if candidates.is_empty() {
        lines.push("本机可用 Java：一个都没扫到".to_string());
    } else {
        lines.push("本机可用 Java：".to_string());
        for (path, ver) in candidates {
            let mark = if java_allowed(req, *ver) {
                "可用"
            } else {
                "不行"
            };
            lines.push(format!("  · Java {} [{}] {}", ver, mark, path));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_mod_files_keeps_only_enabled_jars() {
        let dir = tempfile::TempDir::new().unwrap();
        for name in ["a.jar", "b.jar.disabled", "notes.txt", "C.JAR"] {
            fs::write(dir.path().join(name), b"x").unwrap();
        }

        let mut files = list_mod_files(dir.path());
        files.sort();
        assert_eq!(files, ["C.JAR", "a.jar"]);
    }

    #[test]
    fn test_mods_dir_prefers_isolated_then_shared() {
        let dir = tempfile::TempDir::new().unwrap();
        let shared = dir.path().join("mods");
        let isolated = dir.path().join("versions").join("pack").join("mods");
        fs::create_dir_all(&shared).unwrap();

        assert_eq!(mods_dir(dir.path(), "pack"), shared);
        fs::create_dir_all(&isolated).unwrap();
        assert_eq!(mods_dir(dir.path(), "pack"), isolated);
    }

    fn vanilla_json(mc: &str, java: u64) -> Value {
        serde_json::json!({ "id": mc, "javaVersion": { "majorVersion": java } })
    }

    /// 1.12 及更早那代 Forge：libraries 里留着 LaunchWrapper（实测 1.8.9 installer 的 install_profile.json）
    fn legacy_forge_json(mc: &str) -> Value {
        serde_json::json!({
            "id": format!("{}-forge-14.23.5.2860", mc),
            "inheritsFrom": mc,
            "libraries": [
                { "name": "net.minecraftforge:forge:1.12.2-14.23.5.2860" },
                { "name": "net.minecraft.launchwrapper:launchwrapper:1.12" }
            ]
        })
    }

    /// 1.13 之后 Forge 改用 modlauncher（实测 1.16.5 installer 的 version.json）
    fn modern_forge_json(mc: &str) -> Value {
        serde_json::json!({
            "id": format!("{}-forge-36.2.39", mc),
            "inheritsFrom": mc,
            "libraries": [
                { "name": "net.minecraftforge:forge:1.16.5-36.2.39" },
                { "name": "cpw.mods:modlauncher:8.1.3" }
            ]
        })
    }

    #[test]
    fn test_class_file_major_maps_to_java() {
        assert_eq!(class_file_to_java(52), Some(8));
        assert_eq!(class_file_to_java(55), Some(11));
        assert_eq!(class_file_to_java(61), Some(17));
        assert_eq!(class_file_to_java(65), Some(21));
        assert_eq!(class_file_to_java(45), None);
    }

    #[test]
    fn test_vanilla_declares_no_upper_bound() {
        let req = build_requirement(Some(8), LoaderKind::Vanilla, &[]);
        assert_eq!((req.min, req.target), (8, 8));
        assert_eq!(req.hard_max, None);
        assert!(java_allowed(&req, 8));
        // 这就是他提的那条：原版 1.12.2 没用到旧 Forge 的那些依赖，新 JDK 能起
        assert!(java_allowed(&req, 17));
        assert!(java_allowed(&req, 26));
    }

    #[test]
    fn test_legacy_forge_caps_and_modern_forge_does_not() {
        let legacy = legacy_forge_json("1.12.2");
        assert_eq!(
            detect_loader(&legacy, "1.12.2-forge-14.23.5.2860"),
            LoaderKind::ForgeLegacy
        );
        let legacy_req = build_requirement(Some(8), LoaderKind::ForgeLegacy, &[]);
        assert_eq!(legacy_req.hard_max, Some(8));
        assert!(java_allowed(&legacy_req, 8));
        // 搜索结果里 1.12.2 Forge 的兼容范围就是 Java 7-8，11 一样起不来
        assert!(!java_allowed(&legacy_req, 11));
        assert!(!java_allowed(&legacy_req, 17));

        // modlauncher 时代的 Forge 能往上走：1.16.5 官方支持 Java 8 与 11
        let modern = modern_forge_json("1.16.5");
        assert_eq!(
            detect_loader(&modern, "1.16.5-forge-36.2.39"),
            LoaderKind::Forge
        );
        let modern_req = build_requirement(Some(8), LoaderKind::Forge, &[]);
        assert_eq!(modern_req.hard_max, None);
        assert!(java_allowed(&modern_req, 8));
        assert!(java_allowed(&modern_req, 11));

        // NeoForge 的声明是下限，较新的 JDK 不该被拦，但低于声明仍然不行
        let neoforge = build_requirement(Some(21), LoaderKind::NeoForge, &[]);
        assert_eq!(neoforge.hard_max, None);
        assert!(java_allowed(&neoforge, 25));
        assert!(!java_allowed(&neoforge, 17));
    }

    #[test]
    fn test_legacy_forge_is_also_detected_from_main_class() {
        // libraries 被裁过的整合包：mainClass 本身就是 LaunchWrapper 的入口
        let json = serde_json::json!({
            "id": "1.8-forge-11.14.4.1577",
            "mainClass": "net.minecraft.launchwrapper.Launch",
            "libraries": [{ "name": "net.minecraftforge:forge:1.8-11.14.4.1577" }]
        });
        assert_eq!(
            detect_loader(&json, "1.8-forge-11.14.4.1577"),
            LoaderKind::ForgeLegacy
        );
    }

    #[test]
    fn test_mod_class_version_raises_the_floor() {
        let mods = vec![
            ModProbe {
                file: "newmod-1.0.jar".to_string(),
                java: 21,
            },
            ModProbe {
                file: "oldmod-1.0.jar".to_string(),
                java: 8,
            },
        ];
        let req = build_requirement(Some(8), LoaderKind::Vanilla, &mods);

        assert_eq!(req.min, 21);
        assert_eq!(req.target, 8);
        assert!(!java_allowed(&req, 17));
        assert!(java_allowed(&req, 21));
        assert!(req.reasons.iter().any(|r| r.contains("newmod-1.0.jar")));
    }

    #[test]
    fn test_forge_with_newer_mod_is_unsatisfiable() {
        let mods = vec![ModProbe {
            file: "oops.jar".to_string(),
            java: 21,
        }];
        let req = build_requirement(Some(8), LoaderKind::ForgeLegacy, &mods);

        assert!(req.unsatisfiable());
        assert!(!java_allowed(&req, 8));
        assert!(!java_allowed(&req, 21));

        let profile = InstanceProfile {
            requirement: req,
            loader: LoaderKind::ForgeLegacy,
            mods,
            declared_java: Some(8),
            resolved: true,
            version_id: "1.8.9-forge-14.23.5.2860".to_string(),
            game_dir: PathBuf::from("/tmp"),
        };
        let report = build_report(
            &profile,
            Some(("C:/jdk-26/bin/java.exe", 26)),
            &[("C:/jdk-26/bin/java.exe".to_string(), 26u32)],
        );
        assert!(report.contains("无法同时满足"), "{}", report);
        assert!(report.contains("Java 26 [不行]"), "{}", report);
    }

    #[test]
    fn test_missing_declaration_falls_back_without_crashing() {
        let req = build_requirement(None, LoaderKind::Vanilla, &[]);
        assert_eq!(req.min, FALLBACK_JAVA);
        assert!(req.reasons.iter().any(|r| r.contains("兜底")));
    }

    #[test]
    fn test_fabric_and_quilt_are_detected_and_uncapped() {
        let fabric =
            serde_json::json!({ "libraries": [{ "name": "net.fabricmc:fabric-loader:0.15.0" }] });
        let quilt =
            serde_json::json!({ "libraries": [{ "name": "org.quiltmc:quilt-loader:0.23.1" }] });
        assert_eq!(detect_loader(&fabric, ""), LoaderKind::Fabric);
        assert_eq!(detect_loader(&quilt, ""), LoaderKind::Quilt);
        assert_eq!(
            build_requirement(Some(17), LoaderKind::Fabric, &[]).hard_max,
            None
        );
    }

    #[test]
    fn test_report_lists_mods_and_candidates() {
        let json = vanilla_json("1.21.4", 21);
        assert_eq!(declared_major(&json), Some(21));

        let req = build_requirement(Some(21), LoaderKind::Vanilla, &[]);
        let profile = InstanceProfile {
            requirement: req,
            loader: LoaderKind::Vanilla,
            mods: vec![ModProbe {
                file: "a.jar".to_string(),
                java: 21,
            }],
            declared_java: Some(21),
            resolved: true,
            version_id: "1.21.4".to_string(),
            game_dir: PathBuf::from("/tmp"),
        };
        let candidates = vec![
            ("C:/java-17/bin/java.exe".to_string(), 17u32),
            ("C:/java-21/bin/java.exe".to_string(), 21u32),
        ];
        let report = build_report(&profile, Some(("C:/java-17/bin/java.exe", 17)), &candidates);

        assert!(report.contains("a.jar → Java 21"));
        assert!(report.contains("Java 17 [不行]"));
        assert!(report.contains("Java 21 [可用]"));
    }

    #[test]
    fn test_disabled_mods_are_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let mods = tmp.path().to_path_buf();
        write_jar(&mods.join("live.jar"), 65);
        write_jar(&mods.join("dead.jar.disabled"), 65);

        let probes = scan_mods(&mods);
        assert_eq!(probes.len(), 1, "{:?}", probes);
        assert_eq!(probes[0].file, "live.jar");
    }

    fn write_jar(path: &Path, class_major: u16) {
        use zip::write::SimpleFileOptions;
        let file = fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("com/example/A.class", SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(
            &mut zip,
            &[
                0xCA,
                0xFE,
                0xBA,
                0xBE,
                0,
                0,
                (class_major >> 8) as u8,
                class_major as u8,
            ],
        )
        .unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn test_jar_java_version_reads_first_class() {
        let tmp = tempfile::tempdir().unwrap();
        let jar = tmp.path().join("x.jar");
        write_jar(&jar, 61);
        assert_eq!(jar_java_version(&jar), Some(17));
    }

    #[test]
    fn test_analyze_walks_inherits_from_for_declaration() {
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().to_path_buf();
        let versions = game.join("versions");

        // 原版 JSON 带声明，Forge 实例 JSON 不带
        let parent_dir = versions.join("1.8.9");
        fs::create_dir_all(&parent_dir).unwrap();
        fs::write(
            parent_dir.join("1.8.9.json"),
            serde_json::to_string(&vanilla_json("1.8.9", 8)).unwrap(),
        )
        .unwrap();

        let child_id = "1.8.9-forge-11.15.1.2318";
        let child_dir = versions.join(child_id);
        fs::create_dir_all(&child_dir).unwrap();
        fs::write(
            child_dir.join(format!("{}.json", child_id)),
            serde_json::to_string(&legacy_forge_json("1.8.9")).unwrap(),
        )
        .unwrap();

        let profile = analyze(child_id, &game);
        assert_eq!(profile.loader, LoaderKind::ForgeLegacy);
        assert!(profile.resolved);
        assert_eq!(profile.requirement.target, 8);
        assert_eq!(profile.requirement.hard_max, Some(8));
    }
}
