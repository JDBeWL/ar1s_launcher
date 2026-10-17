//! Minecraft 启动流程模块
//!
//! 串起 token 刷新、隔离目录、natives 解压、classpath、参数拼装、Java 匹配与进程监控

mod arguments;
mod classpath;
pub mod crash_diagnosis;
mod isolation;
pub mod java;
pub mod java_requirement;
mod natives;
mod process;
mod version_json;

use crate::errors::LauncherError;
use crate::models::LaunchOptions;
use crate::services::config::{
    load_config, save_config, set_last_selected_version, update_instance_last_played,
};
use crate::services::memory::{is_memory_setting_safe, optimize_jvm_memory_args};
use log::{info, warn};
use tauri::Emitter;

pub use classpath::find_library_jar;

pub async fn launch_minecraft(
    options: LaunchOptions,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    let emit = |event: &str, msg: String| {
        let _ = window.emit(event, msg);
    };

    let mut options = options;

    if options.auth_type.as_deref() == Some("microsoft") && options.access_token.is_none() {
        let cfg = load_config()?;
        if cfg.auth_type == crate::models::AuthType::Microsoft {
            let expires_at = cfg.ms_expires_at.unwrap_or(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            if now >= expires_at - 60 {
                if let Some(refresh_token) = cfg.ms_refresh_token.clone() {
                    match crate::services::microsoft_auth::refresh_and_authenticate(&refresh_token)
                        .await
                    {
                        Ok(auth_result) => {
                            let mut save_cfg = (*load_config()?).clone();
                            save_cfg.ms_access_token = Some(auth_result.access_token.clone());
                            save_cfg.ms_refresh_token = Some(auth_result.refresh_token.clone());
                            save_cfg.ms_expires_at = Some(auth_result.expires_at);
                            save_cfg.username = Some(auth_result.username.clone());
                            save_cfg.uuid = Some(auth_result.uuid.clone());
                            save_cfg.skin_url = auth_result.skin_url.clone();
                            save_config(&save_cfg)?;

                            options.access_token = Some(auth_result.access_token);
                            if options.uuid.is_none() {
                                options.uuid = Some(auth_result.uuid);
                            }
                            info!("Microsoft token 刷新成功");
                        }
                        Err(e) => {
                            warn!("Microsoft token 刷新失败: {}, 将使用旧 token", e);
                            options.access_token = cfg.ms_access_token.clone();
                        }
                    }
                } else {
                    warn!("未找到 refresh_token，无法刷新 Microsoft token");
                    options.access_token = cfg.ms_access_token.clone();
                }
            } else {
                options.access_token = cfg.ms_access_token.clone();
            }

            if options.uuid.is_none() {
                options.uuid = cfg.uuid.clone();
            }
        }
    }

    let uuid = if options.auth_type.as_deref() == Some("microsoft") {
        options
            .uuid
            .clone()
            .unwrap_or_else(|| java::generate_offline_uuid(&options.username))
    } else {
        java::generate_offline_uuid(&options.username)
    };
    let mut config = (*load_config()?).clone();
    config.username = Some(options.username.clone());
    config.uuid = Some(uuid.clone());
    save_config(&config)?;

    let _ = update_instance_last_played(&options.version);
    let _ = set_last_selected_version(&options.version);

    let game_dir = java::resolve_game_dir(&config.game_dir);
    let version_dir = game_dir.join("versions").join(&options.version);

    emit("log-debug", format!("尝试启动版本: {}", options.version));
    emit("log-debug", format!("游戏目录: {}", game_dir.display()));

    let version_json = version_json::load_and_merge_version_json(&game_dir, &options.version)?;

    let libraries_base_dir = game_dir.join("libraries");
    let assets_base_dir = game_dir.join("assets");

    emit(
        "log-debug",
        format!("库文件目录: {}", libraries_base_dir.display()),
    );
    emit(
        "log-debug",
        format!("资源文件目录: {}", assets_base_dir.display()),
    );

    // Minecraft 版本 JSON 用 "osx" 而非 Rust 的 "macos"
    let current_os = if std::env::consts::OS == "macos" {
        "osx"
    } else {
        std::env::consts::OS
    };

    isolation::prepare_isolated_version_directory(&config, &game_dir, &version_dir)?;
    let natives_dir = natives::extract_natives(
        &version_json,
        &version_dir,
        &libraries_base_dir,
        current_os,
        &emit,
    )?;

    let mut classpath = classpath::build_classpath(
        &version_json,
        &libraries_base_dir,
        &version_dir,
        &options.version,
        &emit,
    )?;

    let main_class = version_json["mainClass"]
        .as_str()
        .ok_or_else(|| LauncherError::Custom("无法在json中找到mainClass".to_string()))?;

    if main_class == "net.minecraft.launchwrapper.Launch" {
        classpath::precheck_launchwrapper_libraries(&mut classpath, &libraries_base_dir, &emit)?;
    }

    let assets_index = version_json["assetIndex"]["id"]
        .as_str()
        .unwrap_or(&options.version);

    let (jvm_args, game_args_vec) = arguments::build_arguments(
        &version_json,
        &config,
        &options,
        &uuid,
        &version_dir,
        &game_dir,
        &assets_base_dir,
        assets_index,
        &classpath,
        &emit,
    );

    let profile = java::profile_for(&options.version, &game_dir);

    let mut java_path = if let Some(ref override_path) = options.override_java_path {
        emit(
            "log-info",
            format!("使用临时指定的 Java: {}", override_path),
        );
        override_path.clone()
    } else if config.auto_match_java {
        if let Some(matched) = java::auto_match_java(&profile) {
            emit(
                "log-info",
                format!("自动匹配 Java: {} (版本 {})", matched.path, matched.version),
            );
            if let Some(ref warning) = matched.warning {
                emit("log-warning", warning.clone());
            }
            matched.path
        } else {
            emit(
                "log-warning",
                "自动匹配 Java 失败，使用配置中的 Java 路径".to_string(),
            );
            java::resolve_java_path(&config)?
        }
    } else {
        java::resolve_java_path(&config)?
    };

    let mut java_version = java::detect_java_version(&java_path);

    // 三条分支选出来的 Java 都要过同一道判据；版本 JSON 读不到时不硬拦
    if profile.resolved
        && let Some(ver) = java_version
    {
        if !java_requirement::java_allowed(&profile.requirement, ver) {
            let candidates = java::collect_candidates(&config);
            match java::choose_java(&profile.requirement, &candidates) {
                Some(better) => {
                    emit(
                        "log-warning",
                        format!(
                            "{} 实例要求 Java {}{}，当前 Java {} 不满足，已换成 Java {} ({})",
                            profile.loader.label(),
                            profile.requirement.min,
                            match profile.requirement.hard_max {
                                Some(max) => format!("~{}", max),
                                None => "+".to_string(),
                            },
                            ver,
                            better.version,
                            better.path
                        ),
                    );
                    java_path = better.path;
                    java_version = Some(better.version);
                }
                None => {
                    let report = java_requirement::build_report(
                        &profile,
                        Some((java_path.as_str(), ver)),
                        &candidates,
                    );
                    // 判据没覆盖全，所以这不是否决而是确认：用户看过报告坚持要起，就带着风险起
                    if options.force_java_mismatch {
                        emit(
                            "log-warning",
                            format!("已按用户确认强制启动，风险自负：\n{}", report),
                        );
                    } else {
                        emit("log-error", report.clone());
                        return Err(LauncherError::Custom(report));
                    }
                }
            }
        }
    }

    emit("log-debug", format!("使用的Java路径: {}", java_path));
    if let Some(v) = java_version {
        emit("log-debug", format!("检测到Java版本: {}", v));
    } else {
        emit(
            "log-warning",
            "无法检测Java版本，将使用默认JVM参数".to_string(),
        );
    }

    let lwjgl_lib_path = natives_dir.to_string_lossy().to_string();
    let memory_mb = options.memory.unwrap_or(2048);

    if let Err(e) = is_memory_setting_safe(memory_mb) {
        emit("log-warning", format!("内存设置警告: {}", e));
    }

    // 生成 JVM 参数：默认只有 G1GC 优化，其余按用户在高级选项里的开关追加
    let mut final_args = optimize_jvm_memory_args(memory_mb, java_version, &config.jvm_advanced);

    final_args.extend([
        format!("-Djava.library.path={}", lwjgl_lib_path),
        format!("-Dorg.lwjgl.librarypath={}", lwjgl_lib_path),
        "-Dfile.encoding=UTF-8".to_string(),
        "-Dorg.lwjgl.openal.mapping.use=false".to_string(),
    ]);
    final_args.extend(jvm_args);

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

    // 版本 JSON 的 has_custom_resolution 规则可能已注入 --width/--height；Forge 只取第一个 --width，重复传参会让用户设置被占位符挤掉
    let has_arg = |flag: &str| final_args.iter().any(|a| a == flag);
    let width_present = has_arg("--width");
    let height_present = has_arg("--height");

    if !width_present {
        if let Some(width) = options.window_width.or(config.window_width) {
            final_args.push("--width".to_string());
            final_args.push(width.to_string());
        }
    }
    if !height_present {
        if let Some(height) = options.window_height.or(config.window_height) {
            final_args.push("--height".to_string());
            final_args.push(height.to_string());
        }
    }

    // 未替换的 ${...} 占位符会让游戏直接崩溃，这里提前告警
    if let Some(unresolved) = final_args.iter().find(|a| a.contains("${")) {
        emit(
            "log-warning",
            format!("启动参数中检测到未替换的占位符: {}", unresolved),
        );
    }
    if options.fullscreen.unwrap_or(config.fullscreen) {
        final_args.push("--fullscreen".to_string());
    }

    let working_dir = if config.version_isolation {
        version_dir
    } else {
        game_dir
    };

    if let Some(ref lang) = config.language {
        if let Err(e) = crate::utils::minecraft::set_game_language(&working_dir, lang) {
            emit("log-warning", format!("无法设置游戏语言: {}", e));
        }
    }

    let mods_root = java_requirement::mods_dir(&profile.game_dir, &options.version);
    let launch_ctx = crash_diagnosis::LaunchContext {
        version_id: options.version.clone(),
        loader: profile.loader.label().to_string(),
        declared_java: profile.declared_java,
        used_java_path: java_path.clone(),
        used_java_version: java_version,
        memory_mb,
        mod_files: java_requirement::list_mod_files(&mods_root),
        runtime_secs: None,
    };

    process::spawn_and_monitor_process(&java_path, final_args, &working_dir, launch_ctx, window)
}
