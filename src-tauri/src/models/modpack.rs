use serde::{Deserialize, Serialize};

// Modrinth整合包信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthModpack {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub author: String,
    pub downloads: u64,
    pub date_created: String,
    pub date_modified: String,
    pub latest_version: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub categories: Vec<String>,
}

// Modrinth整合包版本信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthModpackVersion {
    pub id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub featured: bool,
    pub date_published: String,
    pub downloads: u64,
    pub files: Vec<ModrinthFile>,
    pub dependencies: Vec<ModrinthDependency>,
}

// Modrinth文件信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: ModrinthHashes,
}

// Modrinth文件哈希值
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthHashes {
    pub sha1: String,
    pub sha512: String,
}

// Modrinth依赖关系
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: String,
}

// Modrinth搜索参数
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthSearchParams {
    pub query: Option<String>,
    pub game_versions: Option<Vec<String>>,
    pub loaders: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// Modrinth搜索响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthSearchResponse {
    pub hits: Vec<ModrinthModpack>,
    pub total_hits: u32,
}

// 整合包安装选项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackInstallOptions {
    pub modpack_id: String,
    pub version_id: String,
    pub instance_name: String,
    pub install_path: String,
}