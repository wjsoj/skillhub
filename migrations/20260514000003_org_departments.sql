-- Organizations, department tree (closure table), and per-department
-- memberships. Every namespace is rooted at exactly one department.
-- Cross-department access is *only* possible via cross_scope_grants.

CREATE TABLE organizations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE departments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    parent_id       UUID REFERENCES departments(id) ON DELETE RESTRICT,
    slug            TEXT NOT NULL,
    name            TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, slug)
);
CREATE INDEX departments_org_idx    ON departments (org_id);
CREATE INDEX departments_parent_idx ON departments (parent_id);

-- Closure table: every ancestor/descendant pair, including self (depth=0).
-- Maintained by the application layer; gives O(1) ancestor lookups.
CREATE TABLE department_closure (
    ancestor_id     UUID NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
    descendant_id   UUID NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
    depth           INTEGER NOT NULL CHECK (depth >= 0),
    PRIMARY KEY (ancestor_id, descendant_id)
);
CREATE INDEX department_closure_desc_idx ON department_closure (descendant_id);

CREATE TABLE department_memberships (
    department_id   UUID NOT NULL REFERENCES departments(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('director','manager','member')),
    granted_by      UUID REFERENCES users(id),
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (department_id, user_id)
);

-- Every namespace belongs to one department (the namespace "home"). Old rows
-- get NULL until backfilled by the operator.
ALTER TABLE namespaces
    ADD COLUMN department_id UUID REFERENCES departments(id) ON DELETE RESTRICT;
CREATE INDEX namespaces_department_idx ON namespaces (department_id);

-- Cross-department grants. Either grantee_department_id (give a whole
-- department access) or grantee_user_id (give one user access). Target is
-- either a department, namespace, or specific skill.
CREATE TABLE cross_scope_grants (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    grantee_department_id       UUID REFERENCES departments(id) ON DELETE CASCADE,
    grantee_user_id             UUID REFERENCES users(id) ON DELETE CASCADE,
    target_department_id        UUID REFERENCES departments(id) ON DELETE CASCADE,
    target_namespace_id         UUID REFERENCES namespaces(id) ON DELETE CASCADE,
    target_skill_id             UUID REFERENCES skills(id) ON DELETE CASCADE,
    scope                       TEXT NOT NULL CHECK (scope IN ('read','write','admin')),
    reason                      TEXT NOT NULL,
    granted_by                  UUID NOT NULL REFERENCES users(id),
    granted_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at                  TIMESTAMPTZ,
    revoked_at                  TIMESTAMPTZ,
    CHECK (
        (grantee_department_id IS NOT NULL) <> (grantee_user_id IS NOT NULL)
    ),
    CHECK (
        (target_department_id  IS NOT NULL)::int +
        (target_namespace_id   IS NOT NULL)::int +
        (target_skill_id       IS NOT NULL)::int = 1
    )
);
CREATE INDEX grants_grantee_dept_idx ON cross_scope_grants (grantee_department_id);
CREATE INDEX grants_grantee_user_idx ON cross_scope_grants (grantee_user_id);
CREATE INDEX grants_target_dept_idx  ON cross_scope_grants (target_department_id);
CREATE INDEX grants_target_ns_idx    ON cross_scope_grants (target_namespace_id);
CREATE INDEX grants_target_skill_idx ON cross_scope_grants (target_skill_id);
