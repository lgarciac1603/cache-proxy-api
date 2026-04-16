FROM rust:slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN groupadd -r appuser && useradd -r -g appuser appuser

COPY --from=builder /app/target/release/cache-proxy-api /usr/local/bin/cache-proxy-api

RUN chown -R appuser:appuser /app

USER appuser

EXPOSE 3000

CMD ["cache-proxy-api"]
