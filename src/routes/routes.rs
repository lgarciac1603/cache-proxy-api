use axum::{
	Router,
	routing::{any, get},
};

use crate::handlers::{health, info, proxy_request, root};
use crate::state::AppState;

pub fn create_routes() -> Router<AppState> {
	Router::new()
		.route("/", get(root))
		.route("/health", get(health))
		.route("/info", get(info))
		.route("/{*path}", any(proxy_request))
}
