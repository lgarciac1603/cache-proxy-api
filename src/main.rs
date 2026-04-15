use dotenvy::dotenv;
use std::{ env };
use axum::{ routing::get, Router };

#[tokio::main]

async fn main() {
	dotenv().ok();

	let app = Router::new().route("/", get(|| async { "Hello world!" }));
	let port = env::var("PORT").expect("PORT variable not found");

	let app_url = format!("0.0.0.0:{port}");

	let listener = tokio::net::TcpListener::bind(&app_url).await.unwrap();
	axum::serve(listener, app).await.unwrap();

	print!("Server running on: {}", app_url);
}
