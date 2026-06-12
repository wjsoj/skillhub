<div align="center">

# SkillHub

**An enterprise-grade, self-hosted agent skill registry — Rust edition.**

Publish, version, discover, and install agent skills under team/department
namespaces, with review, audit, and fine-grained access control.

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
[![Stack](https://img.shields.io/badge/stack-Axum_%C2%B7_SQLx_%C2%B7_Postgres-informational)](#tech-stack)

</div>

---

SkillHub is a Rust reimplementation of the [iflytek/skillhub](https://github.com/iflytek/skillhub)
reference design — same product shape, rebuilt on an async Rust stack with a
clean, layered architecture. It treats **AI agents as first-class API
consumers**: any agent can self-onboard from nothing but a base URL, read a
guide written for it, and start discovering and contributing skills.

## Features

- **Agent-first onboarding** — every instance serves its own live, always-accurate API guide (`/agents.md`), a machine-readable capability manifest, and an installable connector skill. An agent needs only the URL.
- **Semantic duplicate detection** — embeddings (pgvector) + a pluggable `Embedder` (OpenAI-compatible HTTP or a deterministic offline stub). Pre-publish checks return top-K candidates with confidence bands.
- **Collaboration & versioning** — a Draft → Proposal → Review → Merge pipeline (like pull requests), with per-skill collaborator roles and version lineage.
- **AI iteration harness** — a sandboxed workspace where agents open a job, push patches, run tests, and submit results as reviewable proposals through the same pipeline humans use.
- **Department-level permissions** — `Organization → Department (tree) → Namespace → Skill` with default-deny policy evaluation; cross-department access only via explicit, auditable grants.
- **Built for ops** — `tracing` + Prometheus, compile-time-checked SQL, and a slim multi-stage release image.

## Agent-first onboarding

Agents are a primary audience. Every deployment documents itself — the docs
are compiled into the binary and rendered with the instance's `public_base_url`,
so they can never drift from what the running server implements.

| Endpoint | What it is |
|---|---|
| `GET /llms.txt` | Tiny discovery index ([llms.txt](https://llmstxt.org) convention) |
| `GET /agents.md` | Complete agent guide — auth, endpoint contracts, worked workflows |
| `GET /skill.md` | Installable connector skill (also in [`skill/skillhub/`](skill/skillhub/SKILL.md)) |
| `GET /api/v1/meta/manifest` | Machine-readable manifest: auth modes + implemented capabilities |

To hook an agent up, just give it the URL ("connect to my SkillHub at
`https://hub.example.com`") or install the connector skill:

```bash
mkdir -p ~/.claude/skills/skillhub
curl -s https://hub.example.com/skill.md > ~/.claude/skills/skillhub/SKILL.md
```

> [!TIP]
> The manifest's `capabilities` map is the runtime source of truth. Agents are
> told to trust it over any cached copy of the guide.

## Tech stack

| Layer | Choice |
|---|---|
| HTTP | Axum 0.7 + Tower / Tower-HTTP |
| Async | Tokio |
| Database | PostgreSQL 16 + SQLx (compile-time checked) |
| Cache | Redis 7 |
| Search | Postgres FTS (`tsvector` + GIN) + pgvector |
| Storage | Skill content in Postgres (TEXT/JSONB) — no object store needed |
| Auth | argon2 + JWT + OAuth2 + prefix-hashed tokens |
| Observability | `tracing` + Prometheus exporter |
| Frontend | React 19 + TypeScript + Vite (under `web/`) |

## Architecture

```
┌─────────────┐  ┌─────────────┐  ┌──────────────┐
│   Web UI    │  │  AI Agents  │  │  REST API    │
│  (React 19) │  │             │  │              │
└──────┬──────┘  └──────┬──────┘  └──────┬───────┘
       │                │                │
       └────────────────┼────────────────┘
                        ▼
                 ┌─────────────┐
                 │   Axum app  │  Auth · RBAC · Services
                 │   (Rust)    │  OAuth2 · API Tokens · Audit
                 └──────┬──────┘
                        │
           ┌────────────┴────────────┐
           ▼                         ▼
      PostgreSQL 16              Redis 7
   (skill content + metadata)   (cache)
```

> Skills are text — a `SKILL.md` plus a JSON manifest and a few small files —
> so their content lives directly in Postgres (`TEXT` / `JSONB`). There's no
> object store in the default stack. An `ObjectStore` trait
> (`skillhub-storage`) exists for the day skills carry large binary
> attachments; until then it's optional and unused.

Crate boundaries enforce a clean, dependency-inverted architecture:

- `skillhub-domain` knows nothing about HTTP, DB, or IO. It defines entities and the *repository traits*.
- `skillhub-infra` provides SQLx-backed implementations of those traits, plus config and pool plumbing.
- `skillhub-auth`, `skillhub-storage`, `skillhub-search`, `skillhub-notification` are independent capabilities.
- `skillhub-app` is the only binary: it wires everything together and exposes the HTTP surface.

```
skillhub/
├── crates/
│   ├── skillhub-app/            # binary: HTTP server + composition root
│   ├── skillhub-domain/         # pure domain: entities + repo traits
│   ├── skillhub-infra/          # config, PgPool, Redis, sqlx repos
│   ├── skillhub-auth/           # password/JWT/OAuth2/tokens + policy evaluator
│   ├── skillhub-storage/        # ObjectStore trait (optional; for large attachments)
│   ├── skillhub-search/         # Postgres FTS + semantic duplicate detection
│   ├── skillhub-embeddings/     # Embedder trait (HTTP / stub backends)
│   ├── skillhub-harness/        # AI iteration workspace + sandboxed runner
│   └── skillhub-notification/   # notification dispatch
├── migrations/                  # sqlx migrations
├── config/                      # default.toml + env overrides
├── skill/skillhub/              # the installable connector SKILL.md
├── docs/                        # design + agent guide + architecture
├── web/                         # React 19 frontend
├── docker-compose.yml           # dev deps: postgres, redis
└── Dockerfile                   # multi-stage release image
```

## Getting started

### Prerequisites

- Rust 1.80+ (the toolchain is pinned via `rust-toolchain.toml`)
- Docker + Docker Compose (for Postgres + Redis)
- Node.js + pnpm (only if working on the `web/` frontend)

### Quick start

```bash
cp .env.example .env
make dev-up        # start Postgres + Redis
make migrate       # apply the schema
make run           # cargo run -p skillhub-app
```

Then:

- API: <http://localhost:8080>
- Health: `GET /healthz`
- REST: `GET /api/v1/...`
- Agent guide: `GET /agents.md`
- ClawHub registry: `GET /clawhub/...`

### Install skills with the real `clawhub` CLI

This registry implements the [`clawhub`](https://www.npmjs.com/package/clawhub)
CLI's registry protocol under the `/clawhub` prefix, so the upstream CLI can
search and install straight from it:

```bash
clawhub --registry http://localhost:8080/clawhub search csv
clawhub --registry http://localhost:8080/clawhub install csv-clean
# → SKILL.md + manifest.json written to ./skills/csv-clean/
```

Reads are anonymous and see only `global` skills; pass one of this registry's
JWTs as the clawhub token to install team/private skills you have access to.
Since skills are text, "download" zips the `SKILL.md` + manifest from
Postgres — no object store.

> [!NOTE]
> Without `SKILLHUB__EMBEDDER__URL` set, the server falls back to a
> deterministic stub embedder — fine for local dev and CI, but not
> semantically meaningful. Point it at any OpenAI-compatible embeddings
> endpoint (Ollama, vLLM, OpenAI, …) for real similarity search.

### Frontend

```bash
make web-dev       # cd web && pnpm install && pnpm dev
```

The Vite dev server runs on port 5173 and proxies `/api` to the backend.

## Configuration

Configuration loads from `config/default.toml`, then environment variables
override any key using the `SKILLHUB__<SECTION>__<KEY>` convention (double
underscores). See [`.env.example`](.env.example) for the common set:

| Variable | Description |
|---|---|
| `SKILLHUB__SERVER__PUBLIC_BASE_URL` | Public URL; renders into the agent docs |
| `SKILLHUB__DATABASE__URL` | Postgres connection string |
| `SKILLHUB__REDIS__URL` | Redis connection string |
| `SKILLHUB__AUTH__JWT_SECRET` | JWT signing secret (change in production) |
| `SKILLHUB__EMBEDDER__URL` | Optional OpenAI-compatible embeddings endpoint |

## Development

Common `make` targets:

| Target | Description |
|---|---|
| `make dev-up` / `dev-down` / `dev-reset` | Manage Postgres + Redis |
| `make migrate` | Run sqlx migrations against the dev DB |
| `make run` | Run the backend |
| `make build` | Release build of `skillhub-app` |
| `make test` | Run the full workspace test suite |
| `make fmt` / `make lint` | `cargo fmt` / `cargo clippy -D warnings` |
| `make web-dev` | Start the React frontend |

## Roadmap

- **M0 — scaffold:** workspace, domain traits, schema, app skeleton. ✅
- **M1 — vertical slice:** users + namespaces + skills + publish + Postgres FTS. ✅
- **M2 — governance:** reviews, audit, RBAC, JWT & API tokens. ✅

M1 + M2 are implemented end-to-end: real registration/login with Argon2 + JWT,
API tokens (`Authorization: ApiToken sk_…`), user/namespace directories,
skill creation & version publishing, weighted Postgres full-text search,
a cross-skill review queue, stars, and a `clawhub`-compatible `/cli/install`
that resolves a skill and counts the install. Authorization is enforced
throughout (namespace/skill ownership checks, default-deny visibility
filtering, PII redaction).

Out of scope by choice: third-party OAuth/SSO (username-password + JWT +
API tokens cover the self-hosted case), and an object-store backend (skill
content is text in Postgres; only worth adding if skills grow large binary
attachments).

See [docs/design.md](docs/design.md) for the full design narrative and
[docs/architecture.md](docs/architecture.md) for the architecture overview.

---

Licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
