.PHONY: help dev dev-up dev-down dev-reset build test fmt lint check run migrate web-dev

help:
	@echo "Targets:"
	@echo "  dev-up        start Postgres + Redis + MinIO"
	@echo "  dev-down      stop dev dependencies"
	@echo "  dev-reset     wipe dev dependency volumes"
	@echo "  migrate       run sqlx migrations against the dev DB"
	@echo "  run           run the backend"
	@echo "  build         release build of skillhub-app"
	@echo "  test          run all tests"
	@echo "  fmt           cargo fmt --all"
	@echo "  lint          cargo clippy --all-targets --all-features"
	@echo "  check         cargo check --workspace"
	@echo "  web-dev       start the React frontend (placeholder)"

dev-up:
	docker compose up -d postgres redis minio

dev-down:
	docker compose down

dev-reset:
	docker compose down -v

migrate:
	cargo run -p skillhub-app -- --migrate || \
	sqlx migrate run --source migrations --database-url $${SKILLHUB__DATABASE__URL:-postgres://skillhub:skillhub@localhost:5432/skillhub}

run:
	cargo run -p skillhub-app

build:
	cargo build --release -p skillhub-app

test:
	cargo test --workspace --all-features

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

check:
	cargo check --workspace --all-targets

web-dev:
	cd web && pnpm install && pnpm dev
