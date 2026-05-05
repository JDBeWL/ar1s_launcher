//! JVM 和游戏参数构建逻辑

use crate::models::{GameConfig, LaunchOptions};
use std::path::Path;

/// 从版本 JSON 中获取基础 Minecraft 版本名
/// 用于 Forge 的 ignoreList 参数（需要原版 MC jar 文件名）
fn get_base_minecraft_version(version_json: &serde_json::Value, fallback: &str) -> String {
    // 1. 优先使用 jar 字段（直接指定了使用哪个 jar）
    if let Some(jar) = version_json.get("jar").and_then(|j| j.as_str()) {
        return jar.to_string();
    }
    
    // 2. 从 inheritsFrom 链中查找基础 MC 版本
    // Forge 版本格式通常是 "1.20.2-forge-48.0.48"，我们需要提取 "1.20.2"
    if let Some(inherits) = version_json.get("inheritsFrom").and_then(|i| i.as_str()) {
        // 如果 inheritsFrom 包含 "forge"，提取前面的 MC 版本
        if inherits.contains("-forge") || inherits.contains("-neoforge") {
            if let Some(mc_ver) = inherits.split('-').next() {
                return mc_ver.to_string();
            }
        }
        // 如果 inheritsFrom 看起来像纯 MC 版本号（如 "1.20.2"）
        if inherits.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) 
           && !inherits.contains("forge") {
            return inherits.to_string();
        }
    }
    
    // 3. 从当前版本 ID 尝试提取
    if fallback.contains("-forge") {
        if let Some(mc_ver) = fallback.split("-forge").next() {
            return mc_ver.to_string();
        }
    }
    
    // 4. 回退到传入的版本名
    fallback.to_string()
}

/// 构建 JVM 和游戏参数
pub fn build_arguments(
    version_json: &serde_json::Value,
    config: &GameConfig,
    options: &LaunchOptions,
    uuid: &str,
    version_dir: &Path,
    game_dir: &Path,
    assets_dir: &Path,
    assets_index: &str,
    current_os: &str,
    classpath: &[std::path::PathBuf],
    emit: &impl Fn(&str, String),
) -> (Vec<String>, Vec<String>) {
    let libraries_dir = game_dir.join("libraries");
    let classpath_separator = if cfg!(windows) { ";" } else { ":" };
    
    // 获取原版 Minecraft 版本名（用于 Forge 的 ignoreList）
    // 优先使用 jar 字段，其次从 inheritsFrom 链中查找基础 MC 版本
    let base_mc_version = get_base_minecraft_version(version_json, &options.version);
    
    let replace_placeholders = |arg: &str| -> String {
        let actual_game_dir = if config.version_isolation {
            version_dir.to_string_lossy().to_string()
        } else {
            game_dir.to_string_lossy().to_string()
        };

        let auth_access_token = options
            .access_token
            .as_deref()
            .unwrap_or("0");

        let auth_uuid = options
            .uuid
            .as_deref()
            .unwrap_or(uuid);

        let user_type = if options.auth_type.as_deref() == Some("microsoft") {
            "msa"
        } else {
            "mojang"
        };

        let version_type = if let Some(t) = version_json["type"].as_str() {
            if t == "snapshot" || t == "old_beta" || t == "old_alpha" {
                t
            } else {
                "release"
            }
        } else {
            "release"
        };

        arg.replace("${auth_player_name}", &options.username)
            .replace("${version_name}", &base_mc_version)
            .replace("${game_directory}", &actual_game_dir)
            .replace("${assets_root}", &assets_dir.to_string_lossy())
            .replace("${assets_index_name}", assets_index)
            .replace("${auth_uuid}", auth_uuid)
            .replace("${auth_access_token}", auth_access_token)
            .replace("${user_type}", user_type)
            .replace("${version_type}", version_type)
            .replace("${user_properties}", "{}")
            // 新版 Forge (1.13+) 需要的占位符
            .replace("${library_directory}", &libraries_dir.to_string_lossy())
            .replace("${classpath_separator}", classpath_separator)
    };

    let mut jvm_args = vec![];
    let mut game_args_vec = vec![];

    // 处理新版 (1.13+) `arguments` 格式
    if let Some(arguments) = version_json.get("arguments") {
        jvm_args = parse_jvm_arguments(arguments, current_os, &replace_placeholders);
        game_args_vec = parse_game_arguments(arguments, current_os, &replace_placeholders);
    }
    // 处理旧版 `minecraftArguments` 格式
    else if let Some(mc_args) = version_json["minecraftArguments"].as_str() {
        game_args_vec = mc_args.split(' ').map(&replace_placeholders).collect();
    }

    // 自动补齐 tweakClass
    auto_add_tweak_class(
        version_json,
        options,
        classpath,
        &mut game_args_vec,
        emit,
    );

    (jvm_args, game_args_vec)
}

/// 解析 JVM 参数
fn parse_jvm_arguments(
    arguments: &serde_json::Value,
    current_os: &str,
    replace_placeholders: &impl Fn(&str) -> String,
) -> Vec<String> {
    let mut jvm_args = vec![];

    let Some(jvm) = arguments["jvm"].as_array() else {
        return jvm_args;
    };

    for arg in jvm {
        if let Some(s) = arg.as_str() {
            jvm_args.push(replace_placeholders(s));
        } else if let Some(obj) = arg.as_object() {
            if is_rule_allowed(obj, current_os) {
                if let Some(value) = obj.get("value") {
                    if let Some(s) = value.as_str() {
                        jvm_args.push(replace_placeholders(s));
                    } else if let Some(arr) = value.as_array() {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                jvm_args.push(replace_placeholders(s));
                            }
                        }
                    }
                }
            }
        }
    }

    jvm_args
}

/// 解析游戏参数
fn parse_game_arguments(
    arguments: &serde_json::Value,
    current_os: &str,
    replace_placeholders: &impl Fn(&str) -> String,
) -> Vec<String> {
    let mut game_args = vec![];

    if let Some(game) = arguments["game"].as_array() {
        for arg in game {
            if let Some(s) = arg.as_str() {
                game_args.push(replace_placeholders(s));
            } else if let Some(obj) = arg.as_object() {
                if is_rule_allowed(obj, current_os) {
                    if let Some(value) = obj.get("value") {
                        if let Some(s) = value.as_str() {
                            game_args.push(replace_placeholders(s));
                        } else if let Some(arr) = value.as_array() {
                            for item in arr {
                                if let Some(s) = item.as_str() {
                                    game_args.push(replace_placeholders(s));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    game_args
}

/// 检查规则是否允许
/// Minecraft 规则语义：
/// - allow 规则：只允许匹配的 OS/特性，其他被排除
/// - disallow 规则：禁止匹配的 OS/特性
/// - features 规则：根据启动器支持的功能决定是否包含参数
fn is_rule_allowed(obj: &serde_json::Map<String, serde_json::Value>, current_os: &str) -> bool {
    let Some(rules) = obj.get("rules").and_then(|r| r.as_array()) else {
        return true;
    };

    let mut allowed = true;

    for rule in rules {
        let action = rule["action"].as_str().unwrap_or("");

        // 检查 features 条件
        if let Some(features) = rule.get("features") {
            let feature_allowed = check_features(features);
            match action {
                "allow" => {
                    allowed = feature_allowed;
                }
                "disallow" => {
                    if feature_allowed {
                        allowed = false;
                    }
                }
                _ => {}
            }
            continue;
        }

        // 检查 OS 条件
        if let Some(os) = rule.get("os") {
            if let Some(name) = os["name"].as_str() {
                match action {
                    "allow" => {
                        if name == current_os {
                            allowed = true;
                        } else if !allowed {
                            // 已被其他 allow 规则排除了，保持排除
                        } else {
                            allowed = false;
                        }
                    }
                    "disallow" => {
                        if name == current_os {
                            allowed = false;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            // 没有 OS 也没有 features 条件的规则
            match action {
                "allow" => {
                    allowed = true;
                }
                "disallow" => {
                    allowed = false;
                }
                _ => {}
            }
        }
    }

    allowed
}

/// 检查启动器是否支持指定的 features
/// Minecraft 1.20+ 的版本 JSON 使用 features 来控制可选参数
fn check_features(features: &serde_json::Value) -> bool {
    // 启动器支持的 features
    const SUPPORTED_FEATURES: &[&str] = &[
        "has_custom_resolution",  // 启动器支持 --width/--height
    ];

    if let Some(obj) = features.as_object() {
        // 所有请求的 features 都必须是启动器支持的
        for (key, value) in obj {
            if value.as_bool() == Some(true) && !SUPPORTED_FEATURES.contains(&key.as_str()) {
                return false;
            }
        }
    }
    true
}

/// 自动补齐 tweakClass（仅在 LaunchWrapper 主类下）
fn auto_add_tweak_class(
    version_json: &serde_json::Value,
    options: &LaunchOptions,
    classpath: &[std::path::PathBuf],
    game_args: &mut Vec<String>,
    emit: &impl Fn(&str, String),
) {
    let main_class = version_json["mainClass"].as_str().unwrap_or("");
    let has_tweak_class_flag = game_args.iter().any(|a| a == "--tweakClass");

    if main_class != "net.minecraft.launchwrapper.Launch" || has_tweak_class_flag {
        return;
    }

    // 检测是否存在 Forge/FML 相关库
    let forge_in_libraries = version_json
        .get("libraries")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().any(|lib| {
                lib.get("name")
                    .and_then(|n| n.as_str())
                    .map(|name| name.contains("net.minecraftforge") || name.contains("cpw.mods"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    let forge_in_classpath = classpath.iter().any(|p| {
        let s = p.to_string_lossy().to_lowercase();
        s.contains("minecraftforge")
            || s.contains("forge-")
            || s.contains("/fml/")
            || s.contains("\\fml\\")
    });

    if !forge_in_libraries && !forge_in_classpath {
        emit(
            "log-debug",
            "跳过自动补齐 tweakClass：未检测到 Forge/FML 库，避免 ClassNotFound".to_string(),
        );
        return;
    }

    // 从版本 id 推断基础 MC 版本
    let base_ver = options
        .version
        .split("-forge")
        .next()
        .unwrap_or(&options.version);

    let tweaker = if base_ver.starts_with("1.7.10") {
        "cpw.mods.fml.common.launcher.FMLTweaker"
    } else {
        "net.minecraftforge.fml.common.launcher.FMLTweaker"
    };

    emit("log-debug", format!("自动补齐 tweakClass: {}", tweaker));

    // 插入到参数最前
    game_args.insert(0, tweaker.to_string());
    game_args.insert(0, "--tweakClass".to_string());
}
