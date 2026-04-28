use axum::{ routing::get, Router };

use crate::handlers::{ health, root, info, proxy_place_holder };
use crate::state::AppState;

pub fn create_routes() -> Router<AppState> {
  Router::new()
    .route("/", get(root))
    .route("/health", get(health))
    .route("/info", get(info))
    .route("/*path", get(proxy_place_holder))
}
