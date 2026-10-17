//! 全局 HTTP 客户端模块
//!
//! 惰性初始化的共享 reqwest 客户端（浏览器 UA、30 秒超时），供 API 与元数据请求
//! 下载走 download::http 的专用客户端（禁用压缩、更长超时），不用这里的

use reqwest::Client;
use std::time::Duration;

/// 用浏览器 UA，部分 API 会拒绝非浏览器标识
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Ar1sLauncher/1.0";

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

pub fn get_client() -> &'static Client {
    &HTTP_CLIENT
}
