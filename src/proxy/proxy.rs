use axum::{
    Json,
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::config::ProxyConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoute {
    pub backend: String,
    pub route_prefix: String,
    pub upstream_url: String,
    pub cache_ttl_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body_base64: String,
}

impl CachedResponse {
    pub fn from_parts(status: StatusCode, headers: &HeaderMap, body: &[u8]) -> Self {
        Self {
            status: status.as_u16(),
            headers: headers
                .iter()
                .filter(|(name, _)| should_forward_header(name))
                .filter_map(|(name, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value| (name.as_str().to_string(), value.to_string()))
                })
                .collect(),
            body_base64: STANDARD.encode(body),
        }
    }

    pub fn into_response(self, cache_status: &str) -> Result<Response, ProxyError> {
        let status = StatusCode::from_u16(self.status).map_err(|err| {
            ProxyError::internal(format!("Cached response had an invalid status code: {err}"))
        })?;
        let body = STANDARD.decode(self.body_base64).map_err(|err| {
            ProxyError::internal(format!("Cached response body was invalid: {err}"))
        })?;

        let mut headers = HeaderMap::new();
        for (name, value) in self.headers {
            let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|err| {
                ProxyError::internal(format!(
                    "Cached response had an invalid header name '{name}': {err}"
                ))
            })?;
            let header_value = HeaderValue::from_str(&value).map_err(|err| {
                ProxyError::internal(format!(
                    "Cached response had an invalid header value for '{name}': {err}"
                ))
            })?;

            headers.append(header_name, header_value);
        }

        Ok(build_response(status, &headers, body, cache_status))
    }
}

#[derive(Debug)]
pub struct ProxyError {
    status: StatusCode,
    error: &'static str,
    message: String,
}

#[derive(Serialize)]
struct ProxyErrorBody<'a> {
    error: &'a str,
    message: &'a str,
}

impl ProxyError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: "route_not_found",
            message: message.into(),
        }
    }

    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            error: "bad_gateway",
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: "internal_error",
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

impl std::error::Error for ProxyError {}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ProxyErrorBody {
                error: self.error,
                message: &self.message,
            }),
        )
            .into_response()
    }
}

pub fn resolve_route(
    config: &ProxyConfig,
    request_path: &str,
    query: Option<&str>,
) -> Result<ResolvedRoute, ProxyError> {
    let route = config
        .routes
        .iter()
        .filter(|route| route_matches_path(&route.prefix, request_path))
        .max_by_key(|route| route.prefix.len())
        .ok_or_else(|| ProxyError::not_found(format!("No route matches path '{request_path}'")))?;

    let backend = config.backends.get(&route.backend).ok_or_else(|| {
        ProxyError::internal(format!(
            "Route '{}' references missing backend '{}'",
            route.prefix, route.backend
        ))
    })?;

    let suffix = request_path
        .strip_prefix(&route.prefix)
        .unwrap_or_default()
        .trim_start_matches('/');
    let upstream_url = build_upstream_url(backend, suffix, query);

    Ok(ResolvedRoute {
        backend: route.backend.clone(),
        route_prefix: route.prefix.clone(),
        upstream_url,
        cache_ttl_seconds: route.cache_ttl_seconds,
    })
}

pub fn build_cache_key(method: &Method, upstream_url: &str) -> String {
    format!("cache-proxy:{}:{}", method.as_str(), upstream_url)
}

pub fn should_cache_request(
    method: &Method,
    cache_ttl_seconds: Option<u64>,
    cache_bypassed: bool,
) -> Option<u64> {
    if cache_bypassed || *method != Method::GET {
        return None;
    }

    cache_ttl_seconds
}

pub fn should_bypass_cache(headers: &HeaderMap, query: Option<&str>) -> bool {
    let header_bypass = headers
        .get("x-cache-bypass")
        .and_then(|value| value.to_str().ok())
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let query_bypass = query
        .map(|query| {
            query.contains("cache_bypass=1")
                || query.contains("cache_bypass=true")
                || query.contains("cache_bypass=True")
        })
        .unwrap_or(false);

    header_bypass || query_bypass
}

pub fn should_forward_header(name: &HeaderName) -> bool {
    let name = name.as_str();

    !name.eq_ignore_ascii_case("host")
        && !name.eq_ignore_ascii_case("connection")
        && !name.eq_ignore_ascii_case("keep-alive")
        && !name.eq_ignore_ascii_case("proxy-authenticate")
        && !name.eq_ignore_ascii_case("proxy-authorization")
        && !name.eq_ignore_ascii_case("te")
        && !name.eq_ignore_ascii_case("trailers")
        && !name.eq_ignore_ascii_case("transfer-encoding")
        && !name.eq_ignore_ascii_case("upgrade")
        && !name.eq_ignore_ascii_case("content-length")
        && !name.eq_ignore_ascii_case("x-cache")
}

pub fn build_response(
    status: StatusCode,
    headers: &HeaderMap,
    body: Vec<u8>,
    cache_status: &str,
) -> Response {
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;

    for (name, value) in headers {
        if should_forward_header(name) {
            response.headers_mut().append(name, value.clone());
        }
    }

    response.headers_mut().insert(
        HeaderName::from_static("x-cache"),
        HeaderValue::from_str(cache_status).unwrap_or_else(|_| HeaderValue::from_static("INVALID")),
    );

    response
}

fn route_matches_path(prefix: &str, request_path: &str) -> bool {
    request_path == prefix
        || request_path
            .strip_prefix(prefix)
            .map(|suffix| suffix.starts_with('/'))
            .unwrap_or(false)
}

fn build_upstream_url(backend: &str, suffix: &str, query: Option<&str>) -> String {
    let mut upstream_url = backend.trim_end_matches('/').to_string();

    if !suffix.is_empty() {
        upstream_url.push('/');
        upstream_url.push_str(suffix);
    }

    if let Some(query) = query {
        upstream_url.push('?');
        upstream_url.push_str(query);
    }

    upstream_url
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{
        Json, Router,
        body::{Body, to_bytes},
        http::{HeaderMap, HeaderValue, Method, Request, StatusCode},
        routing::get,
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use crate::{
        config::{ProxyConfig, RouteRule},
        handlers::proxy_request,
        state::AppState,
    };

    use super::{CachedResponse, build_cache_key, resolve_route, should_bypass_cache};

    fn sample_proxy_config() -> ProxyConfig {
        let mut backends = HashMap::new();
        backends.insert(
            "coingecko-api".to_string(),
            "https://api.coingecko.com/api/v3".to_string(),
        );
        backends.insert(
            "favorites-api".to_string(),
            "http://favorites-api:8090".to_string(),
        );

        ProxyConfig {
            backends,
            routes: vec![
                RouteRule {
                    prefix: "/providers".to_string(),
                    backend: "favorites-api".to_string(),
                    cache_ttl_seconds: None,
                },
                RouteRule {
                    prefix: "/providers/coingecko".to_string(),
                    backend: "coingecko-api".to_string(),
                    cache_ttl_seconds: Some(60),
                },
            ],
        }
    }

    #[test]
    fn resolves_longest_matching_route_and_preserves_query_string() {
        let config = sample_proxy_config();
        let resolved = resolve_route(
            &config,
            "/providers/coingecko/coin/bitcoin",
            Some("vs_currency=usd&days=7"),
        )
        .unwrap();

        assert_eq!(resolved.backend, "coingecko-api");
        assert_eq!(
            resolved.upstream_url,
            "https://api.coingecko.com/api/v3/coin/bitcoin?vs_currency=usd&days=7"
        );
        assert_eq!(resolved.cache_ttl_seconds, Some(60));
    }

    #[test]
    fn builds_stable_cache_keys() {
        assert_eq!(
            build_cache_key(
                &Method::GET,
                "https://api.coingecko.com/api/v3/markets?vs_currency=usd"
            ),
            "cache-proxy:GET:https://api.coingecko.com/api/v3/markets?vs_currency=usd"
        );
    }

    #[test]
    fn detects_cache_bypass_controls() {
        let mut headers = HeaderMap::new();
        headers.insert("x-cache-bypass", HeaderValue::from_static("true"));

        assert!(should_bypass_cache(&headers, None));
        assert!(should_bypass_cache(
            &HeaderMap::new(),
            Some("cache_bypass=1")
        ));
        assert!(!should_bypass_cache(
            &HeaderMap::new(),
            Some("vs_currency=usd")
        ));
    }

    #[test]
    fn rebuilds_cached_response_with_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let cached = CachedResponse::from_parts(StatusCode::OK, &headers, br#"{"ok":true}"#);
        let response = cached.into_response("HIT").unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["x-cache"], "HIT");
        assert_eq!(response.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn forwards_requests_to_the_resolved_backend() {
        async fn upstream_handler() -> Json<Value> {
            Json(json!({
                "source": "upstream",
                "ok": true
            }))
        }

        let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = upstream_listener.local_addr().unwrap();
        tokio::spawn(async move {
            let upstream_app = Router::new().route("/coin/bitcoin", get(upstream_handler));
            axum::serve(upstream_listener, upstream_app).await.unwrap();
        });

        let mut backends = HashMap::new();
        backends.insert(
            "coingecko-api".to_string(),
            format!("http://{}", upstream_addr),
        );

        let proxy_state = AppState {
            proxy_config: Arc::new(ProxyConfig {
                backends,
                routes: vec![RouteRule {
                    prefix: "/providers/coingecko".to_string(),
                    backend: "coingecko-api".to_string(),
                    cache_ttl_seconds: Some(60),
                }],
            }),
            http_client: reqwest::Client::new(),
            redis_client: redis::Client::open("redis://127.0.0.1:6379/").unwrap(),
        };

        let proxy_app = Router::new()
            .route("/{*path}", axum::routing::any(proxy_request))
            .with_state(proxy_state);

        let response = proxy_app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/providers/coingecko/coin/bitcoin")
                    .header("x-cache-bypass", "true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["x-cache"], "BYPASS");

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["source"], "upstream");
        assert_eq!(payload["ok"], true);
    }
}
