//! 全局 HTTP 客户端模块
//!
//! 共享 reqwest 客户端（浏览器 UA、30 秒超时），代理设置变更后可整体重建；
//! 下载走 download::http 的专用客户端（禁用压缩、更长超时），不用这里的

use reqwest::Client;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

/// 用浏览器 UA，部分 API 会拒绝非浏览器标识
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Ar1sLauncher/1.0";

static HTTP_CLIENT: LazyLock<RwLock<Client>> = LazyLock::new(|| RwLock::new(build_client()));

/// 返回客户端副本：内部是 Arc，克隆很轻；正在进行的请求继续用旧客户端
pub fn get_client() -> Client {
    HTTP_CLIENT
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

pub fn reload() {
    let client = build_client();
    *HTTP_CLIENT
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = client;
}

fn build_client() -> Client {
    crate::services::proxy::apply(
        Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .user_agent(USER_AGENT),
    )
    .build()
    .expect("Failed to create HTTP client")
}
