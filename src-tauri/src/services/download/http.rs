//! 全局 HTTP 客户端管理

use crate::errors::LauncherError;
use std::sync::Arc;
use std::time::Duration;

/// 全局 HTTP 客户端（懒加载单例）
static HTTP_CLIENT: std::sync::OnceLock<Arc<reqwest::Client>> = std::sync::OnceLock::new();

/// 获取全局 HTTP 客户端
pub fn get_http_client() -> Result<Arc<reqwest::Client>, LauncherError> {
    let client = HTTP_CLIENT.get_or_init(|| {
        Arc::new(create_client(16)) // 默认支持 16 线程
    });
    Ok(client.clone())
}

/// 创建 HTTP 客户端
fn create_client(max_connections_per_host: usize) -> reqwest::Client {
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Ar1s-Launcher/1.0",
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
        .timeout(Duration::from_secs(300)) // 5 分钟总超时
        .build()
        .expect("Failed to create HTTP client")
}

/// 创建用于版本清单获取的客户端（较短超时）
pub fn get_manifest_client() -> Result<reqwest::Client, LauncherError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| LauncherError::Custom(format!("创建HTTP客户端失败: {}", e)))
}
