use crate::config::ProxyConfig;
use redis::Client as RedisClient;
use reqwest::Client;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub proxy_config: Arc<ProxyConfig>,
    pub http_client: Client,
    pub redis_client: RedisClient,
}
