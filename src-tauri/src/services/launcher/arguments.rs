//! JVM 和游戏参数构建逻辑

use crate::models::{GameConfig, LaunchOptions};
use std::path::Path;

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
    let replace_placeholders = |arg: &str| -> String {
        let actual_game_dir = if config.version_isolation {
            version_dir.to_string_lossy().to_string()
        } else {
            game_dir.to_string_lossy().to_string()
        };

        arg.replace("${auth_player_name}", &options.username)
            .replace("${version_name}", &options.version)
            .replace("${game_directory}", &actual_game_dir)
            .replace("${assets_root}", &assets_dir.to_string_lossy())
            .replace("${assets_index_name}", assets_index)
            .replace("${auth_uuid}", uuid)
            .replace("${auth_access_token}", "0")
            .replace("${user_type}", "mojang")
            .replace(
                "${version_type}",
                version_json["type"].as_str().unwrap_or("release"),
            )
            .replace("${user_properties}", "{}")
    };

    let mut jvm_args = vec![];
    let mut game_args_vec = vec![];

    // 处理新版 (1.13+) `arguments` 格式
    if let Some(arguments) = version_json.get("arguments") {
        jvm_args = parse_jvm_arguments(arguments, current_os, &replace_placeholders);
        game_args_vec = parse_game_arguments(arguments, &replace_placeholders);
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
    replace_placeholders: &impl Fn(&str) -> String,
) -> Vec<String> {
    let mut game_args = vec![];

    if let Some(game) = arguments["game"].as_array() {
        for arg in game {
            if let Some(s) = arg.as_str() {
                game_args.push(replace_placeholders(s));
            }
        }
    }

    game_args
}

/// 检查规则是否允许
fn is_rule_allowed(obj: &serde_json::Map<String, serde_json::Value>, current_os: &str) -> bool {
    let Some(rules) = obj.get("rules").and_then(|r| r.as_array()) else {
        return true;
    };

    let mut allowed = true;
    for rule in rules {
        if let Some(os) = rule.get("os") {
            if let Some(name) = os["name"].as_str() {
                if name == current_os {
                    allowed = rule["action"].as_str() == Some("allow");
                } else {
                    allowed = rule["action"].as_str() != Some("allow");
                }
            }
        }
    }
    allowed
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
