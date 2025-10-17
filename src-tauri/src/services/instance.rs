use crate::errors::LauncherError;
use crate::models::{ForgeVersion, DownloadJob};
use crate::services::{config, download, forge};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use serde::Serialize;
use tauri::Emitter;

#[derive(Clone, Serialize)]
struct InstallProgress {
    progress: u8,
    message: String,
    indeterminate: bool,
}

/// 清理实例创建过程中创建的文件和目录
fn cleanup_instance_creation(
    game_dir: &PathBuf,
    instance_name: &str,
    _base_version_id: &str,
) {
    println!("Instance: 开始清理实例创建过程中的文件和目录");

    // 1. 清理实例目录
    let instance_dir = game_dir.join("versions").join(instance_name);
    if instance_dir.exists() {
        println!("Instance: 清理实例目录: {}", instance_dir.display());
        if let Err(e) = fs::remove_dir_all(&instance_dir) {
            println!("Instance: 清理实例目录失败: {}", e);
        } else {
            println!("Instance: 实例目录清理完成");
        }
    }

    // 2. 清理可能创建的临时文件
    let instance_json = game_dir.join("versions").join(instance_name).join(format!("{}.json", instance_name));
    if instance_json.exists() {
        println!("Instance: 清理实例JSON文件: {}", instance_json.display());
        if let Err(e) = fs::remove_file(&instance_json) {
            println!("Instance: 清理实例JSON文件失败: {}", e);
        }
    }

    let instance_jar = game_dir.join("versions").join(instance_name).join(format!("{}.jar", instance_name));
    if instance_jar.exists() {
        println!("Instance: 清理实例JAR文件: {}", instance_jar.display());
        if let Err(e) = fs::remove_file(&instance_jar) {
            println!("Instance: 清理实例JAR文件失败: {}", e);
        }
    }

    println!("Instance: 清理完成");
}

// Helper function to copy a directory recursively
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), std::io::Error> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub async fn create_instance(
    new_instance_name: String,
    base_version_id: String,
    forge_version: Option<ForgeVersion>,
    window: &tauri::Window,
) -> Result<(), LauncherError> {
    let config = config::load_config()?;
    let game_dir = PathBuf::from(config.game_dir);
    let versions_dir = game_dir.join("versions");

    let source_dir = versions_dir.join(&base_version_id);
    let dest_dir = versions_dir.join(&new_instance_name);

    // 发送进度更新
    let send_progress = |progress: u8, message: &str, indeterminate: bool| {
        let _ = window.emit("instance-install-progress", InstallProgress {
            progress,
            message: message.to_string(),
            indeterminate,
        });
    };

    // 1. Check if the destination instance name already exists
    if dest_dir.exists() {
        return Err(LauncherError::Custom(format!(
            "名为 '{}' 的实例已存在.",
            new_instance_name
        )));
    }

    send_progress(5, "检查基础版本...", false);

    // 2. Check if the base version exists, if not, download it.
    if !source_dir.exists() {
        send_progress(10, "下载基础版本...", true);
        if let Err(e) = download::process_and_download_version(base_version_id.clone(), config.download_mirror.clone(), window).await {
            // 基础版本下载失败，此时实例目录尚未创建，不需要清理
            return Err(e);
        }
    }
    
    if !source_dir.exists() {
        // 基础版本下载失败，此时实例目录尚未创建，不需要清理
        return Err(LauncherError::Custom(format!(
            "基础版本 '{}' 下载失败或未找到.",
            base_version_id
        )));
    }

    send_progress(30, "复制基础文件...", false);
    
    // 3. Copy the entire directory
    if let Err(e) = copy_dir_all(&source_dir, &dest_dir) {
        cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
        return Err(e.into());
    }

    send_progress(40, "重命名配置文件...", false);
    
    // 4. Rename the .json and .jar files
    let old_json_path = dest_dir.join(format!("{}.json", base_version_id));
    let new_json_path = dest_dir.join(format!("{}.json", new_instance_name));
    if let Err(e) = fs::rename(&old_json_path, &new_json_path) {
        cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
        return Err(e.into());
    }

    let old_jar_path = dest_dir.join(format!("{}.jar", base_version_id));
    if old_jar_path.exists() {
        let new_jar_path = dest_dir.join(format!("{}.jar", new_instance_name));
        if let Err(e) = fs::rename(&old_jar_path, &new_jar_path) {
            cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e.into());
        }
    }

    send_progress(50, "更新配置文件...", false);
    
    // 5. Modify the 'id' field in the new JSON file
    let json_str = match fs::read_to_string(&new_json_path) {
        Ok(s) => s,
        Err(e) => {
            cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e.into());
        }
    };
    
    let mut json: Value = match serde_json::from_str(&json_str) {
        Ok(j) => j,
        Err(e) => {
            cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e.into());
        }
    };

    if let Some(id_field) = json.get_mut("id") {
        *id_field = Value::String(new_instance_name.clone());
    }

    let modified_json_str = match serde_json::to_string_pretty(&json) {
        Ok(s) => s,
        Err(e) => {
            cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e.into());
        }
    };
    
    if let Err(e) = fs::write(&new_json_path, modified_json_str) {
        cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
        return Err(e.into());
    }

    // 6. Install Forge if requested
    if let Some(forge_version) = forge_version {
        send_progress(60, "安装Forge加载器...", true);
        if let Err(e) = forge::install_forge(dest_dir.clone(), forge_version.clone()).await {
            cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
            return Err(e);
        }

        // After Forge installer runs it usually creates a version entry like
        // `{mcversion}-forge-{forge_version}` under game_dir/versions.
        // Try to merge the Forge version json with the base (vanilla) json
        // following the rules described in the UI: Forge fields preferred,
        // libraries de-duplicated by group:artifact:version, id set to instance name.

        // Try to find actual forge-created directory. Common pattern is
        // "{mcversion}-forge-{build}" but some installers may write slightly different ids.
        // We'll scan the versions directory for entries starting with "{mcversion}-forge".
        let mut found_forge_id: Option<String> = None;
        if let Ok(entries) = fs::read_dir(game_dir.join("versions")) {
            let prefix = format!("{}-forge", forge_version.mcversion);
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == format!("{}-forge-{}", forge_version.mcversion, forge_version.version) {
                        // exact expected match
                        found_forge_id = Some(name.to_string());
                        break;
                    }
                }
            }
            if found_forge_id.is_none() {
                // fallback: first directory that starts with the prefix
                if let Ok(entries2) = fs::read_dir(game_dir.join("versions")) {
                    for entry in entries2.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with(&prefix) {
                                found_forge_id = Some(name.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        let forge_json_path = if let Some(ref fid) = found_forge_id {
            game_dir.join("versions").join(fid).join(format!("{}.json", fid))
        } else {
            game_dir.join("versions").join(format!("{}-forge-{}", forge_version.mcversion, forge_version.version)).join(format!("{}-forge-{}.json", forge_version.mcversion, forge_version.version))
        };
        let base_json_path = game_dir.join("versions").join(&base_version_id).join(format!("{}.json", &base_version_id));
        let instance_json_path = dest_dir.join(format!("{}.json", new_instance_name));

        if forge_json_path.exists() && base_json_path.exists() {
            let base_str = match fs::read_to_string(&base_json_path) {
                Ok(s) => s,
                Err(e) => {
                    cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };
            let forge_str = match fs::read_to_string(&forge_json_path) {
                Ok(s) => s,
                Err(e) => {
                    cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };
            let base_json: Value = match serde_json::from_str(&base_str) {
                Ok(j) => j,
                Err(e) => {
                    cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };
            let forge_json: Value = match serde_json::from_str(&forge_str) {
                Ok(j) => j,
                Err(e) => {
                    cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };

            // Start merged as forge_json, then fill missing fields from base_json
            let mut merged = forge_json.clone();

            // Ensure id is set to instance name
            merged["id"] = Value::String(new_instance_name.clone());

            // mainClass: prefer Forge (already in merged if present), otherwise take from base
            if !merged.get("mainClass").is_some() {
                if let Some(mc) = base_json.get("mainClass") {
                    merged["mainClass"] = mc.clone();
                }
            }

            // arguments: prefer Forge. If Forge uses old `minecraftArguments`, convert to `arguments.game` array
            if merged.get("arguments").is_none() {
                if let Some(mc_args) = forge_json.get("minecraftArguments").and_then(|v| v.as_str()) {
                    // split on spaces (same behaviour as launcher)
                    let parts: Vec<Value> = mc_args
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    let mut args_obj = serde_json::Map::new();
                    args_obj.insert("game".to_string(), Value::Array(parts));
                    merged["arguments"] = Value::Object(args_obj);
                } else if let Some(base_args) = base_json.get("arguments") {
                    merged["arguments"] = base_args.clone();
                } else if let Some(base_mc_args) = base_json.get("minecraftArguments").and_then(|v| v.as_str()) {
                    let parts: Vec<Value> = base_mc_args
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .map(|s| Value::String(s.to_string()))
                        .collect();
                    let mut args_obj = serde_json::Map::new();
                    args_obj.insert("game".to_string(), Value::Array(parts));
                    merged["arguments"] = Value::Object(args_obj);
                }
            }

            // libraries: use Forge as primary, then append base libraries that are missing (dedupe by name)
            let mut final_libs: Vec<Value> = Vec::new();
            let mut seen = std::collections::HashSet::new();

            if let Some(forge_libs) = forge_json.get("libraries").and_then(|v| v.as_array()) {
                for lib in forge_libs {
                    // try to get a unique key: library.name (group:artifact:version)
                    if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                        seen.insert(name.to_string());
                    }
                    final_libs.push(lib.clone());
                }
            }

            if let Some(base_libs) = base_json.get("libraries").and_then(|v| v.as_array()) {
                for lib in base_libs {
                    let mut add = true;
                    if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                        if seen.contains(name) {
                            add = false;
                        }
                    }
                    if add {
                        final_libs.push(lib.clone());
                    }
                }
            }

            if !final_libs.is_empty() {
                merged["libraries"] = Value::Array(final_libs);
            }

            // Fill other missing top-level fields from base_json (logging, assetIndex, etc.)
            if let Some(obj) = base_json.as_object() {
                for (k, v) in obj.iter() {
                    if !merged.get(k).is_some() {
                        merged[k] = v.clone();
                    }
                }
            }

            // Finally, write merged json to instance json path
            let merged_str = match serde_json::to_string_pretty(&merged) {
                Ok(s) => s,
                Err(e) => {
                    cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                    return Err(e.into());
                }
            };
            if let Err(e) = fs::write(&instance_json_path, merged_str) {
                cleanup_instance_creation(&game_dir, &new_instance_name, &base_version_id);
                return Err(e.into());
            }

            // After writing merged JSON, ensure required files (client, libraries, natives, assets) are downloaded
            // Build download jobs from merged manifest and run download_all_files
            // Helper closure to collect download jobs
            let collect_download_jobs = |merged_json: &Value| -> Result<Vec<DownloadJob>, LauncherError> {
                let mut jobs: Vec<DownloadJob> = Vec::new();
                let libraries_base_dir = game_dir.join("libraries");
                let assets_base_dir = game_dir.join("assets");

                // 1) client jar
                if let Some(client) = merged_json.get("downloads").and_then(|d| d.get("client")) {
                    if let Some(url) = client.get("url").and_then(|u| u.as_str()) {
                        let size = client.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                        let hash = client.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string();
                        let path = dest_dir.join(format!("{}.jar", new_instance_name));
                        jobs.push(DownloadJob { url: url.to_string(), fallback_url: None, path, size, hash });
                    }
                }

                // 2) asset index -> objects
                if let Some(asset_idx) = merged_json.get("assetIndex") {
                    if let Some(idx_id) = asset_idx.get("id").and_then(|v| v.as_str()) {
                        if let Some(idx_url) = asset_idx.get("url").and_then(|v| v.as_str()) {
                            let index_path = assets_base_dir.join("indexes").join(format!("{}.json", idx_id));
                            jobs.push(DownloadJob {
                                url: idx_url.to_string(),
                                fallback_url: None,
                                path: index_path.clone(),
                                size: asset_idx.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                                hash: asset_idx.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string(),
                            });

                            // If index already exists we still want to enqueue individual objects later when calling download_all_files,
                            // but building the list of objects requires the index file to be present. We'll handle objects download by reading the index after download.
                        }
                    }
                }

                // 3) libraries + natives
                if let Some(libs) = merged_json.get("libraries").and_then(|v| v.as_array()) {
                    for lib in libs {
                        // rules evaluation (copying logic from download.rs)
                        let mut should_download = true;
                        if let Some(rules) = lib.get("rules").and_then(|r| r.as_array()) {
                            should_download = false;
                            for rule in rules {
                                let action = rule.get("action").and_then(|a| a.as_str()).unwrap_or("");
                                if let Some(os) = rule.get("os") {
                                    if let Some(name) = os.get("name").and_then(|n| n.as_str()) {
                                        let current_os = std::env::consts::OS;
                                        if name == current_os {
                                            should_download = action == "allow";
                                        }
                                    }
                                } else {
                                    should_download = action == "allow";
                                }
                            }
                        }

                        if !should_download {
                            continue;
                        }

                        // artifact
                        if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                            let path = artifact.get("path").and_then(|p| p.as_str()).map(|s| s.to_string());
                            let url = artifact.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
                            let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                            let hash = artifact.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string();
                            if let Some(path_str) = path {
                                let file_path = libraries_base_dir.join(path_str.clone());
                                let download_url = if let Some(u) = url { u } else { format!("https://libraries.minecraft.net/{}", path_str) };
                                jobs.push(DownloadJob { url: download_url, fallback_url: None, path: file_path, size, hash });
                            }
                        }

                        // natives/classifiers
                        if let Some(natives) = lib.get("natives") {
                            if let Some(natives_map) = natives.as_object() {
                                let current_os = std::env::consts::OS;
                                for (os_name, classifier_val) in natives_map.iter() {
                                    let classifier = classifier_val.as_str().unwrap_or("");
                                    if os_name == current_os || lib.get("name").and_then(|n| n.as_str()).map_or(false, |s| s.contains("lwjgl")) {
                                        // prefer downloads.classifiers
                                        if let Some(classifiers) = lib.get("downloads").and_then(|d| d.get("classifiers")) {
                                            if let Some(artifact) = classifiers.get(classifier) {
                                                if let (Some(path), Some(url)) = (artifact.get("path").and_then(|p| p.as_str()), artifact.get("url").and_then(|u| u.as_str())) {
                                                    let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                                    let hash = artifact.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string();
                                                    let file_path = libraries_base_dir.join(path);
                                                    jobs.push(DownloadJob { url: url.to_string(), fallback_url: None, path: file_path, size, hash });
                                                    continue;
                                                }
                                            }
                                        }

                                        // fallback: try classifiers under top-level "classifiers"
                                        if let Some(classifiers) = lib.get("classifiers") {
                                            if let Some(artifact) = classifiers.get(classifier) {
                                                if let (Some(path), Some(url)) = (artifact.get("path").and_then(|p| p.as_str()), artifact.get("url").and_then(|u| u.as_str())) {
                                                    let size = artifact.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                                    let hash = artifact.get("sha1").and_then(|h| h.as_str()).unwrap_or("").to_string();
                                                    let file_path = libraries_base_dir.join(path);
                                                    jobs.push(DownloadJob { url: url.to_string(), fallback_url: None, path: file_path, size, hash });
                                                    continue;
                                                }
                                            }
                                        }

                                        // last resort: derive path from name
                                        if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                                            let parts: Vec<&str> = name.split(":").collect();
                                            if parts.len() >= 3 {
                                                let group_id = parts[0].replace(".", "/");
                                                let artifact_id = parts[1];
                                                let version = parts[2];
                                                let classifier_replaced = classifier.replace("${arch}", if cfg!(target_pointer_width = "64") { "64" } else { "32" });
                                                let natives_path = format!("{}/{}/{}/{}-{}-{}.jar", group_id, artifact_id, version, artifact_id, version, classifier_replaced);
                                                let natives_url = format!("https://libraries.minecraft.net/{}", natives_path);
                                                let file_path = libraries_base_dir.join(&natives_path);
                                                jobs.push(DownloadJob { url: natives_url, fallback_url: None, path: file_path, size: 0, hash: "".to_string() });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(jobs)
            };

            // collect and run download jobs
            if let Ok(mut jobs) = collect_download_jobs(&merged) {
                // If asset index job was present, we need to download index first, then expand objects into jobs
                // Find asset index job(s)
                let mut index_jobs: Vec<DownloadJob> = Vec::new();
                jobs.retain(|j| {
                    if j.path.to_string_lossy().contains("indexes") {
                        index_jobs.push(j.clone());
                        false
                    } else {
                        true
                    }
                });

                // First download index files
                if !index_jobs.is_empty() {
                    let _ = download::download_all_files(index_jobs.clone(), window, index_jobs.len() as u64, config.download_mirror.clone()).await;
                    // For each index, parse and enqueue objects
                    for idx in index_jobs.iter() {
                        if idx.path.exists() {
                            if let Ok(idx_content) = fs::read_to_string(&idx.path) {
                                if let Ok(idx_json) = serde_json::from_str::<Value>(&idx_content) {
                                    if let Some(objects) = idx_json.get("objects").and_then(|o| o.as_object()) {
                                        for (_p, obj) in objects.iter() {
                                            if let Some(hash) = obj.get("hash").and_then(|h| h.as_str()) {
                                                let size = obj.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                                let download_url = format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash);
                                                let file_path = game_dir.join("assets").join("objects").join(&hash[..2]).join(hash);
                                                jobs.push(DownloadJob { url: download_url, fallback_url: None, path: file_path, size, hash: hash.to_string() });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Finally run download for remaining jobs (libraries, client, assets objects)
                if !jobs.is_empty() {
                    send_progress(70, "下载游戏文件...", true);
                    let _ = download::download_all_files(jobs.clone(), window, jobs.len() as u64, config.download_mirror.clone()).await;
                }
            }
        } else {
            // If forge json doesn't exist yet, just continue without merging
            println!("Forge json not found at {}. Skipping merge.", forge_json_path.display());
        }
    }

    send_progress(100, "安装完成！", false);
    
    Ok(())
}