use serde::Deserialize;
use std::collections::HashMap;
use std:: { env, fs } ;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AppConfig {
  pub port: u16,
  pub redis_url: String,
  pub proxy_config_path: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ProxyConfig {
  pub backends: HashMap<String, String>,
  pub routes: Vec<RouteRule>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RouteRule {
  pub prefix: String,
  pub backend: String,
}

pub fn load_app_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
  let port = env::var("PORT")?.parse::<u16>()?;
  let redis_url = env::var("REDIS_URL")?;
  let proxy_config_path = env::var("PROXY_CONFIG_PATH")?;

  Ok(AppConfig {
    port,
    redis_url,
    proxy_config_path,
  })
}

pub fn load_proxy_config(path: &str) -> Result<ProxyConfig, Box<dyn std::error::Error>> {
  let contents = fs::read_to_string(path)?;
  let config: ProxyConfig = serde_json::from_str(&contents)?;

  Ok(config)
}
