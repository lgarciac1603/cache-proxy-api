use axum::{ routing::get, Router };

use crate::handlers::{ health, root };

pub fn create_routes() -> Router {
  Router::new()
    .route("/", get(root))
    .route("/health", get(health))
}