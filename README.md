# cache-proxy-api

A **Rust** microservice intended to act as an **HTTP proxy + Redis-backed cache** in front of one or more backends.

In its current state, the project already:

- starts as an Axum-based API
- loads global configuration from environment variables
- loads a `proxy-config.json` file
- exposes basic technical endpoints

It does **not** yet implement the real proxy flow or caching inside the HTTP request path, but it already has the foundation required to resolve which backend matches a given route.

---

## Current status

### Available endpoints

| Endpoint | Purpose |
| --- | --- |
| `/` | minimal test response |
| `/health` | service healthcheck |
| `/info` | basic microservice information |

### Configuration currently loaded

The service expects these variables:

| Variable | Description |
| --- | --- |
| `PORT` | HTTP port used by the microservice |
| `REDIS_URL` | Redis connection string |
| `PROXY_CONFIG_PATH` | path to the `proxy-config.json` file |

Example:

```env
PORT=8070
REDIS_URL=redis://redis:6379
PROXY_CONFIG_PATH=/app/src/config/proxy-config.json
```

---

## How it fits into the larger stack

Inside `crypto-dashboard`, this microservice runs as `cache-proxy-api` and receives traffic through:

- `nginx.conf` in Docker: `/api-cache/*` -> `http://cache-proxy-api:8070/`
- `proxy.conf.json` in development: `/api-cache/*` -> `http://localhost:8070/`

In other words, the frontend does not talk directly to CoinGecko or to the proxy container itself; it goes through `/api-cache`.

---

## Project structure

```text
src/
  config/
    config.rs
    proxy-config.json
  handlers/
    handlers.rs
    mod.rs
  routes/
    routes.rs
    mod.rs
  main.rs
```

### Responsibility of each part

| File / module | Responsibility |
| --- | --- |
| `main.rs` | service startup |
| `src/routes` | HTTP route definitions |
| `src/handlers` | endpoint handlers |
| `src/config/config.rs` | env var and JSON loading |
| `src/config/proxy-config.json` | backend catalog and routing rules |

---

## Tutorial: how configuration works

Configuration is split into **two layers**:

1. **environment variables** for global process settings
2. **JSON file** for backends and routing rules

### 1. Environment variables

They are used for process-wide settings such as:

- which port the service listens on
- where Redis is located
- where the proxy configuration file lives

### 2. `proxy-config.json`

It is used to define:

- which backends exist
- how the service decides which backend should receive a request

---

## How to fill `proxy-config.json`

The current shape of the file is:

```json
{
  "backends": {
    "coingecko-api": "https://api.coingecko.com/api/v3",
    "cpp-rest-api": "http://cpp-rest-api:8080",
    "favorites-api": "http://favorites-api:8090"
  },
  "routes": [
    { "prefix": "/providers/coingecko", "backend": "coingecko-api" },
    { "prefix": "/internal/cpp-rest-api", "backend": "cpp-rest-api" },
    { "prefix": "/internal/favorites-api", "backend": "favorites-api" }
  ]
}
```

### What `backends` means

`backends` is a map:

- the **key** is a logical backend name
- the **value** is the real backend base URL

Example:

```json
"backends": {
  "coingecko-api": "https://api.coingecko.com/api/v3"
}
```

Here:

- `coingecko-api` is the internal name
- `https://api.coingecko.com/api/v3` is the real destination

### What `routes` means

`routes` is a list of rules.

Each rule says:

- if the request path starts with a given `prefix`
- then it should resolve to a given `backend`

Example:

```json
{ "prefix": "/providers/coingecko", "backend": "coingecko-api" }
```

That means:

- if something like `/providers/coingecko/markets` arrives
- the service should identify that request as belonging to `coingecko-api`

---

## How to add a new backend

Suppose you want to add another provider or internal API.

### Step 1: add it to `backends`

Example:

```json
"market-data-api": "http://market-data-api:8085"
```

### Step 2: add at least one rule to `routes`

Example:

```json
{ "prefix": "/providers/market-data", "backend": "market-data-api" }
```

### Expected result

Requests like:

```text
/providers/market-data/assets
```

should resolve to:

```text
http://market-data-api:8085
```

---

## Practical rules for filling the file

### 1. Use clear logical names

Good examples:

- `coingecko-api`
- `cpp-rest-api`
- `favorites-api`

Avoid ambiguous names such as:

- `api1`
- `backend-test`
- `service-x`

### 2. Use stable prefixes

Recommended examples:

- `/providers/coingecko`
- `/internal/favorites-api`

Avoid prefixes that are too generic, such as:

- `/api`
- `/service`

### 3. Do not mix logical names with URLs

The backend name and the backend URL should remain separate concepts.

That lets you change the URL later without rewriting all routing rules.

### 4. Keep one prefix per functional domain

Example:

- external providers under `/providers/*`
- internal APIs under `/internal/*`

That makes the system easier to understand and maintain.

---

## Example aligned with `crypto-dashboard`

Today the frontend uses these proxy routes:

```text
/api-cache/providers/coingecko/markets
/api-cache/providers/coingecko/coin/:id
```

Because nginx and the Angular dev proxy rewrite the `/api-cache` prefix, the useful path seen by this microservice is:

```text
/providers/coingecko/...
```

That is why this rule exists:

```json
{ "prefix": "/providers/coingecko", "backend": "coingecko-api" }
```

---

## What to check if `proxy-config.json` fails

### Error: file cannot be read

Check:

- that `PROXY_CONFIG_PATH` points to a real path
- that the file exists inside the container or local environment

### Error: invalid JSON

Check:

- missing commas
- unclosed braces
- field names different from `backends` and `routes`

### Error: backend does not exist

If a route points to:

```json
{ "prefix": "/x", "backend": "missing-backend" }
```

but `backends` does not contain that key, then the configuration is incomplete.

---

## Next evolution of the microservice

The next natural steps for this project are:

1. use `proxy-config.json` in a placeholder endpoint
2. resolve which backend matches an incoming path
3. forward the request to the real backend
4. integrate Redis into the caching flow
5. design the cache key and expiration policy

---

## Summary

Think about this microservice like this:

- **env vars** = how the process starts
- **proxy-config.json** = how the backend map is described
- **routes + handlers** = the microservice's own endpoints
- **Redis** = the cache layer that will be integrated later

If you are going to change `proxy-config.json`, the main rule is:

**define the backend in `backends` first, then create the matching rule in `routes`.**
