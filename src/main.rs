mod config;
mod handlers;
mod routes;

use dotenvy::dotenv;

#[tokio::main]
async fn main() {
	dotenv().ok();

	let app_config = config::load_app_config()
		.expect("Failed to load app config");
	let proxy_config = config::load_proxy_config(&app_config.proxy_config_path)
		.expect("Failed to load proxy config");

	print!("App config: {:?}", app_config);
	print!("Proxy config: {:?}", proxy_config);
	
	let app = routes::create_routes();
	let app_url = format!("0.0.0.0:{}", app_config.port);
	let listener = tokio::net::TcpListener::bind(&app_url).await.unwrap();

	print!("Server running on: {}", app_url);

	axum::serve(listener, app).await.unwrap();
}
