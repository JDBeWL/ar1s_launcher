//! 下载模块
//!
//! 该模块负责 Minecraft 游戏文件的下载，包括：
//! - 版本文件下载
//! - 批量文件下载
//! - 单文件下载
//! - 版本清单获取

mod batch;
mod file;
mod http;
mod manifest;
mod state;
mod version;

pub use batch::download_all_files;
pub use http::get_http_client;
pub use manifest::get_versions;
pub use version::process_and_download_version;
