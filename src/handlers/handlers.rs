use std::env;

use axum::{
    Json,
    body::Bytes,
    extract::{OriginalUri, State},
    http::{HeaderMap, Method},
    response::Response,
};
use redis::AsyncCommands;
use serde::Serialize;

use crate::{
    proxy::{
        CachedResponse, ProxyError, build_cache_key, build_response, resolve_route,
        should_bypass_cache, should_cache_request, should_forward_header,
    },
    state::AppState,
};

pub async fn root() -> &'static str {
    "Hello world!"
}

pub async fn health() -> &'static str {
    "ok"
}

#[derive(Serialize)]
pub struct InfoResponse {
    service: String,
    version: String,
    port: String,
}

pub async fn info() -> Json<InfoResponse> {
    Json(InfoResponse {
        service: "cache-proxy-api".to_string(),
        version: "0.1".to_string(),
        port: env::var("PORT")
            .expect("PORT variable not found in env vars")
            .to_string(),
    })
}

pub async fn proxy_request(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    OriginalUri(original_uri): OriginalUri,
    body: Bytes,
) -> Result<Response, ProxyError> {
    let request_path = original_uri.path().to_string();
    let resolved_route = resolve_route(
        state.proxy_config.as_ref(),
        &request_path,
        original_uri.query(),
    )?;
    let cache_bypassed = should_bypass_cache(&headers, original_uri.query());
    let cache_ttl = should_cache_request(&method, resolved_route.cache_ttl_seconds, cache_bypassed);
    let cache_key = cache_ttl.map(|_| build_cache_key(&method, &resolved_route.upstream_url));

    if let (Some(ttl), Some(cache_key)) = (cache_ttl, cache_key.as_ref()) {
        let mut redis_connection = state
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|err| ProxyError::bad_gateway(format!("Failed to connect to Redis: {err}")))?;

        let cached_payload: Option<String> = redis_connection
            .get(cache_key)
            .await
            .map_err(|err| ProxyError::bad_gateway(format!("Failed to read from Redis: {err}")))?;

        if let Some(cached_payload) = cached_payload {
            println!("Cache hit for {} {}", method, resolved_route.upstream_url);
            let cached_response: CachedResponse =
                serde_json::from_str(&cached_payload).map_err(|err| {
                    ProxyError::internal(format!("Failed to deserialize cached response: {err}"))
                })?;

            return cached_response.into_response("HIT");
        }

        println!(
            "Cache miss for {} {} (ttl={}s)",
            method, resolved_route.upstream_url, ttl
        );
    }

    println!("Forwarding {} {}", method, resolved_route.upstream_url);

    let mut upstream_request = state
        .http_client
        .request(method.clone(), &resolved_route.upstream_url);

    for (name, value) in &headers {
        if should_forward_header(name) {
            upstream_request = upstream_request.header(name, value);
        }
    }

    if !body.is_empty() {
        upstream_request = upstream_request.body(body.clone());
    }

    let upstream_response = upstream_request
        .send()
        .await
        .map_err(|err| ProxyError::bad_gateway(format!("Upstream request failed: {err}")))?;

    let status = upstream_response.status();
    let upstream_headers = upstream_response.headers().clone();
    let upstream_body = upstream_response.bytes().await.map_err(|err| {
        ProxyError::bad_gateway(format!("Failed to read upstream response body: {err}"))
    })?;

    if let (Some(ttl), Some(cache_key)) = (cache_ttl, cache_key.as_ref()) {
        if status.is_success() {
            let mut redis_connection = state
                .redis_client
                .get_multiplexed_async_connection()
                .await
                .map_err(|err| {
                    ProxyError::bad_gateway(format!("Failed to connect to Redis: {err}"))
                })?;

            let cached_response =
                CachedResponse::from_parts(status, &upstream_headers, upstream_body.as_ref());
            let serialized = serde_json::to_string(&cached_response).map_err(|err| {
                ProxyError::internal(format!("Failed to serialize cached response: {err}"))
            })?;

            let _: () = redis_connection
                .set_ex(cache_key, serialized, ttl)
                .await
                .map_err(|err| {
                    ProxyError::bad_gateway(format!("Failed to write to Redis: {err}"))
                })?;
        } else {
            println!(
                "Skipping cache write for {} because upstream returned status {}",
                resolved_route.upstream_url, status
            );
        }
    }

    let cache_status = match (cache_ttl, cache_bypassed) {
        (Some(_), false) if status.is_success() => "MISS",
        (Some(_), false) => "SKIP",
        (_, true) => "BYPASS",
        _ => "DISABLED",
    };

    Ok(build_response(
        status,
        &upstream_headers,
        upstream_body.to_vec(),
        cache_status,
    ))
}
