//! Minecraft 通用工具函数
//!
//! 提供多个模块共享的 Minecraft 相关工具函数，避免代码重复。

/// 将 Maven 坐标转换为文件路径
///
/// 例如: `"net.minecraftforge:forge:1.20.2-48.0.48"` → `"net/minecraftforge/forge/1.20.2-48.0.48/forge-1.20.2-48.0.48.jar"`
pub fn maven_name_to_path(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }

    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = if parts.len() > 3 { Some(parts[3]) } else { None };

    let filename = if let Some(c) = classifier {
        format!("{}-{}-{}.jar", artifact, version, c)
    } else {
        format!("{}-{}.jar", artifact, version)
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

/// 评估 Minecraft 版本 JSON 中的 rules 规则
///
/// 规则逻辑：
/// - 无 rules → 默认允许
/// - 有 rules → 默认禁止，按顺序评估每条规则：
///   - 无条件规则(无 os 字段)：无条件应用 action
///   - 有条件规则(有 os 字段)：仅当条件匹配时应用 action
pub fn evaluate_rules(rules: Option<&serde_json::Value>) -> bool {
    let Some(rules_array) = rules.and_then(|r| r.as_array()) else {
        return true; // 无 rules 默认允许
    };

    let mc_os = get_mc_os_name();
    let mut allowed = false; // 有 rules 时默认禁止

    for rule in rules_array {
        let action_is_allow = rule["action"].as_str() == Some("allow");

        if let Some(os) = rule.get("os") {
            // 有 OS 条件 → 仅当 OS 匹配时应用
            if let Some(name) = os["name"].as_str() {
                if name == mc_os {
                    allowed = action_is_allow;
                }
            }
        } else {
            // 无条件规则 → 直接应用
            allowed = action_is_allow;
        }
    }

    allowed
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
    fn test_maven_name_to_path_basic() {
        let result = maven_name_to_path("net.minecraftforge:forge:1.20.2-48.0.48");
        assert_eq!(
            result,
            Some("net/minecraftforge/forge/1.20.2-48.0.48/forge-1.20.2-48.0.48.jar".to_string())
        );
    }

    #[test]
    fn test_maven_name_to_path_with_classifier() {
        let result = maven_name_to_path("org.lwjgl:lwjgl:3.3.1:natives-windows");
        assert_eq!(
            result,
            Some("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar".to_string())
        );
    }

    #[test]
    fn test_maven_name_to_path_invalid() {
        assert_eq!(maven_name_to_path("invalid"), None);
        assert_eq!(maven_name_to_path("only:two"), None);
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
        // 有 rules 数组但为空 → 默认禁止
        assert!(!evaluate_rules(Some(&rules)));
    }

    #[test]
    fn test_get_mc_os_name() {
        let os = get_mc_os_name();
        // 必须是已知的 OS 名称之一
        assert!(
            os == "windows" || os == "linux" || os == "osx",
            "未知的 OS 名称: {}",
            os
        );
    }
}

