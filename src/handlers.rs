use std::env;
use axum::Json;
use serde::Serialize;

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
		port: env::var("PORT").expect("PORT variable not found in env vars").to_string(),
	})
}
