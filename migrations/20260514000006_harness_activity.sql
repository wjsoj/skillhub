-- AI iteration harness + per-skill activity timeline.

CREATE TABLE iteration_jobs (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id            UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    base_version_id     UUID REFERENCES skill_versions(id) ON DELETE SET NULL,
    started_by          UUID NOT NULL REFERENCES users(id),
    agent               TEXT NOT NULL,
    intent              TEXT NOT NULL,
    state               TEXT NOT NULL CHECK (state IN
                            ('queued','running','succeeded','failed','cancelled','submitted')),
    workspace_key       TEXT NOT NULL,
    log_uri             TEXT,
    error               TEXT,
    submitted_proposal  UUID REFERENCES version_proposals(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at          TIMESTAMPTZ,
    finished_at         TIMESTAMPTZ
);
CREATE INDEX iteration_jobs_skill_idx ON iteration_jobs (skill_id, state);
CREATE INDEX iteration_jobs_started_by_idx ON iteration_jobs (started_by);

CREATE TABLE iteration_patches (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id              UUID NOT NULL REFERENCES iteration_jobs(id) ON DELETE CASCADE,
    seq                 INTEGER NOT NULL,
    path                TEXT NOT NULL,
    op                  TEXT NOT NULL CHECK (op IN ('write','delete','rename')),
    new_path            TEXT,
    content_sha256      TEXT,
    size_bytes          BIGINT,
    applied_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (job_id, seq)
);

CREATE TABLE iteration_test_runs (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id              UUID NOT NULL REFERENCES iteration_jobs(id) ON DELETE CASCADE,
    command             TEXT NOT NULL,
    exit_code           INTEGER,
    duration_ms         INTEGER,
    stdout_uri          TEXT,
    stderr_uri          TEXT,
    started_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at         TIMESTAMPTZ
);

-- Activity timeline: append-only per-skill story log.
CREATE TABLE activity_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id        UUID REFERENCES skills(id) ON DELETE CASCADE,
    namespace_id    UUID REFERENCES namespaces(id) ON DELETE CASCADE,
    actor_id        UUID REFERENCES users(id),
    verb            TEXT NOT NULL,
    payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX activity_skill_idx     ON activity_events (skill_id, occurred_at DESC);
CREATE INDEX activity_namespace_idx ON activity_events (namespace_id, occurred_at DESC);
CREATE INDEX activity_actor_idx     ON activity_events (actor_id, occurred_at DESC);
