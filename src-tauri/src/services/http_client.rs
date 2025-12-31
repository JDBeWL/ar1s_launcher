use reqwest::Client;
use std::time::Duration;

/// 全局 HTTP 客户端（连接池复用）
static HTTP_CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .user_agent("Ar1s-Launcher/1.0")
        .build()
        .expect("Failed to create HTTP client")
});

/// 获取全局 HTTP 客户端
pub fn get_client() -> &'static Client {
    &HTTP_CLIENT
}

/// 创建带自定义超时的客户端（用于特殊场景）
pub fn create_client_with_timeout(timeout_secs: u64) -> Client {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(5)
        .user_agent("Ar1s-Launcher/1.0")
        .build()
        .expect("Failed to create HTTP client")
}
