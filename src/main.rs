mod config;
mod handlers;
mod proxy;
mod routes;
mod state;

use dotenvy::dotenv;
use redis::Client as RedisClient;
use reqwest::Client;
use std::sync::Arc;

use state::AppState;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app_config = config::load_app_config().expect("Failed to load app config");
    let proxy_config = config::load_proxy_config(&app_config.proxy_config_path)
        .expect("Failed to load proxy config");
    let redis_client =
        RedisClient::open(app_config.redis_url.clone()).expect("Failed to create Redis client");

    println!(
        "Loaded cache-proxy-api config: port={}, redis_url={}, routes={}",
        app_config.port,
        app_config.redis_url,
        proxy_config.routes.len()
    );

    let state = AppState {
        proxy_config: Arc::new(proxy_config),
        http_client: Client::new(),
        redis_client,
    };

    let app = routes::create_routes().with_state(state);
    let app_url = format!("0.0.0.0:{}", app_config.port);
    let listener = tokio::net::TcpListener::bind(&app_url).await.unwrap();

    println!("cache-proxy-api listening on {}", app_url);

    axum::serve(listener, app).await.unwrap();
}
