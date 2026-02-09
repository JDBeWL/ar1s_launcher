//! 全局 HTTP 客户端管理
//!
//! 提供通用的全局 HTTP 客户端（带连接池复用、超时、UA），
//! 适用于 API 请求、加载器安装、整合包下载等场景。
//! 下载模块 (`download::http`) 有自己的专用客户端（禁用压缩、更长超时）。

use reqwest::Client;
use std::time::Duration;

/// 统一的 User-Agent 标识（供所有模块共享）
pub const USER_AGENT: &str = "Ar1s-Launcher/1.0";

/// 全局通用 HTTP 客户端（懒加载单例，连接池复用）
static HTTP_CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to create HTTP client")
});

/// 获取全局通用 HTTP 客户端引用
///
/// 适用于 API 请求、加载器版本查询、整合包搜索等场景。
/// 连接池自动复用，无需每次创建新客户端。
pub fn get_client() -> &'static Client {
    &HTTP_CLIENT
}
