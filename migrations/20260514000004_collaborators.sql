-- Skill-level collaborators, decoupled from namespace membership.
-- Namespace membership says "you may create skills here";
-- collaborator role says "you may modify *this* skill".

CREATE TABLE skill_collaborators (
    skill_id    UUID NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role        TEXT NOT NULL CHECK (role IN ('maintainer','writer','reader')),
    added_by    UUID NOT NULL REFERENCES users(id),
    added_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (skill_id, user_id)
);
CREATE INDEX skill_collaborators_user_idx ON skill_collaborators (user_id);
