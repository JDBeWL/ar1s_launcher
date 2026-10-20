//! 整合包数据模型
//!
//! Modrinth 接口的项目、版本、文件与搜索响应，加前端提交的本机安装参数

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthModpackVersion {
    pub id: String,
    /// 所属项目，模组更新与依赖去重时用
    #[serde(default)]
    pub project_id: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: ModrinthHashes,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthSearchResponse {
    pub hits: Vec<ModrinthModpack>,
    pub total_hits: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackInstallOptions {
    pub modpack_id: String,
    pub version_id: String,
    pub instance_name: String,
    pub install_path: String,
}

/// 从本地 .mrpack 安装的参数；file_path 是用户选中的压缩包
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalModpackInstallOptions {
    pub file_path: String,
    pub instance_name: String,
}

/// 导出整合包的参数
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModpackExportOptions {
    /// 整合包名称，同时也当 index 里的 name
    pub name: String,
    /// 版本号，写进 index 的 versionId
    pub version: String,
    /// 简介可省略，前端不填时不下发这个字段
    #[serde(default)]
    pub summary: Option<String>,
    /// 保存位置，必须以 .mrpack 结尾
    pub target_path: String,
    /// 是否把存档一起打包（体积大、常常是私人的）
    pub include_saves: bool,
}

/// 导出结果，供前端提示
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModpackExportResult {
    pub path: String,
    pub name: String,
    pub version_id: String,
    /// 写进 index.files 的文件数（安装时按链接下载）
    pub linked_files: usize,
    /// 打进 overrides 的文件数
    pub override_files: usize,
    pub total_bytes: u64,
}
