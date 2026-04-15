use dotenvy::dotenv;
use std::{ env };
use axum::{ routing::get, Router };

#[tokio::main]

async fn main() {
	dotenv().ok();

	let app = Router::new()
		.route("/", get(root))
		.route("/health", get(health));
	
	let port = env::var("PORT").expect("PORT variable not found");
	let app_url = format!("0.0.0.0:{port}");

	let listener = tokio::net::TcpListener::bind(&app_url).await.unwrap();
	print!("Server running on: {}", app_url);
	
	axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
	"Hello world!"
}

async fn health() -> &'static str {
	"ok"
}
