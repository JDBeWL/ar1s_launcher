mod errors;
mod models;
pub mod services;
pub mod controllers;

pub use errors::LauncherError;
pub use models::*;

// 直接导出launcher模块中的函数
pub use services::launcher::launch_minecraft;

// 导出config模块中的函数
pub use services::config::{load_config, save_config};


