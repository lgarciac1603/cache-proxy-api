use dotenvy::dotenv;
use std::env;
use axum::{ routing::get, Router };

#[tokio::main]

async fn main() {
	dotenv().ok();

	let app = Router::new().route("/", get(|| async { "Hello world!" }));
	let port = env::var("PORT").expect("PORT variable not found");

	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
	axum::serve(listener, app).await.unwrap();
}
