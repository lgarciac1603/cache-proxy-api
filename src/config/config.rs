use serde::Deserialize;
use std::collections::HashMap;
use std::{
    env, fs,
    io::{Error, ErrorKind},
};

use reqwest::Url;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub redis_url: String,
    pub proxy_config_path: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProxyConfig {
    pub backends: HashMap<String, String>,
    pub routes: Vec<RouteRule>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RouteRule {
    pub prefix: String,
    pub backend: String,
    #[serde(default)]
    pub cache_ttl_seconds: Option<u64>,
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
    validate_proxy_config(&config)?;

    Ok(config)
}

pub(crate) fn validate_proxy_config(
    config: &ProxyConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if config.backends.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "proxy config must define at least one backend",
        )
        .into());
    }

    if config.routes.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "proxy config must define at least one route",
        )
        .into());
    }

    for (backend_name, backend_url) in &config.backends {
        Url::parse(backend_url).map_err(|err| {
            Error::new(
                ErrorKind::InvalidData,
                format!("backend '{backend_name}' has an invalid URL '{backend_url}': {err}"),
            )
        })?;
    }

    for route in &config.routes {
        if !route.prefix.starts_with('/') {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("route prefix '{}' must start with '/'", route.prefix),
            )
            .into());
        }

        if !config.backends.contains_key(&route.backend) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "route prefix '{}' references unknown backend '{}'",
                    route.prefix, route.backend
                ),
            )
            .into());
        }

        if matches!(route.cache_ttl_seconds, Some(0)) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "route prefix '{}' must use a cache TTL greater than 0 seconds",
                    route.prefix
                ),
            )
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{ProxyConfig, RouteRule, validate_proxy_config};

    fn valid_proxy_config() -> ProxyConfig {
        let mut backends = HashMap::new();
        backends.insert(
            "coingecko-api".to_string(),
            "https://api.coingecko.com/api/v3".to_string(),
        );

        ProxyConfig {
            backends,
            routes: vec![RouteRule {
                prefix: "/providers/coingecko".to_string(),
                backend: "coingecko-api".to_string(),
                cache_ttl_seconds: Some(60),
            }],
        }
    }

    #[test]
    fn rejects_route_with_unknown_backend() {
        let mut config = valid_proxy_config();
        config.routes[0].backend = "missing-backend".to_string();

        assert!(validate_proxy_config(&config).is_err());
    }

    #[test]
    fn rejects_invalid_backend_url() {
        let mut config = valid_proxy_config();
        config
            .backends
            .insert("coingecko-api".to_string(), "not a url".to_string());

        assert!(validate_proxy_config(&config).is_err());
    }
}
