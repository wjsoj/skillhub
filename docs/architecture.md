# Architecture Notes

## Crate dependency graph

```
                  ┌─────────────────┐
                  │ skillhub-domain │  (pure: entities + repo traits)
                  └────────┬────────┘
                           │
       ┌──────────┬────────┼─────────┬────────────────┐
       │          │        │         │                │
       ▼          ▼        ▼         ▼                ▼
  infra      storage    search   notification        auth
  (SQLx)     (S3/FS)    (PG FTS) (log/email)       (JWT/OAuth)
       │          │        │         │                │
       └──────────┴────────┼─────────┴────────────────┘
                           ▼
                  ┌─────────────────┐
                  │  skillhub-app   │  (binary: axum router + DI)
                  └─────────────────┘
```

`skillhub-domain` is a no-IO crate. It declares entities (`Skill`,
`SkillVersion`, `Namespace`, `User`, `ApiToken`, `ReviewRequest`,
`AuditEntry`) and the `*Repository` traits operating on them.

Concrete `SQLx` implementations live in `skillhub-infra::repo`,
keeping SQL strings out of the domain layer.

`skillhub-app` is the only crate that depends on `axum` and is the
composition root: it builds the `AppState`, wires concrete
implementations of every trait, and mounts the HTTP router.

## HTTP surface

- `/healthz`, `/readyz` — liveness / readiness probes
- `/api/v1/...` — native REST API (auth, namespaces, skills, versions, search, reviews, tokens, users, admin)
- `/cli/...` — ClawHub-style compatibility surface for existing CLIs

## Auth model

- **Sessions** — JWT (HS256), short-lived, carried in cookie or `Authorization: Bearer`.
- **API tokens** — opaque `sk_<prefix>_<secret>`; the prefix is indexed, the secret is sha256-hashed at rest.
- **OAuth2** — providers declared in config, web flow + device-code flow.
- **RBAC** — `SUPER_ADMIN` (platform) + namespace roles (`Owner / Admin / Member`).

## Storage layout

Object keys: `skills/<namespace>/<slug>/<version>.tgz`.
Metadata + manifest live in Postgres (`skill_versions.manifest` JSONB).

## Search

`skills.search_vector` is a generated `tsvector` column with weights
A/B/C across `display_name / slug / description`, indexed by GIN.
Queries use `plainto_tsquery` + `ts_rank_cd` and apply visibility
filtering from the calling principal.
