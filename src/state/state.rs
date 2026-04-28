use std::sync::Arc;
use reqwest::Client;
use crate::config::ProxyConfig;

#[derive(Clone)]
pub struct AppState {
  pub proxy_config: Arc<ProxyConfig>,
  pub http_client: Client,
}
