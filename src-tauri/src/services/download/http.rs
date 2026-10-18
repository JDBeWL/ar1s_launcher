//! 下载 HTTP 客户端
//!
//! 独立于全局客户端的大文件专用 reqwest：禁用压缩、总超时 300 秒；
//! 代理设置变更后可整体重建

use std::sync::{LazyLock, RwLock};
use std::time::Duration;

static HTTP_CLIENT: LazyLock<RwLock<reqwest::Client>> =
    LazyLock::new(|| RwLock::new(create_client(16)));

/// 返回客户端副本：内部是 Arc，克隆很轻；正在进行的下载继续用旧客户端
pub fn get_http_client() -> reqwest::Client {
    HTTP_CLIENT
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

pub(crate) fn reload() {
    let client = create_client(16);
    *HTTP_CLIENT
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = client;
}

fn create_client(max_connections_per_host: usize) -> reqwest::Client {
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Ar1sLauncher/1.0",
        ),
    );
    default_headers.insert(
        reqwest::header::ACCEPT_ENCODING,
        reqwest::header::HeaderValue::from_static("identity"),
    );

    crate::services::proxy::apply(
        reqwest::Client::builder()
            .default_headers(default_headers)
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .pool_max_idle_per_host(max_connections_per_host * 4)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300)),
    )
    .build()
    .expect("Failed to create HTTP client")
}
