-- Initial schema for SkillHub (Rust).
--
-- Mirrors the Java reference model: users, oauth identities, namespaces
-- with role-scoped membership, skills (visibility-controlled),
-- skill versions (with tags + status), reviews, audit log, api tokens.

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ---------- users ----------
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        TEXT NOT NULL UNIQUE,
    email           TEXT UNIQUE,
    display_name    TEXT,
    avatar_url      TEXT,
    is_super_admin  BOOLEAN NOT NULL DEFAULT FALSE,
    password_hash   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE oauth_identities (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider    TEXT NOT NULL,
    subject     TEXT NOT NULL,
    linked_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, subject)
);

-- ---------- namespaces ----------
CREATE TABLE namespaces (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            TEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL,
    scope           TEXT NOT NULL CHECK (scope IN ('global','team')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE namespace_members (
    namespace_id    UUID NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('owner','admin','member')),
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (namespace_id, user_id)
);

-- ---------- skills ----------
CREATE TABLE skills (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    namespace_id    UUID NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    slug            TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    description     TEXT,
    visibility      TEXT NOT NULL CHECK (visibility IN ('private','team','global')),
    downloads       BIGINT NOT NULL DEFAULT 0,
    stars           BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    search_vector   tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('simple', coalesce(display_name,'')), 'A') ||
        setweight(to_tsvector('simple', coalesce(slug,'')), 'B') ||
        setweight(to_tsvector('simple', coalesce(description,'')), 'C')
    ) STORED,
    UNIQUE (namespace_id, slug)
);

CREATE INDEX skills_search_idx ON skills USING GIN (search_vector);
CREATE INDEX skills_downloads_idx ON skills (downloads DESC);
CREATE INDEX skills_visibility_idx ON skills (visibility);

CREATE TABLE skill_versions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id            UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    version             TEXT NOT NULL,
    tags                TEXT[] NOT NULL DEFAULT '{}',
    manifest            JSONB NOT NULL,
    storage_key         TEXT NOT NULL,
    size_bytes          BIGINT NOT NULL,
    checksum_sha256     TEXT NOT NULL,
    status              TEXT NOT NULL CHECK (status IN ('pending','approved','rejected','yanked')),
    published_by        UUID NOT NULL REFERENCES users(id),
    published_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (skill_id, version)
);

CREATE INDEX skill_versions_skill_idx ON skill_versions (skill_id);
CREATE INDEX skill_versions_tags_idx ON skill_versions USING GIN (tags);

CREATE TABLE skill_stars (
    skill_id    UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    starred_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (skill_id, user_id)
);

CREATE TABLE skill_ratings (
    skill_id    UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    score       SMALLINT NOT NULL CHECK (score BETWEEN 1 AND 5),
    comment     TEXT,
    rated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (skill_id, user_id)
);

-- ---------- reviews ----------
CREATE TABLE review_requests (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_version_id    UUID NOT NULL REFERENCES skill_versions(id) ON DELETE CASCADE,
    kind                TEXT NOT NULL CHECK (kind IN ('publish','promote_to_global')),
    status              TEXT NOT NULL CHECK (status IN ('pending','approved','rejected')),
    requested_by        UUID NOT NULL REFERENCES users(id),
    reviewed_by         UUID REFERENCES users(id),
    note                TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_at          TIMESTAMPTZ
);

-- ---------- api tokens ----------
CREATE TABLE api_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    prefix          TEXT NOT NULL UNIQUE,
    hash            TEXT NOT NULL,
    scopes          TEXT[] NOT NULL DEFAULT '{}',
    expires_at      TIMESTAMPTZ,
    last_used_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------- audit ----------
CREATE TABLE audit_entries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id        UUID REFERENCES users(id),
    action          TEXT NOT NULL,
    target_type     TEXT NOT NULL,
    target_id       TEXT,
    payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX audit_entries_action_idx ON audit_entries (action, occurred_at DESC);
