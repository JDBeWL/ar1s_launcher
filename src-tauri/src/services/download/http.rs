//! 全局 HTTP 客户端管理（下载专用 - 禁用压缩、更长超时）

use std::time::Duration;

static HTTP_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    create_client(16)
});

pub fn get_http_client() -> &'static reqwest::Client {
    &HTTP_CLIENT
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

    reqwest::Client::builder()
        .default_headers(default_headers)
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .pool_max_idle_per_host(max_connections_per_host * 4)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .build()
        .expect("Failed to create HTTP client")
}
