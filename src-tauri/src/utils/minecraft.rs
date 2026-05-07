//! Minecraft 通用工具函数
//!
//! 提供多个模块共享的 Minecraft 相关工具函数，避免代码重复。

/// 判断 Minecraft 版本是否为"新版本"（1.17+，含 26.x.x 新格式）
///
/// 使用数值解析而非字符串前缀匹配，避免 `1.21.1` 等版本误判。
/// 支持带 Forge/Fabric 后缀的版本号（如 `1.20.2-forge-48.0.48`），
/// 会先提取基础 MC 版本再进行比较。
/// 同时支持 2025 年起的新格式 `26.x.x`（主版本号 > 1 一律视为新版本）。
pub fn is_new_version(version: &str) -> bool {
    parse_mc_version(version).map_or(false, |(major, minor)| {
        major > 1 || (major == 1 && minor >= 17)
    })
}

/// 判断 Minecraft 版本是否为"中等版本"（1.12 ~ 1.16.x）
pub fn is_mid_version(version: &str) -> bool {
    parse_mc_version(version).map_or(false, |(major, minor)| {
        major == 1 && (12..=16).contains(&minor)
    })
}

/// 从版本字符串中解析主版本号和次版本号
///
/// 例如：`"1.20.4"` → `Some((1, 20))`，`"1.16.5-forge-36.2.39"` → `Some((1, 16))`，
/// `"26.1.0"` → `Some((26, 1))`
pub fn parse_mc_version(version: &str) -> Option<(u32, u32)> {
    let base = version.split('-').next()?;
    let parts: Vec<&str> = base.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let major: u32 = parts[0].parse().ok()?;
    let minor: u32 = parts[1].parse().ok()?;
    Some((major, minor))
}

/// 将 Maven 坐标转换为文件路径
///
/// 例如: `"net.minecraftforge:forge:1.20.2-48.0.48"` → `"net/minecraftforge/forge/1.20.2-48.0.48/forge-1.20.2-48.0.48.jar"`
///
/// 支持可选的 classifier 和 extension 参数：
/// - `classifier`: Maven 坐标中的第四部分（如 `natives-windows`）
/// - `extension`: 文件扩展名，默认为 `"jar"`
pub fn maven_name_to_path(name: &str, classifier: Option<&str>, extension: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }

    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let resolved_classifier = classifier.or_else(|| parts.get(3).copied());

    let filename = if let Some(c) = resolved_classifier {
        format!("{}-{}-{}.{}", artifact, version, c, extension)
    } else {
        format!("{}-{}.{}", artifact, version, extension)
    };

    Some(format!("{}/{}/{}/{}", group, artifact, version, filename))
}

/// 获取当前 OS 的 Minecraft 名称
///
/// Minecraft 版本 JSON 使用 `"osx"` 而非 Rust 的 `"macos"`。
pub fn get_mc_os_name() -> &'static str {
    if std::env::consts::OS == "macos" {
        "osx"
    } else {
        std::env::consts::OS
    }
}

/// 评估 Minecraft 版本 JSON 中的 rules 规则（库模式）
///
/// 用于判断库是否需要下载/包含。
/// 规则逻辑：
/// - 无 rules → 默认允许
/// - 有 rules → 默认禁止，按顺序评估每条规则：
///   - 无条件规则(无 os 字段)：无条件应用 action
///   - 有条件规则(有 os 字段)：仅当条件匹配时应用 action
pub fn evaluate_rules(rules: Option<&serde_json::Value>) -> bool {
    evaluate_rules_impl(rules, RuleDefault::Deny)
}

/// 评估 Minecraft 版本 JSON 中的 rules 规则（参数模式）
///
/// 用于判断 JVM/游戏参数是否应该包含。
/// 与 `evaluate_rules` 的区别：
/// - 有 rules 时默认**允许**（黑名单模式），只有被 disallow 才排除
/// - 支持 features 条件检查（Minecraft 1.20+ 的可选参数）
pub fn evaluate_rules_for_arguments(rules: Option<&serde_json::Value>) -> bool {
    evaluate_rules_impl(rules, RuleDefault::Allow)
}

/// 规则评估的默认行为
enum RuleDefault {
    Allow,
    Deny,
}

/// 统一的规则评估实现
fn evaluate_rules_impl(rules: Option<&serde_json::Value>, default: RuleDefault) -> bool {
    let Some(rules_array) = rules.and_then(|r| r.as_array()) else {
        return true;
    };

    if rules_array.is_empty() {
        return matches!(default, RuleDefault::Allow);
    }

    let mc_os = get_mc_os_name();
    let mut allowed = matches!(default, RuleDefault::Allow);

    for rule in rules_array {
        let action_is_allow = rule["action"].as_str() == Some("allow");

        if let Some(features) = rule.get("features") {
            if check_features(features) {
                // 启动器支持所有请求的 features，应用 action
                allowed = action_is_allow;
            } else {
                // 启动器不支持请求的 features，这条规则的条件不满足
                // 对于 allow 规则：条件不满足 = 不允许
                // 对于 disallow 规则：条件不满足 = 不禁止（保持默认）
                if action_is_allow {
                    allowed = false;
                }
            }
            continue;
        }

        if let Some(os) = rule.get("os") {
            let os_name = os["name"].as_str().unwrap_or("");
            if os_name == mc_os {
                allowed = action_is_allow;
            } else {
                allowed = !action_is_allow;
            }
        } else {
            allowed = action_is_allow;
        }
    }

    allowed
}

/// 检查启动器是否支持指定的 features
///
/// Minecraft 1.20+ 的版本 JSON 使用 features 来控制可选参数。
/// 只有所有请求的 features 都被启动器支持时才返回 true。
pub fn check_features(features: &serde_json::Value) -> bool {
    const SUPPORTED_FEATURES: &[&str] = &[
        "has_custom_resolution",
    ];

    if let Some(obj) = features.as_object() {
        for (key, value) in obj {
            if value.as_bool() == Some(true) && !SUPPORTED_FEATURES.contains(&key.as_str()) {
                return false;
            }
        }
    }
    true
}

/// 递归查找最终的 JAR 版本（处理多层继承链）
///
/// Minecraft 的版本 JSON 可能通过 `jar` 字段或 `inheritsFrom` 字段
/// 引用另一个版本的 JAR 文件。此函数递归查找继承链，
/// 返回最终应该使用的 JAR 版本名称。
///
/// 查找优先级：
/// 1. `jar` 字段（直接指定使用哪个 JAR）
/// 2. `inheritsFrom` 字段（递归查找父版本）
/// 3. 版本 JSON 的 `id` 字段（兜底）
pub fn find_jar_version(
    version_json: &serde_json::Value,
    game_dir: &std::path::Path,
) -> Result<String, crate::errors::LauncherError> {
    if let Some(jar) = version_json["jar"].as_str() {
        return Ok(jar.to_string());
    }

    if let Some(inherits_from) = version_json["inheritsFrom"].as_str() {
        let parent_json_path = game_dir
            .join("versions")
            .join(inherits_from)
            .join(format!("{}.json", inherits_from));

        if parent_json_path.exists() {
            let parent_str = std::fs::read_to_string(&parent_json_path)?;
            let parent_json: serde_json::Value = serde_json::from_str(&parent_str)?;
            return find_jar_version(&parent_json, game_dir);
        }

        return Ok(inherits_from.to_string());
    }

    if let Some(id) = version_json["id"].as_str() {
        return Ok(id.to_string());
    }

    Err(crate::errors::LauncherError::Custom(
        "无法确定 JAR 版本".to_string(),
    ))
}

/// 修改 options.txt 设置游戏语言
pub fn set_game_language(instance_dir: &std::path::Path, lang: &str) -> std::io::Result<()> {
    let options_path = instance_dir.join("options.txt");
    let content = if options_path.exists() {
        std::fs::read_to_string(&options_path)?
    } else {
        String::new()
    };

    let lang_line = format!("lang:{}", lang);
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut found = false;

    for line in lines.iter_mut() {
        if line.starts_with("lang:") {
            *line = lang_line.clone();
            found = true;
            break;
        }
    }

    if !found {
        lines.push(lang_line);
    }

    std::fs::write(&options_path, lines.join("\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_new_version() {
        assert!(is_new_version("1.17"));
        assert!(is_new_version("1.17.1"));
        assert!(is_new_version("1.20.4"));
        assert!(is_new_version("1.21"));
        assert!(is_new_version("1.21.1"));
        assert!(is_new_version("1.22"));
        assert!(is_new_version("1.20.2-forge-48.0.48"));
        assert!(is_new_version("26.1.0"));
        assert!(is_new_version("26.1"));
        assert!(is_new_version("26.0.1-fabric-0.16.0"));
        assert!(!is_new_version("1.16.5"));
        assert!(!is_new_version("1.12.2"));
        assert!(!is_new_version("1.7.10"));
    }

    #[test]
    fn test_is_mid_version() {
        assert!(is_mid_version("1.12"));
        assert!(is_mid_version("1.16.5"));
        assert!(is_mid_version("1.14.4"));
        assert!(!is_mid_version("1.17"));
        assert!(!is_mid_version("1.11"));
        assert!(!is_mid_version("1.7.10"));
        assert!(!is_mid_version("26.1.0"));
    }

    #[test]
    fn test_maven_name_to_path_basic() {
        let result = maven_name_to_path("net.minecraftforge:forge:1.20.2-48.0.48", None, "jar");
        assert_eq!(
            result,
            Some("net/minecraftforge/forge/1.20.2-48.0.48/forge-1.20.2-48.0.48.jar".to_string())
        );
    }

    #[test]
    fn test_maven_name_to_path_with_classifier() {
        let result = maven_name_to_path("org.lwjgl:lwjgl:3.3.1", Some("natives-windows"), "jar");
        assert_eq!(
            result,
            Some("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar".to_string())
        );
    }

    #[test]
    fn test_maven_name_to_path_invalid() {
        assert_eq!(maven_name_to_path("invalid", None, "jar"), None);
        assert_eq!(maven_name_to_path("only:two", None, "jar"), None);
    }

    #[test]
    fn test_maven_name_to_path_empty() {
        assert_eq!(maven_name_to_path("", None, "jar"), None);
    }

    #[test]
    fn test_maven_name_to_path_extra_parts() {
        let result = maven_name_to_path("com.example:artifact:1.0:class1:class2", None, "jar");
        assert_eq!(
            result,
            Some("com/example/artifact/1.0/artifact-1.0-class1.jar".to_string())
        );
    }

    #[test]
    fn test_maven_name_to_path_no_dots_in_group() {
        let result = maven_name_to_path("mymod:core:2.0.0", None, "jar");
        assert_eq!(
            result,
            Some("mymod/core/2.0.0/core-2.0.0.jar".to_string())
        );
    }

    #[test]
    fn test_maven_name_to_path_custom_extension() {
        let result = maven_name_to_path("net.minecraftforge:forge:1.20.2-48.0.48", Some("sources"), "zip");
        assert_eq!(
            result,
            Some("net/minecraftforge/forge/1.20.2-48.0.48/forge-1.20.2-48.0.48-sources.zip".to_string())
        );
    }

    #[test]
    fn test_evaluate_rules_no_rules() {
        assert!(evaluate_rules(None));
    }

    #[test]
    fn test_evaluate_rules_unconditional_allow() {
        let rules = serde_json::json!([{"action": "allow"}]);
        assert!(evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_unconditional_disallow() {
        let rules = serde_json::json!([{"action": "disallow"}]);
        assert!(!evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_os_conditional() {
        let current_mc_os = get_mc_os_name();

        // 允许当前 OS
        let rules = serde_json::json!([
            {"action": "allow", "os": {"name": current_mc_os}}
        ]);
        assert!(evaluate_rules(Some(&rules)));

        // 禁止当前 OS
        let rules = serde_json::json!([
            {"action": "disallow", "os": {"name": current_mc_os}}
        ]);
        assert!(!evaluate_rules(Some(&rules)));

        // 允许其他 OS（当前 OS 不匹配，保持默认禁止）
        let other_os = if current_mc_os == "windows" { "osx" } else { "windows" };
        let rules = serde_json::json!([
            {"action": "allow", "os": {"name": other_os}}
        ]);
        assert!(!evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_mixed() {
        let current_mc_os = get_mc_os_name();

        // 先无条件 allow，再条件性 disallow 当前 OS → 最终 disallow
        let rules = serde_json::json!([
            {"action": "allow"},
            {"action": "disallow", "os": {"name": current_mc_os}}
        ]);
        assert!(!evaluate_rules(Some(&rules)));

        // 先无条件 disallow，再条件性 allow 当前 OS → 最终 allow
        let rules = serde_json::json!([
            {"action": "disallow"},
            {"action": "allow", "os": {"name": current_mc_os}}
        ]);
        assert!(evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_empty_array() {
        let rules = serde_json::json!([]);
        assert!(!evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_os_no_match() {
        let other_os = if get_mc_os_name() == "windows" { "osx" } else { "windows" };
        let rules = serde_json::json!([
            {"action": "disallow", "os": {"name": other_os}}
        ]);
        assert!(evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_multiple_os_rules() {
        let current_mc_os = get_mc_os_name();

        let rules = serde_json::json!([
            {"action": "allow", "os": {"name": "windows"}},
            {"action": "disallow", "os": {"name": current_mc_os}}
        ]);
        assert!(!evaluate_rules(Some(&rules)));

        let rules = serde_json::json!([
            {"action": "disallow", "os": {"name": "windows"}},
            {"action": "allow", "os": {"name": current_mc_os}}
        ]);
        assert!(evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_invalid_action() {
        let rules = serde_json::json!([
            {"action": "maybe"}
        ]);
        assert!(!evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_missing_action() {
        let rules = serde_json::json!([
            {"os": {"name": "windows"}}
        ]);
        assert!(!evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_os_without_name() {
        let rules = serde_json::json!([
            {"action": "allow", "os": {"arch": "x86"}}
        ]);
        assert!(!evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_non_array() {
        let rules = serde_json::json!({"action": "allow"});
        assert!(evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_for_arguments_no_rules() {
        assert!(evaluate_rules_for_arguments(None));
    }

    #[test]
    fn test_evaluate_rules_for_arguments_default_allow() {
        let rules = serde_json::json!([
            {"action": "disallow", "os": {"name": "osx"}}
        ]);
        let current_mc_os = get_mc_os_name();
        if current_mc_os != "osx" {
            assert!(evaluate_rules_for_arguments(Some(&rules)));
        }
    }

    #[test]
    fn test_evaluate_rules_for_arguments_disallow_current_os() {
        let current_mc_os = get_mc_os_name();
        let rules = serde_json::json!([
            {"action": "disallow", "os": {"name": current_mc_os}}
        ]);
        assert!(!evaluate_rules_for_arguments(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_for_arguments_features_supported() {
        let rules = serde_json::json!([
            {"action": "allow", "features": {"has_custom_resolution": true}}
        ]);
        assert!(evaluate_rules_for_arguments(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_for_arguments_features_unsupported() {
        // 不支持的 features 应该返回 false，参数不应该被添加
        let rules = serde_json::json!([
            {"action": "allow", "features": {"is_demo_user": true}}
        ]);
        assert!(!evaluate_rules_for_arguments(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_for_arguments_empty_array() {
        let rules = serde_json::json!([]);
        assert!(evaluate_rules_for_arguments(Some(&rules)));
    }

    #[test]
    fn test_evaluate_rules_for_arguments_allow_on_osx_only() {
        let rules = serde_json::json!([
            {"action": "allow", "os": {"name": "osx"}}
        ]);
        let current_mc_os = get_mc_os_name();
        if current_mc_os == "osx" {
            assert!(evaluate_rules_for_arguments(Some(&rules)));
        } else {
            assert!(!evaluate_rules_for_arguments(Some(&rules)));
        }
    }

    #[test]
    fn test_evaluate_rules_allow_on_osx_only_library() {
        let rules = serde_json::json!([
            {"action": "allow", "os": {"name": "osx"}}
        ]);
        let current_mc_os = get_mc_os_name();
        if current_mc_os == "osx" {
            assert!(evaluate_rules(Some(&rules)));
        } else {
            assert!(!evaluate_rules(Some(&rules)));
        }
    }

    #[test]
    fn test_check_features_supported() {
        let features = serde_json::json!({"has_custom_resolution": true});
        assert!(check_features(&features));
    }

    #[test]
    fn test_check_features_unsupported() {
        let features = serde_json::json!({"is_demo_user": true});
        assert!(!check_features(&features));
    }

    #[test]
    fn test_check_features_mixed() {
        let features = serde_json::json!({"has_custom_resolution": true, "is_demo_user": true});
        assert!(!check_features(&features));
    }

    #[test]
    fn test_get_mc_os_name() {
        let os = get_mc_os_name();
        assert!(
            os == "windows" || os == "linux" || os == "osx",
            "未知的 OS 名称: {}",
            os
        );
    }

    #[test]
    fn test_set_game_language_create_new_file() {
        let dir = tempfile::tempdir().unwrap();
        set_game_language(dir.path(), "zh_cn").unwrap();

        let content = std::fs::read_to_string(dir.path().join("options.txt")).unwrap();
        assert!(content.contains("lang:zh_cn"));
    }

    #[test]
    fn test_set_game_language_update_existing() {
        let dir = tempfile::tempdir().unwrap();
        let options_path = dir.path().join("options.txt");
        std::fs::write(&options_path, "lang:en_us\nrenderDistance:12\n").unwrap();

        set_game_language(dir.path(), "zh_cn").unwrap();

        let content = std::fs::read_to_string(&options_path).unwrap();
        assert!(content.contains("lang:zh_cn"));
        assert!(!content.contains("lang:en_us"));
        assert!(content.contains("renderDistance:12"));
    }

    #[test]
    fn test_set_game_language_append_to_existing() {
        let dir = tempfile::tempdir().unwrap();
        let options_path = dir.path().join("options.txt");
        std::fs::write(&options_path, "renderDistance:12\nfov:70\n").unwrap();

        set_game_language(dir.path(), "ja_jp").unwrap();

        let content = std::fs::read_to_string(&options_path).unwrap();
        assert!(content.contains("lang:ja_jp"));
        assert!(content.contains("renderDistance:12"));
        assert!(content.contains("fov:70"));
    }

    #[test]
    fn test_set_game_language_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let options_path = dir.path().join("options.txt");
        std::fs::write(&options_path, "").unwrap();

        set_game_language(dir.path(), "en_us").unwrap();

        let content = std::fs::read_to_string(&options_path).unwrap();
        assert!(content.contains("lang:en_us"));
    }

    #[test]
    fn test_set_game_language_update_preserves_other_lang_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let options_path = dir.path().join("options.txt");
        std::fs::write(&options_path, "lang:en_us\nlangServer:en_us\n").unwrap();

        set_game_language(dir.path(), "zh_cn").unwrap();

        let content = std::fs::read_to_string(&options_path).unwrap();
        assert!(content.contains("lang:zh_cn"));
        assert!(content.contains("langServer:en_us"));
    }
}

