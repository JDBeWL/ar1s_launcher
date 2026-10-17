//! 下载模块
//!
//! 组装版本清单里的 jar、资源与库文件任务，并发下载、续传、校验并上报进度

pub mod batch;
mod file;
mod http;
mod manifest;
mod state;
mod version;

pub use batch::download_all_files;
pub use file::download_file;
pub use http::get_http_client;
pub use manifest::VersionSizeInfo;
pub use manifest::get_version_size;
pub use manifest::get_versions;
pub use version::process_and_download_version;
