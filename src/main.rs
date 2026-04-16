mod handlers;
mod routes;

use dotenvy::dotenv;
use std::env;

#[tokio::main]

async fn main() {
	dotenv().ok();
	let app = routes::create_routes();
	
	let port = env::var("PORT").expect("PORT variable not found");
	let app_url = format!("0.0.0.0:{port}");

	let listener = tokio::net::TcpListener::bind(&app_url).await.unwrap();
	print!("Server running on: {}", app_url);
	
	axum::serve(listener, app).await.unwrap();
}
