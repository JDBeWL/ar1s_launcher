//! 前端错误落盘 command
//!
//! logger 在生产环境静默，error 级日志经此写入启动器日志，供诊断包收集

use crate::utils::logger;

#[tauri::command]
pub fn log_frontend_error(context: String, message: String) {
    logger::log_frontend_message(&context, &message);
}
