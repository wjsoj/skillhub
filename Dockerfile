# syntax=docker/dockerfile:1.7

FROM rust:1.82-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY migrations ./migrations
COPY config ./config
# Agent-facing docs are embedded into the binary via include_str!
COPY docs/agent-guide.md ./docs/agent-guide.md
COPY skill ./skill

RUN cargo build --release -p skillhub-app

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/skillhub /usr/local/bin/skillhub
COPY --from=builder /app/migrations ./migrations
COPY --from=builder /app/config ./config

ENV RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["skillhub"]
