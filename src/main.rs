mod config;
mod handlers;
mod routes;
mod state;

use dotenvy::dotenv;
use reqwest::Client;
use std::sync::Arc;

use state::AppState;

#[tokio::main]
async fn main() {
	dotenv().ok();

	let app_config = config::load_app_config()
		.expect("Failed to load app config");
	let proxy_config = config::load_proxy_config(&app_config.proxy_config_path)
		.expect("Failed to load proxy config");

	print!("App config: {:?}", app_config);
	print!("Proxy config: {:?}", proxy_config);

	let state = AppState {
		proxy_config: Arc::new(proxy_config),
		http_client: Client::new(),
	};
	
	let app = routes::create_routes().with_state(state);
	let app_url = format!("0.0.0.0:{}", app_config.port);
	let listener = tokio::net::TcpListener::bind(&app_url).await.unwrap();

	print!("Server running on: {}", app_url);

	axum::serve(listener, app).await.unwrap();
}
