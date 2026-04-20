# Copilot Instructions

## Build and test commands

- `cargo build`
- `cargo test`
- `cargo test <filter>` to run a single test or subset once tests exist, for example `cargo test health_handler`
- `docker compose up --build` to run the API with Redis in the local compose setup

## High-level architecture

- `src/main.rs` is the full startup path: it loads `.env`, reads `PORT`, `REDIS_URL`, and `PROXY_CONFIG_PATH` into `AppConfig`, loads the JSON proxy config from disk, builds the Axum router, and then binds `0.0.0.0:$PORT`.
- Routing is centralized in `src/routes/routes.rs`. The current HTTP surface is only `/`, `/health`, and `/info`.
- Runtime configuration is split into two layers:
  - environment variables parsed by `src/config/config.rs`
  - proxy rules loaded from JSON, with a `backends` map and `routes` array of `{ prefix, backend }`
- The project already loads Redis and proxy-routing configuration at startup, but request handling does not use either yet. If you implement proxy behavior, extend the existing config types and startup flow rather than introducing a second config path.

## Key conventions

- `config`, `handlers`, and `routes` each use a thin `mod.rs` re-export pattern. Follow that structure when adding modules or public entrypoints.
- Handlers return Axum response types directly (`&'static str` or `Json<T>`) instead of custom response wrappers.
- Configuration is fail-fast: `load_app_config()` and `load_proxy_config()` return errors upward and `main` aborts on invalid or missing config.
- `/info` reads `PORT` from the environment at request time instead of reusing `AppConfig`. Keep that behavior in mind if you refactor config access.
- `src/backends/` exists but is currently empty; there is no implemented backend/proxy execution layer yet.
