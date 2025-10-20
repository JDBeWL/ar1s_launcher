use crate::errors::LauncherError;
use crate::models::*;
use crate::services::config::{load_config, save_config};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::Emitter;
use uuid::Uuid;

fn generate_offline_uuid(username: &str) -> String {
    // 首先检查配置中是否已有保存的UUID
    if let Ok(config) = load_config() {
        // 如果用户名匹配且已有UUID，则直接返回保存的UUID
        if let (Some(saved_username), Some(saved_uuid)) = (&config.username, &config.uuid) {
            if saved_username == username {
                return saved_uuid.clone();
            }
        }
    }
    // 离线模式：UUID v3 (MD5) 基于 "OfflinePlayer:{username}"
    // 对于 uuid 1.4，使用 Uuid::new_v3() 方法
    Uuid::new_v3(
        &Uuid::NAMESPACE_DNS,
        format!("OfflinePlayer:{}", username).as_bytes(),
    )
    .to_string()
}

/// 启动 Minecraft 游戏
pub async fn launch_minecraft(
    options: LaunchOptions,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    let emit = |event: &str, msg: String| {
        let _ = window.emit(event, msg);
    };
    // 保存用户名和UUID到配置文件
    let uuid = generate_offline_uuid(&options.username);
    let mut config = load_config()?;
    config.username = Some(options.username.clone());
    config.uuid = Some(uuid.clone());
    save_config(&config)?;

    // 继续使用更新后的配置
    let game_dir = PathBuf::from(&config.game_dir);
    let version_dir = game_dir.join("versions").join(&options.version);
    let version_json_path = version_dir.join(format!("{}.json", &options.version));

    emit("log-debug", format!("尝试启动版本: {}", options.version));
    emit("log-debug", format!("游戏目录: {}", game_dir.display()));
    emit("log-debug", format!("版本目录: {}", version_dir.display()));
    emit(
        "log-debug",
        format!("版本JSON路径: {}", version_json_path.display()),
    );

    if !version_json_path.exists() {
        emit(
            "log-error",
            format!("版本JSON文件不存在: {}", version_json_path.display()),
        );
        return Err(LauncherError::Custom(format!(
            "版本 {} 的json文件不存在!",
            options.version
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
                // 父 json 不存在，停止合并
                break;
            }
            let parent_str = fs::read_to_string(&parent_json_path)?;
            let parent_json: serde_json::Value = serde_json::from_str(&parent_str)?;

            // 将父的缺失字段合并到当前 version_json（不覆盖已存在字段），对 libraries 做去重合并
            // 先处理 libraries
            if let Some(parent_libs) = parent_json.get("libraries").and_then(|v| v.as_array()) {
                let mut merged_libs: Vec<serde_json::Value> = Vec::new();
                let mut seen = std::collections::HashSet::new();

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

            // 合并 arguments.game：如果子级缺失则直接补充；如果子级存在则把父级中子级没有的项按父级顺序合并到子级前面（去重）
            if let Some(parent_args) = parent_json.get("arguments") {
                // 如果子级没有 arguments，则直接复制整个父级 arguments
                if version_json.get("arguments").is_none() {
                    version_json["arguments"] = parent_args.clone();
                } else {
                    // 尝试从父级获取 game 数组
                    let parent_game_opt =
                        parent_args.get("game").and_then(|g| g.as_array()).cloned();
                    if let Some(parent_game_arr) = parent_game_opt {
                        // 子级没有 game 数组 -> 直接使用父级的
                        if version_json
                            .get("arguments")
                            .and_then(|a| a.get("game"))
                            .is_none()
                        {
                            version_json["arguments"]["game"] =
                                serde_json::Value::Array(parent_game_arr);
                        } else if let Some(child_game_arr) = version_json
                            .get("arguments")
                            .and_then(|a| a.get("game"))
                            .and_then(|g| g.as_array())
                            .cloned()
                        {
                            // 子级存在 game 数组 -> 合并父级中子级没有的项，按父级顺序放在前面
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
                }
            } else if let Some(parent_mc_args) = parent_json.get("minecraftArguments") {
                // 父级使用旧式 minecraftArguments，转换为数组并合并到子级 game（如果子级没有则直接写入）
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
                    } else if let Some(child_game_arr) = version_json
                        .get("arguments")
                        .and_then(|a| a.get("game"))
                        .and_then(|g| g.as_array())
                        .cloned()
                    {
                        // 合并父级 minecraftArguments 的每一项（按顺序）到子级前面，避免重复
                        let mut merged: Vec<serde_json::Value> = Vec::new();
                        for p in parts {
                            if !child_game_arr.contains(&p) {
                                merged.push(p);
                            }
                        }
                        for c in child_game_arr {
                            merged.push(c);
                        }
                        version_json["arguments"]["game"] = serde_json::Value::Array(merged);
                    } else if version_json
                        .get("arguments")
                        .and_then(|a| a.get("game"))
                        .is_none()
                    {
                        // 子级存在 arguments 但没有 game
                        version_json["arguments"]["game"] = serde_json::Value::Array(parts);
                    }
                }
            }

            // 合并其他顶层缺失字段（不覆盖已有）
            if let Some(obj) = parent_json.as_object() {
                for (k, v) in obj.iter() {
                    if !version_json.get(k).is_some() {
                        version_json[k] = v.clone();
                    }
                }
            }

            // 处理下一层继承（如果父还有 inheritsFrom）
            if let Some(next_parent) = parent_json.get("inheritsFrom").and_then(|v| v.as_str()) {
                parent_id = next_parent.to_string();
            } else {
                break;
            }
        }
    }

    let (libraries_base_dir, assets_base_dir) =
        (game_dir.join("libraries"), game_dir.join("assets"));
    emit(
        "log-debug",
        format!("库文件目录: {}", libraries_base_dir.display()),
    );
    emit(
        "log-debug",
        format!("资源文件目录: {}", assets_base_dir.display()),
    );
    // 统一 OS 名称映射，macos -> osx
    let current_os = if std::env::consts::OS == "macos" {
        "osx"
    } else {
        std::env::consts::OS
    };

    // --- 1. 准备隔离目录 ---
    let natives_dir = version_dir.join("natives");

    // 创建隔离目录
    if config.version_isolation {
        let isolate_dirs = vec![
            ("saves", config.isolate_saves),
            ("resourcepacks", config.isolate_resourcepacks),
            ("logs", config.isolate_logs),
        ];

        for (dir_name, should_isolate) in isolate_dirs {
            let dir_path = version_dir.join(dir_name);
            if should_isolate && !dir_path.exists() {
                fs::create_dir_all(&dir_path)?;
            }
        }

        // 复制options.txt
        let options_src = game_dir.join("options.txt");
        let options_dst = version_dir.join("options.txt");
        if options_src.exists() && !options_dst.exists() {
            fs::copy(&options_src, &options_dst)?;
        }
    }
    emit(
        "log-debug",
        format!("Natives目录: {}", natives_dir.display()),
    );
    if natives_dir.exists() {
        emit(
            "log-debug",
            format!("清理旧的Natives目录: {}", natives_dir.display()),
        );
        fs::remove_dir_all(&natives_dir)?;
    }
    fs::create_dir_all(&natives_dir)?;

    if let Some(libraries) = version_json["libraries"].as_array() {
        for lib in libraries {
            if let Some(natives) = lib.get("natives") {
                emit("log-debug", format!("发现Natives库: {:?}", lib));
                if let Some(os_classifier) = natives.get(current_os) {
                    emit(
                        "log-debug",
                        format!(
                            "正在查找的OS分类器: {}",
                            os_classifier.as_str().unwrap_or("N/A")
                        ),
                    );
                    if let Some(artifact) = lib
                        .get("downloads")
                        .and_then(|d| d.get("classifiers"))
                        .and_then(|c| c.get(os_classifier.as_str().unwrap_or("")))
                    {
                        emit("log-debug", format!("Natives Artifact: {:?}", artifact));
                        let lib_path =
                            libraries_base_dir.join(artifact["path"].as_str().unwrap_or(""));
                        emit(
                            "log-debug",
                            format!("尝试解压Natives库: {}", lib_path.display()),
                        );
                        if !lib_path.exists() {
                            emit(
                                "log-error",
                                format!("Natives库文件不存在: {}", lib_path.display()),
                            );
                            return Err(LauncherError::Custom(format!(
                                "Natives库文件不存在: {}",
                                lib_path.display()
                            )));
                        }
                        let file = fs::File::open(&lib_path)?;
                        let mut archive = zip::ZipArchive::new(file)?;

                        for i in 0..archive.len() {
                            let mut file = archive.by_index(i)?;
                            // 只取条目最后的文件名，避免将库解压到嵌套路径中，确保所有本机库位于 natives 根目录
                            // 使用 owned String 来避免后续对 zip 条目进行可变借用时的借用冲突
                            let entry_name = file.name().to_string();
                            // 检查是否需要排除
                            if let Some(extract_rules) = lib.get("extract") {
                                if let Some(exclude) =
                                    extract_rules.get("exclude").and_then(|e| e.as_array())
                                {
                                    if exclude
                                        .iter()
                                        .any(|v| entry_name.starts_with(v.as_str().unwrap_or("")))
                                    {
                                        continue;
                                    }
                                }
                            }

                            // 跳过文件夹条目
                            if entry_name.ends_with('/') {
                                continue;
                            }

                            // 取出最后一段文件名，避免嵌套目录（例如 some/path/native.dll -> native.dll）
                            let file_stem = std::path::Path::new(&entry_name)
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or(entry_name.as_str());

                            let outpath = natives_dir.join(file_stem);

                            if let Some(p) = outpath.parent() {
                                if !p.exists() {
                                    fs::create_dir_all(&p)?;
                                }
                            }
                            let mut outfile = fs::File::create(&outpath)?;
                            io::copy(&mut file, &mut outfile)?;
                            emit(
                                "log-debug",
                                format!("解压Natives文件: {} -> {}", entry_name, outpath.display()),
                            );
                        }
                        // 列出 natives 目录内容，便于排查缺失的本机库
                        if natives_dir.exists() {
                            if let Ok(entries) = fs::read_dir(&natives_dir) {
                                let mut names = vec![];
                                for e in entries.flatten() {
                                    if let Ok(fname) = e.file_name().into_string() {
                                        names.push(fname);
                                    }
                                }
                                emit(
                                    "log-debug",
                                    format!("natives 目录内容: [{}]", names.join(", ")),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // --- 2. 构建 Classpath ---
    let mut classpath = vec![];
    if let Some(libraries) = version_json["libraries"].as_array() {
        for lib in libraries {
            if lib.get("natives").is_some() {
                continue;
            } // 跳过Natives库

            if let Some(rules) = lib.get("rules").and_then(|r| r.as_array()) {
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
                if !allowed {
                    continue;
                }
            }

            if let Some(path) = lib["downloads"]
                .get("artifact")
                .and_then(|a| a.get("path"))
                .and_then(|p| p.as_str())
            {
                let lib_path = libraries_base_dir.join(path);
                emit(
                    "log-debug",
                    format!("添加到Classpath的库: {}", lib_path.display()),
                );
                if !lib_path.exists() {
                    emit(
                        "log-error",
                        format!("Classpath中的库文件不存在: {}", lib_path.display()),
                    );
                    return Err(LauncherError::Custom(format!(
                        "Classpath中的库文件不存在: {}",
                        lib_path.display()
                    )));
                }
                classpath.push(lib_path);
            } else if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                // 回退：根据 maven 坐标 group:artifact:version 构建本地路径
                let parts: Vec<&str> = name.split(':').collect();
                if parts.len() >= 3 {
                    let group = parts[0].replace('.', "/");
                    let artifact = parts[1];
                    let version = parts[2];
                    let candidate = libraries_base_dir
                        .join(&group)
                        .join(artifact)
                        .join(version)
                        .join(format!("{}-{}.jar", artifact, version));
                    emit(
                        "log-debug",
                        format!("尝试回退解析库路径: {}", candidate.display()),
                    );
                    if candidate.exists() {
                        classpath.push(candidate);
                    } else {
                        emit(
                            "log-error",
                            format!(
                                "库文件缺失（maven 回退也未找到）: name={}，期望路径: {}",
                                name,
                                candidate.display()
                            ),
                        );
                    }
                } else {
                    emit(
                        "log-error",
                        format!(
                            "库条目缺少 downloads.artifact.path，且 name 非法: {:?}",
                            lib
                        ),
                    );
                }
            }
        }
    }
    let main_game_jar_path = version_dir.join(format!("{}.jar", &options.version));
    emit(
        "log-debug",
        format!("主游戏JAR路径: {}", main_game_jar_path.display()),
    );
    if !main_game_jar_path.exists() {
        emit(
            "log-error",
            format!("主游戏JAR文件不存在: {}", main_game_jar_path.display()),
        );
        return Err(LauncherError::Custom(format!(
            "主游戏JAR文件不存在: {}",
            main_game_jar_path.display()
        )));
    }
    classpath.push(main_game_jar_path);

    // --- 3. 获取主类和参数 ---
    let main_class = version_json["mainClass"]
        .as_str()
        .ok_or_else(|| LauncherError::Custom("无法在json中找到mainClass".to_string()))?;
    let assets_index = version_json["assetIndex"]["id"]
        .as_str()
        .unwrap_or(&options.version);
    let assets_dir = assets_base_dir;

    // 预检：当 mainClass 为 LaunchWrapper 时，确保 Classpath 中包含 launchwrapper 库；若缺失尝试自动自愈
    if main_class == "net.minecraft.launchwrapper.Launch" {
        let has_launchwrapper = classpath.iter().any(|p| {
            let s = p.to_string_lossy();
            s.contains("net/minecraft/launchwrapper") || s.contains("launchwrapper-")
        });
        if !has_launchwrapper {
            emit(
                "log-debug",
                "预检：Classpath 未包含 LaunchWrapper，尝试在 libraries 目录自动查找".to_string(),
            );

            // 递归扫描 libraries_base_dir，寻找任意 launchwrapper-*.jar
            fn find_launchwrapper_jar(dir: &std::path::Path) -> Option<std::path::PathBuf> {
                if let Ok(read_dir) = std::fs::read_dir(dir) {
                    for entry in read_dir.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(found) = find_launchwrapper_jar(&path) {
                                return Some(found);
                            }
                        } else {
                            let name = entry.file_name().to_string_lossy().to_lowercase();
                            if name.starts_with("launchwrapper-") && name.ends_with(".jar") {
                                return Some(path);
                            }
                        }
                    }
                }
                None
            }

            if let Some(jar) = find_launchwrapper_jar(&libraries_base_dir) {
                emit(
                    "log-debug",
                    format!(
                        "自动自愈：发现 LaunchWrapper 库，加入 Classpath: {}",
                        jar.display()
                    ),
                );
                classpath.push(jar);
            } else {
                emit(
                    "log-error",
                    "预检失败：Classpath 未包含 LaunchWrapper 库（net.minecraft:launchwrapper），且在 libraries 中未找到可用 JAR".to_string(),
                );
                return Err(LauncherError::Custom(
                    "预检失败：缺少 LaunchWrapper 库。请重新运行 Forge 安装或手动补齐 libraries/net/minecraft/launchwrapper/* 并在版本 JSON 的 libraries 中声明 net.minecraft:launchwrapper:1.12（且包含 downloads.artifact.path）".to_string()
                ));
            }
        }
    }

    // 预检：补齐 jopt-simple（旧版 Forge/FML 需要）
    if main_class == "net.minecraft.launchwrapper.Launch" {
        let has_jopt = classpath.iter().any(|p| {
            let s = p.to_string_lossy();
            s.contains("jopt-simple") || s.contains("joptsimple")
        });
        if !has_jopt {
            emit(
                "log-debug",
                "预检：Classpath 未包含 jopt-simple，尝试在 libraries 目录自动查找".to_string(),
            );

            fn find_jopt_jar(dir: &std::path::Path) -> Option<std::path::PathBuf> {
                if let Ok(read_dir) = std::fs::read_dir(dir) {
                    for entry in read_dir.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(found) = find_jopt_jar(&path) {
                                return Some(found);
                            }
                        } else {
                            let name = entry.file_name().to_string_lossy().to_lowercase();
                            if (name.contains("jopt-simple") || name.contains("joptsimple"))
                                && name.ends_with(".jar")
                            {
                                return Some(path);
                            }
                        }
                    }
                }
                None
            }

            if let Some(jar) = find_jopt_jar(&libraries_base_dir) {
                emit(
                    "log-debug",
                    format!(
                        "自动自愈：发现 jopt-simple 库，加入 Classpath: {}",
                        jar.display()
                    ),
                );
                classpath.push(jar);
            } else {
                emit(
                    "log-error",
                    "预检失败：缺少 jopt-simple 库（net.sf.jopt-simple:jopt-simple）。".to_string(),
                );
                // 保留继续启动以便输出更直观的错误；如需严格可在此返回 Err
            }
        }
    }

    // 预检：补齐 Forge/FML（确保 FMLTweaker 可加载）
    if main_class == "net.minecraft.launchwrapper.Launch" {
        let has_forge_fml = classpath.iter().any(|p| {
            let s = p.to_string_lossy().to_lowercase();
            s.contains("minecraftforge")
                || s.contains("forge-")
                || s.contains("/fml/")
                || s.contains("\\fml\\")
        });

        if !has_forge_fml {
            emit(
                "log-debug",
                "预检：Classpath 未包含 Forge/FML，尝试在 libraries 目录自动查找 forge JAR"
                    .to_string(),
            );

            fn find_forge_jar(dir: &std::path::Path) -> Option<std::path::PathBuf> {
                if let Ok(read_dir) = std::fs::read_dir(dir) {
                    for entry in read_dir.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(found) = find_forge_jar(&path) {
                                return Some(found);
                            }
                        } else {
                            let name = entry.file_name().to_string_lossy().to_lowercase();
                            let full = path.to_string_lossy().to_lowercase();
                            // 兼容常见旧版命名与目录结构
                            if (name.starts_with("forge-")
                                || full.contains("net\\minecraftforge\\forge")
                                || full.contains("net/minecraftforge/forge"))
                                && name.ends_with(".jar")
                            {
                                return Some(path);
                            }
                        }
                    }
                }
                None
            }

            if let Some(jar) = find_forge_jar(&libraries_base_dir) {
                emit(
                    "log-debug",
                    format!("自动自愈：发现 Forge 库，加入 Classpath: {}", jar.display()),
                );
                classpath.push(jar);
            } else {
                emit(
                    "log-error",
                    "预检失败：未在 libraries 中找到 Forge/FML 相关 JAR，可能安装不完整。"
                        .to_string(),
                );
                // 不立刻返回，让后续错误输出更明确
            }
        }
    }

    // 替换通用占位符的辅助函数
    let replace_placeholders = |arg: &str| -> String {
        let actual_game_dir = if config.version_isolation {
            version_dir.to_string_lossy().to_string()
        } else {
            game_dir.to_string_lossy().to_string()
        };

        arg.replace("${auth_player_name}", &options.username)
            .replace("${version_name}", &options.version)
            .replace("${game_directory}", &actual_game_dir)
            .replace("${assets_root}", &assets_dir.to_string_lossy().to_string())
            .replace("${assets_index_name}", assets_index)
            .replace("${auth_uuid}", &uuid)
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
        if let Some(jvm) = arguments["jvm"].as_array() {
            for arg in jvm {
                if let Some(s) = arg.as_str() {
                    jvm_args.push(replace_placeholders(s));
                } else if let Some(obj) = arg.as_object() {
                    // 处理带规则的JVM参数
                    let mut allowed = true;
                    if let Some(rules) = obj.get("rules").and_then(|r| r.as_array()) {
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
                    }
                    if allowed {
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
        }
        if let Some(game) = arguments["game"].as_array() {
            for arg in game {
                if let Some(s) = arg.as_str() {
                    game_args_vec.push(replace_placeholders(s));
                }
            }
        }
    }
    // 处理旧版 `minecraftArguments` 格式
    else if let Some(mc_args) = version_json["minecraftArguments"].as_str() {
        game_args_vec = mc_args.split(' ').map(replace_placeholders).collect();
    }

    // 若缺少 tweakClass，基于版本自动补齐（仅在 LaunchWrapper 主类下，且检测到 Forge/FML 存在时）
    let has_tweak_class_flag = game_args_vec.iter().any(|a| a == "--tweakClass");
    if main_class == "net.minecraft.launchwrapper.Launch" && !has_tweak_class_flag {
        // 检测是否存在 Forge/FML 相关库（双重判断：libraries 声明 + 已构建的 classpath 路径）
        let forge_in_libraries = version_json
            .get("libraries")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().any(|lib| {
                    lib.get("name")
                        .and_then(|n| n.as_str())
                        .map(|name| {
                            name.contains("net.minecraftforge") || name.contains("cpw.mods")
                        })
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

        let has_forge = forge_in_libraries || forge_in_classpath;

        if has_forge {
            // 从版本 id 推断基础 MC 版本（通常形如 "1.12.2-forge-..."）
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
            // 插入到参数最前，确保优先被处理
            game_args_vec.insert(0, tweaker.to_string());
            game_args_vec.insert(0, "--tweakClass".to_string());
        } else {
            emit(
                "log-debug",
                "跳过自动补齐 tweakClass：未检测到 Forge/FML 库，避免 ClassNotFound".to_string(),
            );
        }
    }

    // --- 4. 组装Java启动参数 ---
    let java_path = {
        // 1. 首先尝试使用配置中的Java路径
        if let Some(config_path) = config.java_path {
            if PathBuf::from(&config_path).exists() {
                config_path
            } else {
                // 2. 如果配置路径不存在，尝试在PATH中查找
                if Command::new("java").arg("-version").output().is_ok() {
                    "java".to_string()
                } else {
                    return Err(LauncherError::Custom(format!(
                        "配置的Java路径不存在且系统PATH中未找到Java: {}",
                        config_path
                    )));
                }
            }
        } else {
            // 3. 如果未配置路径，尝试在PATH中查找
            if Command::new("java").arg("-version").output().is_ok() {
                "java".to_string()
            } else {
                return Err(LauncherError::Custom(
                    "未配置Java路径且系统PATH中未找到Java".to_string(),
                ));
            }
        }
    };
    emit("log-debug", format!("使用的Java路径: {}", java_path));

    // 在 JVM 启动参数中设置内存和本机库路径（同时设置 org.lwjgl.librarypath）
    let lwjgl_lib_path = natives_dir.to_string_lossy().to_string();
    let mut final_args = vec![
        format!("-Xmx{}M", options.memory.unwrap_or(2048)),
        format!("-Djava.library.path={}", lwjgl_lib_path),
        format!("-Dorg.lwjgl.librarypath={}", lwjgl_lib_path),
        "-Dfile.encoding=UTF-8".to_string(),
        // 解决旧版 Forge (LWJGL 2) 在 Java 8 上由于 OpenAL 引发的 UnsatisfiedLinkError
        "-Dorg.lwjgl.openal.mapping.use=false".to_string(),
    ];
    final_args.extend(jvm_args);

    // 在可能动态补充了库（如 LaunchWrapper）之后，重新计算最终 Classpath
    let classpath_str = classpath
        .iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join(if cfg!(windows) { ";" } else { ":" });
    emit("log-debug", format!("最终Classpath: {}", classpath_str));

    final_args.push("-cp".to_string());
    final_args.push(classpath_str);
    final_args.push(main_class.to_string());
    final_args.extend(game_args_vec);

    // --- 5. 启动游戏 ---
    let mut command = Command::new(&java_path);
    command.args(&final_args);
    command.current_dir(if config.version_isolation {
        &version_dir
    } else {
        &game_dir
    });

    // 在Windows上隐藏命令行窗口
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW = 0x08000000
        // 使用这个标志可以隐藏命令行窗口
        command.creation_flags(0x08000000);
    }

    // 排查加载问题
    emit(
        "log-debug",
        format!("java.library.path: {}", lwjgl_lib_path),
    );
    emit(
        "log-debug",
        format!("org.lwjgl.librarypath: {}", lwjgl_lib_path),
    );

    // 列出 natives 目录下的文件（包含绝对路径和大小）
    if natives_dir.exists() {
        if let Ok(entries) = fs::read_dir(&natives_dir) {
            let mut details = vec![];
            for e in entries.flatten() {
                let p = e.path();
                if let Ok(meta) = fs::metadata(&p) {
                    details.push(format!("{} ({} bytes)", p.display(), meta.len()));
                } else {
                    details.push(format!("{} (metadata error)", p.display()));
                }
            }
            emit(
                "log-debug",
                format!("natives 目录详细内容: [{}]", details.join(", ")),
            );
        }
    }

    emit("log-debug", format!("最终启动命令: {:?}", command));
    window.emit("launch-command", format!("{:?}", command))?;

    // 启动游戏进程但不等待它结束
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    emit("log-debug", format!("游戏已启动，PID: {:?}", child.id()));

    // 发送游戏启动成功的事件到前端
    window.emit(
        "minecraft-launched",
        format!("游戏已启动，PID: {}", child.id()),
    )?;

    // 在后台线程中监控游戏进程输出并在退出后收集 stdout/stderr，不阻塞主线程
    let window_clone = window.clone();
    std::thread::spawn(move || {
        match child.wait_with_output() {
            Ok(output) => {
                let status = output.status;

                // Emit captured stdout if present
                if !output.stdout.is_empty() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
                    let _ = window_clone.emit("log-debug", format!("游戏 stdout:\n{}", stdout_str));
                }

                // Emit captured stderr if present (treat as error-level)
                if !output.stderr.is_empty() {
                    let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
                    let _ = window_clone.emit("log-error", format!("游戏 stderr:\n{}", stderr_str));
                }

                let _ = window_clone.emit(
                    "log-debug",
                    format!("游戏进程退出，状态码: {:?}", status.code()),
                );

                // 如果游戏以非零退出码退出，发送错误事件到前端，包含 stdout/stderr 输出
                if status.code().unwrap_or(-1) != 0 {
                    let mut combined = String::new();
                    if !output.stdout.is_empty() {
                        combined.push_str("[stdout]\n");
                        combined.push_str(&String::from_utf8_lossy(&output.stdout));
                        combined.push_str("\n");
                    }
                    if !output.stderr.is_empty() {
                        combined.push_str("[stderr]\n");
                        combined.push_str(&String::from_utf8_lossy(&output.stderr));
                    }
                    let _ = window_clone.emit(
                        "minecraft-error",
                        format!(
                            "游戏以非零退出 (code={:?})，输出:\n{}",
                            status.code(),
                            combined
                        ),
                    );
                }

                // 发送游戏退出事件到前端，包含退出码
                let _ = window_clone.emit(
                    "minecraft-exited",
                    format!("游戏已退出，状态码: {:?}", status.code()),
                );
            }
            Err(e) => {
                let _ = window_clone.emit("log-error", format!("监控游戏进程时出错: {}", e));
                // 发送错误事件到前端
                let _ = window_clone.emit("minecraft-error", format!("监控游戏进程时出错: {}", e));
            }
        }
    });

    emit("log-debug", "游戏成功启动".to_string());
    Ok(())
}
