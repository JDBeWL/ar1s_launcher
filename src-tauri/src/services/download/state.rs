//! 下载状态管理（支持断点续传）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 下载状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadState {
    /// 已完成的文件 URL 列表
    pub completed_files: Vec<String>,
    /// 下载失败的文件 URL 列表
    pub failed_files: Vec<String>,
    /// 部分下载的文件信息（URL -> 已下载字节数）
    #[serde(default)]
    pub partial_downloads: HashMap<String, u64>,
    /// 当前活跃的下载（仅内存中）
    #[serde(skip)]
    pub active_downloads: HashMap<String, PathBuf>,
    /// 是否有未保存的更改
    #[serde(skip)]
    pub dirty: bool,
}

impl DownloadState {
    pub fn new() -> Self {
        Self {
            completed_files: Vec::new(),
            failed_files: Vec::new(),
            partial_downloads: HashMap::new(),
            active_downloads: HashMap::new(),
            dirty: false,
        }
    }

    /// 从文件加载状态
    pub fn load_from_file(path: &std::path::Path) -> Option<Self> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
    }

    /// 保存状态到文件
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, content)
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_completed(&mut self, url: String) {
        // 从部分下载中移除
        self.partial_downloads.remove(&url);
        if !self.completed_files.contains(&url) {
            self.completed_files.push(url);
        }
        self.mark_dirty();
    }

    pub fn mark_failed(&mut self, url: String) {
        if !self.failed_files.contains(&url) {
            self.failed_files.push(url);
        }
        self.mark_dirty();
    }

    /// 更新部分下载进度
    #[allow(dead_code)]
    pub fn update_partial(&mut self, url: String, bytes: u64) {
        self.partial_downloads.insert(url, bytes);
        self.mark_dirty();
    }

    /// 获取部分下载的字节数
    #[allow(dead_code)]
    pub fn get_partial_bytes(&self, url: &str) -> u64 {
        self.partial_downloads.get(url).copied().unwrap_or(0)
    }

    pub fn start_download(&mut self, url: String, path: PathBuf) {
        self.active_downloads.insert(url, path);
    }

    pub fn finish_download(&mut self, url: &str) {
        self.active_downloads.remove(url);
    }

    /// 检查文件是否已完成
    pub fn is_completed(&self, url: &str) -> bool {
        self.completed_files.contains(&url.to_string())
    }

    /// 清除失败状态（用于重试）
    #[allow(dead_code)]
    pub fn clear_failed(&mut self, url: &str) {
        self.failed_files.retain(|u| u != url);
        self.mark_dirty();
    }

    /// 重置状态（用于重新下载）
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.completed_files.clear();
        self.failed_files.clear();
        self.partial_downloads.clear();
        self.active_downloads.clear();
        self.dirty = true;
    }
}

impl Default for DownloadState {
    fn default() -> Self {
        Self::new()
    }
}
