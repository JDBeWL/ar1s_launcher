//! 版本 JSON 加载和合并逻辑

use crate::errors::LauncherError;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// 加载并合并版本 JSON 文件，处理 `inheritsFrom` 继承关系
pub fn load_and_merge_version_json(
    game_dir: &Path,
    version: &str,
) -> Result<serde_json::Value, LauncherError> {
    let version_dir = game_dir.join("versions").join(version);
    let version_json_path = version_dir.join(format!("{}.json", version));

    if !version_json_path.exists() {
        return Err(LauncherError::Custom(format!(
            "版本 {} 的json文件不存在!",
            version
        )));
    }

    let version_json_str = fs::read_to_string(&version_json_path)?;
    let mut version_json: serde_json::Value = serde_json::from_str(&version_json_str)?;

    // 如果版本声明了 inheritsFrom，递归加载并合并父版本的字段（子级优先）
    if let Some(mut parent_id) = version_json
        .get("inheritsFrom")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
    {
        let versions_base = game_dir.join("versions");
        // 循环处理多层继承
        while !parent_id.is_empty() {
            let parent_json_path = versions_base
                .join(&parent_id)
                .join(format!("{}.json", &parent_id));
            if !parent_json_path.exists() {
                break;
            }
            let parent_str = fs::read_to_string(&parent_json_path)?;
            let parent_json: serde_json::Value = serde_json::from_str(&parent_str)?;

            merge_libraries(&mut version_json, &parent_json);
            merge_arguments(&mut version_json, &parent_json);
            merge_other_fields(&mut version_json, &parent_json);

            // 处理下一层继承
            if let Some(next_parent) = parent_json.get("inheritsFrom").and_then(|v| v.as_str()) {
                parent_id = next_parent.to_string();
            } else {
                break;
            }
        }
    }
    Ok(version_json)
}

/// 合并 libraries 数组（去重）
fn merge_libraries(version_json: &mut serde_json::Value, parent_json: &serde_json::Value) {
    let Some(parent_libs) = parent_json.get("libraries").and_then(|v| v.as_array()) else {
        return;
    };

    let mut merged_libs: Vec<serde_json::Value> = Vec::new();
    let mut seen = HashSet::new();

    if let Some(cur_libs) = version_json.get("libraries").and_then(|v| v.as_array()) {
        for lib in cur_libs {
            if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                seen.insert(name.to_string());
            }
            merged_libs.push(lib.clone());
        }
    }

    for lib in parent_libs {
        if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
            if seen.contains(name) {
                continue;
            }
        }
        merged_libs.push(lib.clone());
    }

    if !merged_libs.is_empty() {
        version_json["libraries"] = serde_json::Value::Array(merged_libs);
    }
}

/// 合并 arguments（game 和 jvm）
fn merge_arguments(version_json: &mut serde_json::Value, parent_json: &serde_json::Value) {
    if let Some(parent_args) = parent_json.get("arguments") {
        if version_json.get("arguments").is_none() {
            version_json["arguments"] = parent_args.clone();
            return;
        }

        // 合并 game 数组
        if let Some(parent_game_arr) = parent_args.get("game").and_then(|g| g.as_array()).cloned() {
            merge_game_arguments(version_json, parent_game_arr);
        }
    } else if let Some(parent_mc_args) = parent_json.get("minecraftArguments") {
        // 父级使用旧式 minecraftArguments
        if let Some(mc_args_str) = parent_mc_args.as_str() {
            let parts: Vec<serde_json::Value> = mc_args_str
                .split(' ')
                .filter(|s| !s.is_empty())
                .map(|s| serde_json::Value::String(s.to_string()))
                .collect();

            if version_json.get("arguments").is_none() {
                let mut args_obj = serde_json::Map::new();
                args_obj.insert("game".to_string(), serde_json::Value::Array(parts));
                version_json["arguments"] = serde_json::Value::Object(args_obj);
            } else {
                merge_game_arguments(version_json, parts);
            }
        }
    }
}

/// 合并 game 参数数组
fn merge_game_arguments(version_json: &mut serde_json::Value, parent_game_arr: Vec<serde_json::Value>) {
    if version_json
        .get("arguments")
        .and_then(|a| a.get("game"))
        .is_none()
    {
        version_json["arguments"]["game"] = serde_json::Value::Array(parent_game_arr);
        return;
    }

    if let Some(child_game_arr) = version_json
        .get("arguments")
        .and_then(|a| a.get("game"))
        .and_then(|g| g.as_array())
        .cloned()
    {
        let mut merged: Vec<serde_json::Value> = Vec::new();
        for p in parent_game_arr {
            if !child_game_arr.contains(&p) {
                merged.push(p);
            }
        }
        for c in child_game_arr {
            merged.push(c);
        }
        version_json["arguments"]["game"] = serde_json::Value::Array(merged);
    }
}

/// 合并其他顶层字段（不覆盖已有）
fn merge_other_fields(version_json: &mut serde_json::Value, parent_json: &serde_json::Value) {
    if let Some(obj) = parent_json.as_object() {
        for (k, v) in obj.iter() {
            if version_json.get(k).is_none() {
                version_json[k] = v.clone();
            }
        }
    }
}
