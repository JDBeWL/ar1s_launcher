// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use fern::Dispatch;
use log::LevelFilter;
use std::fs;

fn setup_logger() -> Result<(), fern::InitError> {
    // 创建日志目录
    log::info!("[DEBUG] 创建日志目录");
    fs::create_dir_all("logs")?;

    let log_file = format!(
        "logs/ar1s_launcher_{}.log",
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    );

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file(&log_file)?)
        .apply()?;

    Ok(())
}

fn main() {
    // 初始化日志记录器
    if let Err(e) = setup_logger() {
        eprintln!("Error setting up logger: {}", e);
    }

    log::info!("[DEBUG] 程序启动");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            ar1s_launcher_lib::controllers::download_controller::get_versions,
            ar1s_launcher_lib::controllers::download_controller::download_version,
            ar1s_launcher_lib::controllers::launcher_controller::launch_minecraft,
            ar1s_launcher_lib::controllers::config_controller::get_config,
            ar1s_launcher_lib::controllers::config_controller::get_game_dir,
            ar1s_launcher_lib::controllers::config_controller::get_game_dir_info,
            ar1s_launcher_lib::controllers::config_controller::set_game_dir,
            ar1s_launcher_lib::controllers::config_controller::select_game_dir,
            ar1s_launcher_lib::controllers::config_controller::set_version_isolation,
            ar1s_launcher_lib::controllers::java_controller::find_java_installations_command,
            ar1s_launcher_lib::controllers::java_controller::set_java_path_command,
            ar1s_launcher_lib::controllers::config_controller::load_config_key,
            ar1s_launcher_lib::controllers::config_controller::save_config_key,
            ar1s_launcher_lib::controllers::java_controller::validate_java_path,
            ar1s_launcher_lib::controllers::config_controller::get_download_threads,
            ar1s_launcher_lib::controllers::config_controller::set_download_threads,
            ar1s_launcher_lib::controllers::config_controller::validate_version_files,
            ar1s_launcher_lib::controllers::auth_controller::get_saved_username,
            ar1s_launcher_lib::controllers::auth_controller::set_saved_username,
            ar1s_launcher_lib::controllers::auth_controller::get_saved_uuid,
            ar1s_launcher_lib::controllers::auth_controller::set_saved_uuid,
            ar1s_launcher_lib::controllers::config_controller::get_total_memory,
            ar1s_launcher_lib::controllers::config_controller::get_memory_stats,
            ar1s_launcher_lib::controllers::config_controller::recommend_memory,
            ar1s_launcher_lib::controllers::config_controller::validate_memory_setting,
            ar1s_launcher_lib::controllers::config_controller::check_memory_warning,
            ar1s_launcher_lib::controllers::config_controller::get_auto_memory_config,
            ar1s_launcher_lib::controllers::config_controller::set_auto_memory_enabled,
            ar1s_launcher_lib::controllers::config_controller::auto_set_memory,
            ar1s_launcher_lib::controllers::config_controller::analyze_memory_efficiency,
            ar1s_launcher_lib::controllers::instance_controller::create_instance,
            ar1s_launcher_lib::controllers::instance_controller::get_instances,
            ar1s_launcher_lib::controllers::instance_controller::delete_instance,
            ar1s_launcher_lib::controllers::instance_controller::rename_instance,
            ar1s_launcher_lib::controllers::instance_controller::open_instance_folder,
            ar1s_launcher_lib::controllers::forge_controller::get_forge_versions
        ])
        .setup(|_| {
            log::info!("[DEBUG] Tauri应用初始化完成");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
