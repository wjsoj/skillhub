# SkillHub Agent Guide

> You are probably an AI agent reading this. Good — this document is written
> for you. It is served live by the SkillHub instance at `{{BASE_URL}}` and
> always reflects what **this** deployment actually implements.
>
> Machine-readable companion: `GET {{BASE_URL}}/api/v1/meta/manifest`
> Installable connector skill: `GET {{BASE_URL}}/skill.md`

## What SkillHub is

SkillHub is a self-hosted **agent skill registry**: a place where humans and
agents publish, version, review, discover, and install agent skills (prompt
packs, SKILL.md bundles, tool definitions) under team/department namespaces
with audit and access control.

You can interact with it in two roles, often both:

1. **Consumer** — find and install skills for yourself or your user.
2. **Contributor** — improve skills through the *iteration harness*: open a
   sandboxed job, push patches, run tests, and submit the result as a
   reviewable proposal. Your work then flows through the same human review
   pipeline as everyone else's.

## 30-second onboarding

```bash
# 1. Is the server alive?
curl {{BASE_URL}}/healthz                      # → {"status":"ok"}

# 2. What can this instance do? (auth modes, implemented capabilities)
curl {{BASE_URL}}/api/v1/meta/manifest

# 3. Browse the catalog (public, no auth)
curl {{BASE_URL}}/api/v1/skills
```

That is genuinely all you need to start. Read on for auth and write operations.

## Authentication

Check `auth.modes` in the manifest for what this instance supports. Modes:

| Mode | Header | Status |
|---|---|---|
| `mock` | `X-Mock-User-Id: <uuid>` (+ optional `X-Mock-Username: <name>`) | **works today** — dev/local deployments only |
| `jwt` | `Authorization: Bearer <jwt>` | planned, not yet implemented |
| `api_token` | `Authorization: ApiToken sk_<prefix>_<secret>` | planned, not yet implemented |

Until `jwt`/`api_token` land, authenticate by sending `X-Mock-User-Id` with a
user UUID your operator gives you. Endpoints marked **auth** below return
`401 {"error":"unauthorized"}` without credentials.

Authorization is default-deny and layered: super-admin → skill collaborator
role → namespace role → department role (inherited down the department tree)
→ explicit cross-scope grant → public visibility. A denial is a
`403 {"error": "..."}` and is recorded in the audit log. If you are denied
cross-department access, the remedy is a *cross-scope grant* (see Grants),
not retrying.

## Error model

Every error is `{"error": "<message>"}` with a conventional status code:
`400` validation, `401` missing/bad credentials, `403` policy denial,
`404` not found, `409` conflict (duplicate, illegal state transition),
`500` internal. There is no error-code enum yet; branch on status code.

## Core concepts

- **Skill** — the aggregate root. Lives in a namespace, has visibility,
  tags, a manifest (JSON), a README, and versions.
- **SkillVersion** — an immutable published version (semver string).
- **VersionDraft → VersionProposal** — the change pipeline, like a pull
  request: create a draft (new manifest), open a proposal from it, reviewers
  comment/approve, a maintainer merges. Proposal states:
  `open → changes_requested | approved | rejected | withdrawn`,
  `approved → merged`. Illegal transitions return `409`.
- **IterationJob** — a sandboxed workspace for *you*. States:
  `queued → running → succeeded | failed | cancelled → submitted`.
- **Organization / Department / Grant** — the permission hierarchy.
  Department roles (`director | manager | member`) inherit downward;
  cross-department access requires an explicit grant
  (`read | write | admin`).
- **Collaborator** — per-skill role (`maintainer | writer | reader`),
  independent of namespace membership.

## Endpoint reference

Base path for everything below: `{{BASE_URL}}/api/v1`

### Skills (catalog)

| Method & path | Auth | Notes |
|---|---|---|
| `GET /skills` | no | Full catalog, sorted by installs then name |
| `GET /skills/{id}` | no | Single skill by UUID |

Skill object fields: `id`, `namespace_id`, `namespace_slug`,
`department_id?`, `slug`, `display_name`, `description?`, `visibility`,
`manifest` (JSON), `readme?`, `install_command?`, `repository_url?`,
`tags[]`, `downloads`, `install_count`, `stars`, `created_at`, `updated_at`.

> There is no server-side keyword search endpoint yet (`/search` is a stub).
> Fetch `GET /skills` and filter locally; for "does something like X already
> exist?" use the semantic duplicate check below — it is embedding-based and
> much better than string matching.

### Duplicate check (semantic similarity)

| Method & path | Auth | Notes |
|---|---|---|
| `POST /skills/check-duplicate` | **auth** | Call before creating anything new |

Request:

```json
{
  "display_name": "PDF table extractor",
  "slug": "pdf-table-extractor",
  "description": "Extract tables from PDFs into CSV",
  "readme": null,
  "manifest": null,
  "tags": ["pdf", "extraction"],
  "exclude_skill_id": null
}
```

Response: `{"query_hash", "model", "candidates": [{"hit": {...}, "confidence":
"high|medium|low", "suggested_action": "use_existing|review|inform"}]}`.

Honor `suggested_action`: `use_existing` (cosine ≥ 0.88) means open a
proposal on the existing skill instead of publishing a near-duplicate;
`review` means ask your user; `inform` is awareness only.

### Iteration harness — the agent contribution loop

This is the API designed specifically for you. All paths are under
`/skills/{skill_id}/iterations`.

| Method & path | Auth | Notes |
|---|---|---|
| `POST /skills/{sid}/iterations` | **auth** | Open a job → `201` job object |
| `GET /skills/{sid}/iterations` | no | List jobs |
| `GET /skills/{sid}/iterations/{jid}` | no | Job status |
| `POST .../iterations/{jid}/patches` | no | Stage a file change |
| `POST .../iterations/{jid}/run-tests` | no | Run a command in the sandbox |
| `POST .../iterations/{jid}/submit` | **auth** | Turn work into draft + proposal |
| `POST .../iterations/{jid}/cancel` | no | Abandon the job |

Worked loop:

```bash
SID=<skill uuid>; AUTH='-H "X-Mock-User-Id: <your-user-uuid>"'

# 1. Open a job — say who you are and what you intend
JOB=$(curl -s -X POST $AUTH {{BASE_URL}}/api/v1/skills/$SID/iterations \
  -H 'Content-Type: application/json' \
  -d '{"agent":"claude-code","intent":"fix broken regex in section 3","base_version_id":null}')
JID=$(echo "$JOB" | jq -r .id)

# 2. Push patches — content is base64; ops: write | delete | rename
curl -s -X POST {{BASE_URL}}/api/v1/skills/$SID/iterations/$JID/patches \
  -H 'Content-Type: application/json' \
  -d "{\"seq\":1,\"path\":\"SKILL.md\",\"op\":\"write\",\"data_b64\":\"$(base64 -w0 SKILL.md)\",\"new_path\":null}"

# 3. Run tests in the sandbox; inspect exit_code/stdout/stderr and iterate
curl -s -X POST {{BASE_URL}}/api/v1/skills/$SID/iterations/$JID/run-tests \
  -H 'Content-Type: application/json' -d '{"command":"./test.sh"}'

# 4. Submit — creates a VersionDraft + VersionProposal for human review
curl -s -X POST $AUTH {{BASE_URL}}/api/v1/skills/$SID/iterations/$JID/submit \
  -H 'Content-Type: application/json' \
  -d '{"target_version":"1.2.1","title":"Fix regex in section 3","summary":null,"body":"Found via failing test X."}'
# → {"draft_id": "...", "proposal_id": "...", "job_state": "submitted"}
```

Sandbox rules: your job runs in an isolated temp workspace with resource
limits; network is restricted. Patches apply to the workspace only — nothing
touches a published version until a human merges your proposal.

### Drafts & proposals (review pipeline)

All under `/skills/{skill_id}`:

| Method & path | Auth | Notes |
|---|---|---|
| `POST .../drafts` | **auth** | `{base_version_id?, target_version, manifest, summary?}` → `{draft_id}` |
| `POST .../proposals` | **auth** | `{draft_id, title, body?}` → `201` proposal |
| `GET .../proposals` | no | List |
| `GET .../proposals/{pid}` | no | Read one |
| `POST .../proposals/{pid}/reviews` | **auth** | `{verdict: comment\|approve\|request_changes\|reject, body?}` |
| `POST .../proposals/{pid}/decide` | **auth** | `{state: <next state>}` — `409` on illegal transition |
| `POST .../proposals/{pid}/merge` | **auth** | Requires state `approved`, else `409` |

After submitting a proposal, poll `GET .../proposals/{pid}` for state
changes; respond to `changes_requested` by opening a fresh iteration job.

### Collaborators

| Method & path | Auth | Notes |
|---|---|---|
| `GET /skills/{sid}/collaborators` | no | List |
| `POST /skills/{sid}/collaborators` | **auth** | `{user_id, role: maintainer\|writer\|reader}` — `409` if present |
| `POST /skills/{sid}/collaborators/{uid}` | **auth** | Change role: `{role}` |
| `DELETE /skills/{sid}/collaborators/{uid}` | no | Remove → `{"removed":true}` |

### Organizations, departments, grants

| Method & path | Auth | Notes |
|---|---|---|
| `GET /orgs/{org_id}/departments` | no | Flat list (`parent_id` gives the tree) |
| `POST /orgs/{org_id}/departments` | no | `{slug, name, parent_id?}` |
| `POST /departments/{id}/members` | **auth** | `{user_id, role: director\|manager\|member}` |
| `GET /departments/{id}/members` | no | (currently returns an empty stub list) |
| `POST /grants` | **auth** | Cross-scope grant, see below |

Grant request — exactly one `grantee_*` and exactly one `target_*`:

```json
{
  "grantee_user_id": "<uuid>",
  "grantee_department_id": null,
  "target_skill_id": "<uuid>",
  "target_namespace_id": null,
  "target_department_id": null,
  "scope": "read",
  "reason": "agent needs to read dept-B skills for task X",
  "expires_at": null
}
```

### Admin

| Method & path | Auth | Notes |
|---|---|---|
| `POST /admin/reindex-embeddings` | no (operator-only by convention) | Rebuild semantic index → `{indexed, model, skipped}` |

### Health

`GET {{BASE_URL}}/healthz` → `{"status":"ok"}` ·
`GET {{BASE_URL}}/readyz` → `{"status":"ready"}` (root level, not under `/api/v1`).

## Not implemented yet — do not call

These route groups exist but are empty stubs and will 404/405:
`/api/v1/auth`, `/api/v1/tokens`, `/api/v1/users`, `/api/v1/search`,
`/api/v1/versions`, `/api/v1/reviews`, `/api/v1/namespaces`, and the `/cli`
compatibility layer. The manifest's `capabilities` map tells you the current
truth at runtime — trust it over any cached copy of this guide.

## Recommended workflows

**Find & install a skill for your user**
1. `GET /skills`, filter by tags/description locally.
2. Read the chosen skill's `readme` and `manifest` from `GET /skills/{id}`.
3. If `install_command` is set, run it (show your user first); otherwise
   follow the README's install instructions.

**Contribute an improvement**
1. `POST /skills/check-duplicate` if you're proposing something new.
2. Open an iteration job → patch → test → repeat until tests pass.
3. `submit` with a clear title and body explaining what and why.
4. Poll the proposal; address `changes_requested` with a new iteration.

**You were denied (403)**
Don't retry. Tell your user which action was denied; an admin can issue a
cross-scope grant (`POST /grants`) or add you as a collaborator.

## Etiquette for agents

- Identify yourself: set a meaningful `agent` name on iteration jobs.
- One intent per iteration job; small, reviewable proposals.
- Never fabricate test results — `run-tests` output is recorded server-side.
- Everything you do is written to an audit log under your user identity.
