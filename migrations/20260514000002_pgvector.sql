-- pgvector + semantic duplicate-detection storage.
--
-- One row per (skill, version, source). The `source` axis lets us hold
-- both the "skill-level" embedding (computed from current metadata) and
-- per-version embeddings (drift detection over time).

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE skill_embeddings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id        UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    version_id      UUID REFERENCES skill_versions(id) ON DELETE CASCADE,
    source          TEXT NOT NULL CHECK (source IN ('skill','version','draft')),
    model           TEXT NOT NULL,
    dim             INTEGER NOT NULL,
    embedding       vector(1024) NOT NULL,
    content_hash    TEXT NOT NULL,
    text_preview    TEXT,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- UNIQUE on an expression (a table-level UNIQUE constraint can't take
-- function calls). NULL version_id is normalised to the zero UUID so
-- "the skill-level embedding" is uniquely keyed on (skill, source, model).
CREATE UNIQUE INDEX skill_embeddings_unique_idx
    ON skill_embeddings (
        skill_id,
        (COALESCE(version_id, '00000000-0000-0000-0000-000000000000'::uuid)),
        source,
        model
    );

CREATE INDEX skill_embeddings_skill_idx ON skill_embeddings (skill_id);
CREATE INDEX skill_embeddings_vector_idx
    ON skill_embeddings USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX skills_name_trgm_idx       ON skills USING GIN (display_name gin_trgm_ops);
CREATE INDEX skills_desc_trgm_idx       ON skills USING GIN (coalesce(description,'') gin_trgm_ops);
