-- Version lineage, drafts, and proposal flow.
--
-- Lineage rules:
--   * Every published `skill_versions` row has a `lineage_id` (group key).
--   * `parent_version_id` points at the prior version in that lineage.
--   * Forking a new lineage = new `lineage_id`, parent_version_id may
--     cross lineages.

ALTER TABLE skills           ADD COLUMN revision BIGINT NOT NULL DEFAULT 0;
ALTER TABLE skill_versions   ADD COLUMN parent_version_id UUID
    REFERENCES skill_versions(id) ON DELETE SET NULL;
ALTER TABLE skill_versions   ADD COLUMN lineage_id UUID NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE skill_versions   ADD COLUMN etag TEXT NOT NULL DEFAULT '';
CREATE INDEX skill_versions_lineage_idx ON skill_versions (lineage_id);
CREATE INDEX skill_versions_parent_idx  ON skill_versions (parent_version_id);

-- Drafts: in-progress changes that have not yet become a proposal.
CREATE TABLE version_drafts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id            UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    base_version_id     UUID REFERENCES skill_versions(id) ON DELETE SET NULL,
    target_version      TEXT NOT NULL,
    manifest            JSONB NOT NULL DEFAULT '{}'::jsonb,
    storage_key         TEXT,
    size_bytes          BIGINT,
    checksum_sha256     TEXT,
    summary             TEXT,
    created_by          UUID NOT NULL REFERENCES users(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX version_drafts_skill_idx ON version_drafts (skill_id);

-- Proposals: "please review this draft".
CREATE TABLE version_proposals (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id            UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    draft_id            UUID NOT NULL REFERENCES version_drafts(id) ON DELETE CASCADE,
    state               TEXT NOT NULL CHECK (state IN
                            ('open','changes_requested','approved','rejected','merged','withdrawn')),
    title               TEXT NOT NULL,
    body                TEXT,
    opened_by           UUID NOT NULL REFERENCES users(id),
    opened_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_by          UUID REFERENCES users(id),
    decided_at          TIMESTAMPTZ,
    merged_version_id   UUID REFERENCES skill_versions(id) ON DELETE SET NULL
);
CREATE INDEX version_proposals_skill_idx ON version_proposals (skill_id, state);

CREATE TABLE proposal_reviews (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposal_id         UUID NOT NULL REFERENCES version_proposals(id) ON DELETE CASCADE,
    reviewer_id         UUID NOT NULL REFERENCES users(id),
    verdict             TEXT NOT NULL CHECK (verdict IN ('comment','approve','request_changes','reject')),
    body                TEXT,
    reviewed_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX proposal_reviews_proposal_idx ON proposal_reviews (proposal_id);
