//! 版本 JSON 加载和合并逻辑
//!
//! 沿 inheritsFrom 逐级合并父版本，libraries 与 arguments 去重、子版本优先

use crate::errors::LauncherError;
use crate::utils::file_utils;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// 加载版本 JSON 并沿 inheritsFrom 合并父版本，子级字段优先
pub fn load_and_merge_version_json(
    game_dir: &Path,
    version: &str,
) -> Result<serde_json::Value, LauncherError> {
    file_utils::validate_version_id_or_error(version)?;
    let version_dir = file_utils::version_dir(&game_dir.join("versions"), version)?;
    let version_json_path = version_dir.join(format!("{}.json", version));

    if !version_json_path.exists() {
        return Err(LauncherError::Custom(format!(
            "版本 {} 的json文件不存在!",
            version
        )));
    }

    let version_json_str = fs::read_to_string(&version_json_path)?;
    let mut version_json: serde_json::Value = serde_json::from_str(&version_json_str)?;

    if let Some(mut parent_id) = version_json
        .get("inheritsFrom")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
    {
        let versions_base = game_dir.join("versions");
        let mut visited = HashSet::new();
        visited.insert(version.to_string());
        while !parent_id.is_empty() {
            if !visited.insert(parent_id.clone()) {
                return Err(LauncherError::Custom(format!(
                    "版本继承链存在循环，{} 重复出现",
                    parent_id
                )));
            }
            file_utils::validate_version_id_or_error(&parent_id)?;
            let parent_json_path = file_utils::version_dir(&versions_base, &parent_id)?
                .join(format!("{}.json", parent_id));
            if !parent_json_path.exists() {
                break;
            }
            let parent_str = fs::read_to_string(&parent_json_path)?;
            let parent_json: serde_json::Value = serde_json::from_str(&parent_str)?;

            merge_libraries(&mut version_json, &parent_json);
            merge_arguments(&mut version_json, &parent_json);
            merge_other_fields(&mut version_json, &parent_json);

            if let Some(next_parent) = parent_json.get("inheritsFrom").and_then(|v| v.as_str()) {
                parent_id = next_parent.to_string();
            } else {
                break;
            }
        }
    }
    Ok(version_json)
}

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
        if let Some(name) = lib.get("name").and_then(|n| n.as_str())
            && seen.contains(name)
        {
            continue;
        }
        merged_libs.push(lib.clone());
    }

    if !merged_libs.is_empty() {
        version_json["libraries"] = serde_json::Value::Array(merged_libs);
    }
}

fn merge_arguments(version_json: &mut serde_json::Value, parent_json: &serde_json::Value) {
    if let Some(parent_args) = parent_json.get("arguments") {
        if version_json.get("arguments").is_none() {
            version_json["arguments"] = parent_args.clone();
            return;
        }

        if let Some(parent_game_arr) = parent_args.get("game").and_then(|g| g.as_array()).cloned() {
            merge_game_arguments(version_json, parent_game_arr);
        }

        if let Some(parent_jvm_arr) = parent_args.get("jvm").and_then(|j| j.as_array()).cloned() {
            merge_jvm_arguments(version_json, parent_jvm_arr);
        }
    } else if let Some(parent_mc_args) = parent_json.get("minecraftArguments")
        && let Some(mc_args_str) = parent_mc_args.as_str()
    {
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

fn merge_game_arguments(
    version_json: &mut serde_json::Value,
    parent_game_arr: Vec<serde_json::Value>,
) {
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
        version_json["arguments"]["game"] =
            serde_json::Value::Array(merge_arg_groups(&parent_game_arr, &child_game_arr));
    }
}

/// 合并 jvm 数组；新版 Forge 1.13+ 需从父版本继承 --module-path 等参数
fn merge_jvm_arguments(
    version_json: &mut serde_json::Value,
    parent_jvm_arr: Vec<serde_json::Value>,
) {
    if version_json
        .get("arguments")
        .and_then(|a| a.get("jvm"))
        .is_none()
    {
        version_json["arguments"]["jvm"] = serde_json::Value::Array(parent_jvm_arr);
        return;
    }

    if let Some(child_jvm_arr) = version_json
        .get("arguments")
        .and_then(|a| a.get("jvm"))
        .and_then(|j| j.as_array())
        .cloned()
    {
        // 子版本的 JVM 参数优先，父版本的参数追加到前面
        version_json["arguments"]["jvm"] =
            serde_json::Value::Array(merge_arg_groups(&parent_jvm_arr, &child_jvm_arr));
    }
}

/// 把参数数组切成「不可拆的组」：`--add-opens` 这类开关和它后面的值是一体的
fn group_arguments(items: &[serde_json::Value]) -> Vec<Vec<serde_json::Value>> {
    let mut groups: Vec<Vec<serde_json::Value>> = Vec::new();
    let mut index = 0;

    while index < items.len() {
        let item = &items[index];

        if let Some(token) = item.as_str() {
            // 形如 `-cp` / `--add-opens` 的纯开关，与紧随其后的值合成一组；
            // 带 `=` 的（`--enable-native-access=ALL-UNNAMED`）本身就是完整参数
            let is_pair = token.starts_with('-')
                && !token.contains('=')
                && items
                    .get(index + 1)
                    .and_then(|next| next.as_str())
                    .is_some_and(|next| !next.starts_with('-'));

            if is_pair {
                groups.push(vec![item.clone(), items[index + 1].clone()]);
                index += 2;
                continue;
            }
        }

        groups.push(vec![item.clone()]);
        index += 1;
    }

    groups
}

/// 按「组」去重合并：按单个 token 去重会把父版本的 `--add-exports` 开关删掉、只剩裸值，
/// JVM 于是把裸值当主类，直接 ClassNotFoundException 退出（NeoForge 26.x 秒退就是这个）
fn merge_arg_groups(
    parent: &[serde_json::Value],
    child: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let child_groups = group_arguments(child);
    let mut merged: Vec<serde_json::Value> = Vec::new();

    for group in group_arguments(parent) {
        if !child_groups.contains(&group) {
            merged.extend(group);
        }
    }
    merged.extend(child.iter().cloned());
    merged
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

#[cfg(test)]
mod tests {
    use super::{group_arguments, load_and_merge_version_json, merge_arg_groups};
    use serde_json::{Value, json};

    fn strings(items: &[&str]) -> Vec<Value> {
        items.iter().map(|s| json!(s)).collect()
    }

    #[test]
    fn test_inherits_cycle_is_rejected() {
        let dir = tempfile::TempDir::new().unwrap();
        let versions = dir.path().join("versions");
        for (id, parent) in [("cycle-a", "cycle-b"), ("cycle-b", "cycle-a")] {
            let version_dir = versions.join(id);
            std::fs::create_dir_all(&version_dir).unwrap();
            std::fs::write(
                version_dir.join(format!("{id}.json")),
                format!(r#"{{"id":"{id}","inheritsFrom":"{parent}"}}"#),
            )
            .unwrap();
        }

        let err = load_and_merge_version_json(dir.path(), "cycle-a").unwrap_err();

        assert!(err.to_string().contains("循环"), "{err}");
    }

    /// 1.21+/26.x 的原版与 NeoForge 都写了 `--add-exports`，按 token 去重会留下没有开关的裸值
    #[test]
    fn test_repeated_flags_keep_their_own_values() {
        let parent = strings(&[
            "--enable-native-access=ALL-UNNAMED",
            "--add-exports",
            "java.base/jdk.internal.misc=ALL-UNNAMED",
            "-cp",
            "${classpath}",
        ]);
        let child = strings(&[
            "--add-opens",
            "java.base/java.lang.invoke=ALL-UNNAMED",
            "--add-exports",
            "jdk.naming.dns/com.sun.jndi.dns=java.naming",
        ]);

        let merged = merge_arg_groups(&parent, &child);

        assert_eq!(
            merged,
            strings(&[
                "--enable-native-access=ALL-UNNAMED",
                "--add-exports",
                "java.base/jdk.internal.misc=ALL-UNNAMED",
                "-cp",
                "${classpath}",
                "--add-opens",
                "java.base/java.lang.invoke=ALL-UNNAMED",
                "--add-exports",
                "jdk.naming.dns/com.sun.jndi.dns=java.naming",
            ])
        );
    }

    /// 任何以 `-` 开头的开关后面紧跟的值都必须成组，否则合并后会出现裸值
    #[test]
    fn test_no_option_is_left_without_its_value() {
        let parent = strings(&["--add-opens", "java.base/java.lang=ALL-UNNAMED"]);
        let child = strings(&["--add-opens", "java.base/java.util=ALL-UNNAMED"]);

        let merged = merge_arg_groups(&parent, &child);

        let mut index = 0;
        while index < merged.len() {
            let token = merged[index].as_str().unwrap();
            if token.starts_with('-') && !token.contains('=') {
                assert!(
                    merged
                        .get(index + 1)
                        .is_some_and(|v| !v.as_str().unwrap_or("").starts_with('-')),
                    "{} 后面没有值: {:?}",
                    token,
                    merged
                );
                index += 2;
            } else {
                assert!(
                    token.starts_with('-') || token == "${classpath}",
                    "出现了不该有的裸参数: {} ({:?})",
                    token,
                    merged
                );
                index += 1;
            }
        }
    }

    /// 完全相同的组才去重，父子版本都写了同一条时只留一份
    #[test]
    fn test_identical_groups_are_deduped() {
        let parent = strings(&["-Dfoo=bar", "--add-opens", "a/b=ALL-UNNAMED"]);
        let child = strings(&["--add-opens", "a/b=ALL-UNNAMED"]);

        let merged = merge_arg_groups(&parent, &child);

        assert_eq!(
            merged,
            strings(&["-Dfoo=bar", "--add-opens", "a/b=ALL-UNNAMED"])
        );
    }

    #[test]
    fn test_rules_objects_stay_whole() {
        let parent = vec![json!({
            "rules": [{ "action": "allow", "os": { "name": "windows" } }],
            "value": "-XX:+Foo"
        })];
        let child = strings(&["-Xss1M"]);

        let merged = merge_arg_groups(&parent, &child);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0], parent[0]);
    }

    /// 分组时带 `=` 的参数本身就是完整的，不该吃掉后面的值
    #[test]
    fn test_equals_form_is_not_paired() {
        let groups = group_arguments(&strings(&["--add-opens=a/b", "--username", "Steve"]));

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], strings(&["--add-opens=a/b"]));
        assert_eq!(groups[1], strings(&["--username", "Steve"]));
    }
}
