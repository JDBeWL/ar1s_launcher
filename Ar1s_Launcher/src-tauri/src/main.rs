// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    println!("[DEBUG] 程序启动");

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
            ar1s_launcher_lib::controllers::config_controller::get_total_memory
        ])
        .setup(|_| {
            println!("[DEBUG] Tauri应用初始化完成");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}