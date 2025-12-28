//! Minecraft 游戏启动器模块
//!
//! 该模块负责启动 Minecraft 游戏，包括：
//! - 版本 JSON 加载和合并
//! - Classpath 构建
//! - Natives 解压
//! - JVM 和游戏参数构建
//! - 进程启动和监控

mod arguments;
mod classpath;
mod isolation;
mod java;
mod natives;
mod process;
mod version_json;

use crate::errors::LauncherError;
use crate::models::LaunchOptions;
use crate::services::config::{load_config, save_config};
use crate::services::memory::{is_memory_setting_safe, optimize_jvm_memory_args};
use std::path::PathBuf;
use tauri::Emitter;

pub use classpath::find_library_jar;

/// 启动 Minecraft 游戏
pub async fn launch_minecraft(
    options: LaunchOptions,
    window: tauri::Window,
) -> Result<(), LauncherError> {
    let emit = |event: &str, msg: String| {
        let _ = window.emit(event, msg);
    };

    // 保存用户名和 UUID 到配置文件
    let uuid = java::generate_offline_uuid(&options.username);
    let mut config = load_config()?;
    config.username = Some(options.username.clone());
    config.uuid = Some(uuid.clone());
    save_config(&config)?;

    // 设置路径
    let game_dir = PathBuf::from(&config.game_dir);
    let version_dir = game_dir.join("versions").join(&options.version);

    emit("log-debug", format!("尝试启动版本: {}", options.version));
    emit("log-debug", format!("游戏目录: {}", game_dir.display()));

    // 加载版本 JSON
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

    // 统一 OS 名称映射
    let current_os = if std::env::consts::OS == "macos" {
        "osx"
    } else {
        std::env::consts::OS
    };

    // 1. 准备隔离和 Natives 目录
    isolation::prepare_isolated_version_directory(&config, &game_dir, &version_dir)?;
    let natives_dir = natives::extract_natives(
        &version_json,
        &version_dir,
        &libraries_base_dir,
        current_os,
        &emit,
    )?;

    // 2. 构建 Classpath
    let mut classpath = classpath::build_classpath(
        &version_json,
        &libraries_base_dir,
        &version_dir,
        &options.version,
        current_os,
        &emit,
    )?;

    // 3. 获取主类并执行库预检
    let main_class = version_json["mainClass"]
        .as_str()
        .ok_or_else(|| LauncherError::Custom("无法在json中找到mainClass".to_string()))?;

    if main_class == "net.minecraft.launchwrapper.Launch" {
        classpath::precheck_launchwrapper_libraries(&mut classpath, &libraries_base_dir, &emit)?;
    }

    // 4. 构建参数
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
        current_os,
        &classpath,
        &emit,
    );

    // 5. 组装 Java 启动参数
    let java_path = java::resolve_java_path(&config)?;
    emit("log-debug", format!("使用的Java路径: {}", java_path));

    let lwjgl_lib_path = natives_dir.to_string_lossy().to_string();
    let memory_mb = options.memory.unwrap_or(2048);

    // 检查内存设置是否安全
    if let Err(e) = is_memory_setting_safe(memory_mb) {
        emit("log-warning", format!("内存设置警告: {}", e));
    }

    // 生成优化的 JVM 内存参数
    let mut final_args = optimize_jvm_memory_args(memory_mb, &options.version);

    // 添加其他必要的 JVM 参数
    final_args.extend([
        format!("-Djava.library.path={}", lwjgl_lib_path),
        format!("-Dorg.lwjgl.librarypath={}", lwjgl_lib_path),
        "-Dfile.encoding=UTF-8".to_string(),
        "-Dorg.lwjgl.openal.mapping.use=false".to_string(),
    ]);
    final_args.extend(jvm_args);

    // 构建 Classpath 字符串
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

    // 6. 启动游戏
    let working_dir = if config.version_isolation {
        version_dir
    } else {
        game_dir
    };

    process::spawn_and_monitor_process(&java_path, final_args, &working_dir, window)
}
