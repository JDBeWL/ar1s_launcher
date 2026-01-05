pub mod config;
pub mod download;
pub mod http_client;
pub mod java;
pub mod launcher;
pub mod instance;
pub mod loaders;  // 新的统一加载器模块
pub mod file_verification;
pub mod memory;
pub mod modrinth;
pub mod modpack_installer;

// 保留旧的 forge 模块以保持向后兼容（已弃用）
#[deprecated(note = "请使用 loaders::forge 代替")]
pub mod forge;
