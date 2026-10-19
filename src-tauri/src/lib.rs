//! Ar1s Launcher 的 Tauri 后端
//!
//! crate 根：声明命令层与业务层，并把各 controller 注册成 Tauri command

pub mod controllers;
mod errors;
mod models;
pub mod services;
pub mod utils;
pub use errors::LauncherError;
pub use services::config::{load_config, save_config, update_config};
pub use services::launcher::launch_minecraft;
use utils::logger::setup_logger;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            controllers::download_controller::get_versions,
            controllers::download_controller::get_version_size,
            controllers::download_controller::download_version,
            controllers::download_controller::cancel_download,
            controllers::config_controller::get_config,
            controllers::config_controller::get_game_dir,
            controllers::config_controller::get_game_dir_info,
            controllers::config_controller::set_game_dir,
            controllers::java_controller::find_java_installations_command,
            controllers::java_controller::refresh_java_installations,
            controllers::java_controller::set_java_path_command,
            controllers::config_controller::load_config_key,
            controllers::config_controller::save_config_key,
            controllers::java_controller::validate_java_path,
            controllers::java_controller::get_java_version,
            controllers::java_controller::get_custom_java_paths,
            controllers::java_controller::add_custom_java_path,
            controllers::java_controller::remove_custom_java_path,
            controllers::java_controller::check_java_compatibility,
            controllers::config_controller::get_download_threads,
            controllers::config_controller::set_download_threads,
            controllers::config_controller::get_proxy_settings,
            controllers::config_controller::set_proxy_settings,
            controllers::config_controller::test_proxy,
            controllers::config_controller::validate_version_files,
            controllers::auth_controller::get_saved_username,
            controllers::auth_controller::set_saved_username,
            controllers::auth_controller::get_saved_uuid,
            controllers::auth_controller::set_saved_uuid,
            controllers::auth_controller::get_auth_status,
            controllers::auth_controller::start_microsoft_login,
            controllers::auth_controller::complete_microsoft_login,
            controllers::auth_controller::start_microsoft_auth_code_login,
            controllers::auth_controller::complete_microsoft_auth_code_login,
            controllers::auth_controller::cancel_microsoft_login,
            controllers::auth_controller::refresh_microsoft_auth,
            controllers::auth_controller::refresh_skin_url,
            controllers::auth_controller::logout_microsoft,
            controllers::auth_controller::set_auth_type,
            controllers::config_controller::get_memory_stats,
            controllers::config_controller::check_memory_warning,
            controllers::config_controller::get_window_settings,
            controllers::config_controller::set_window_settings,
            controllers::config_controller::get_jvm_advanced_options,
            controllers::config_controller::set_jvm_advanced_options,
            controllers::config_controller::get_effective_jvm_args,
            controllers::config_controller::get_last_selected_version,
            controllers::config_controller::set_last_selected_version,
            controllers::instance_controller::validate_instance_name_cmd,
            controllers::instance_controller::check_instance_name_available,
            controllers::instance_controller::create_instance,
            controllers::instance_controller::get_instances,
            controllers::instance_controller::delete_instance,
            controllers::instance_controller::rename_instance,
            controllers::instance_controller::open_instance_folder,
            controllers::instance_controller::launch_instance,
            controllers::instance_controller::get_instance_mods,
            controllers::instance_controller::set_mod_enabled,
            controllers::instance_controller::delete_instance_mod,
            controllers::mod_controller::search_modrinth_mods,
            controllers::mod_controller::get_modrinth_mod_versions,
            controllers::mod_controller::install_modrinth_mod,
            controllers::mod_controller::check_modrinth_mod_updates,
            controllers::mod_controller::update_modrinth_mod,
            controllers::diagnostics_controller::export_diagnostic_bundle,
            controllers::frontend_log_controller::log_frontend_error,
            controllers::loader_controller::get_forge_versions,
            controllers::loader_controller::get_fabric_versions,
            controllers::loader_controller::get_quilt_versions,
            controllers::loader_controller::get_neoforge_versions,
            controllers::loader_controller::get_available_loaders,
            controllers::modpack_controller::search_modrinth_modpacks,
            controllers::modpack_controller::get_modrinth_modpack_versions,
            controllers::modpack_controller::install_modrinth_modpack,
            controllers::modpack_controller::cancel_modpack_install
        ])
        .setup(|_app| {
            let storage_dir = services::config::init_storage();
            let log_dir = storage_dir.join("logs");
            if let Err(e) = setup_logger(&log_dir) {
                eprintln!("Error setting up logger: {}", e);
            }

            log::info!("[DEBUG] 程序启动");
            log::info!(
                "[DEBUG] Tauri应用初始化完成（存储目录: {}，日志目录: {}）",
                storage_dir.display(),
                log_dir.display()
            );

            if let Err(e) = services::config::preload_config() {
                log::error!("配置预加载失败: {}", e);
            }

            // 预热 Java 检测缓存，放后台跑以免阻塞启动
            tauri::async_runtime::spawn(async {
                log::info!("后台预热 Java 检测缓存...");
                if let Err(e) = services::java::find_java_installations_command().await {
                    log::warn!("Java 缓存预热失败: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
