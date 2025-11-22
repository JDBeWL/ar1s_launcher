pub mod controllers;
mod errors;
mod models;
pub mod services;
pub mod utils;
pub use errors::LauncherError;
pub use models::*;
pub use services::config::{load_config, save_config};
pub use services::launcher::launch_minecraft;
use utils::logger::setup_logger;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            controllers::download_controller::get_versions,
            controllers::download_controller::download_version,
            controllers::launcher_controller::launch_minecraft,
            controllers::config_controller::get_config,
            controllers::config_controller::get_game_dir,
            controllers::config_controller::get_game_dir_info,
            controllers::config_controller::set_game_dir,
            controllers::config_controller::select_game_dir,
            controllers::config_controller::set_version_isolation,
            controllers::java_controller::find_java_installations_command,
            controllers::java_controller::set_java_path_command,
            controllers::config_controller::load_config_key,
            controllers::config_controller::save_config_key,
            controllers::java_controller::validate_java_path,
            controllers::config_controller::get_download_threads,
            controllers::config_controller::set_download_threads,
            controllers::config_controller::validate_version_files,
            controllers::auth_controller::get_saved_username,
            controllers::auth_controller::set_saved_username,
            controllers::auth_controller::get_saved_uuid,
            controllers::auth_controller::set_saved_uuid,
            controllers::config_controller::get_total_memory,
            controllers::config_controller::get_memory_stats,
            controllers::config_controller::recommend_memory,
            controllers::config_controller::validate_memory_setting,
            controllers::config_controller::check_memory_warning,
            controllers::config_controller::get_auto_memory_config,
            controllers::config_controller::set_auto_memory_enabled,
            controllers::config_controller::auto_set_memory,
            controllers::config_controller::analyze_memory_efficiency,
            controllers::instance_controller::create_instance,
            controllers::instance_controller::get_instances,
            controllers::instance_controller::delete_instance,
            controllers::instance_controller::rename_instance,
            controllers::instance_controller::open_instance_folder,
            controllers::instance_controller::launch_instance,
            controllers::forge_controller::get_forge_versions,
            controllers::modpack_controller::search_modrinth_modpacks,
            controllers::modpack_controller::get_modrinth_modpack_versions,
            controllers::modpack_controller::install_modrinth_modpack
        ])
        .setup(|_| {
            log::info!("[DEBUG] Tauri应用初始化完成");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
