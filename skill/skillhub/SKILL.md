---
name: skillhub
description: >-
  Connect to a SkillHub registry — discover, install, and contribute agent
  skills. Use when the user mentions SkillHub, asks to find/install/publish a
  skill from a registry, wants to improve a registry skill, or gives you a
  SkillHub server URL to integrate with.
---

# SkillHub connector

SkillHub is a self-hosted agent skill registry. This skill teaches you how to
connect to **any** SkillHub instance and operate against it. It is
intentionally thin: the server documents itself, and the server's docs always
beat this file.

## Step 0 — locate the instance

You need a base URL. Resolve it in this order:

1. `$SKILLHUB_URL` environment variable, if set.
2. A URL the user gave you in conversation.
3. Ask the user: "What's your SkillHub server URL?"

Verify it: `curl -sf $BASE/healthz` must return `{"status":"ok"}`.

## Step 1 — read the live documentation (always do this)

```bash
curl -s $BASE/agents.md                 # full agent guide for THIS instance
curl -s $BASE/api/v1/meta/manifest      # machine-readable: auth modes + capabilities
```

The guide contains the complete endpoint reference, auth instructions, and
worked examples. **Treat it as the source of truth** — instances differ in
version and enabled capabilities, and this connector file may be older than
the server. Do not assume an endpoint exists; check `capabilities` in the
manifest first.

## Step 2 — authenticate

The manifest's `auth.modes` lists what this instance accepts. As of now most
instances run in dev mode: send `X-Mock-User-Id: <uuid>` (ask your user for
their user UUID). When `jwt` or `api_token` appear in `auth.modes`, prefer
them — header formats are documented in the guide.

Credentials come from your user or `$SKILLHUB_TOKEN` / `$SKILLHUB_USER_ID`.
Never invent credentials; a 401 means ask, a 403 means the user needs a
grant — don't retry either.

## Step 3 — operate

Common tasks, all detailed with request/response shapes in `/agents.md`:

- **Browse / find skills**: `GET $BASE/api/v1/skills` (public). No keyword
  search endpoint yet — filter locally, or use the semantic
  `POST /api/v1/skills/check-duplicate` to find similar existing skills.
- **Install a skill**: read `readme` / `install_command` from
  `GET /api/v1/skills/{id}` and follow it (confirm commands with your user).
- **Contribute**: open an iteration job (`POST /skills/{id}/iterations`),
  push base64 patches, `run-tests`, then `submit` — this creates a draft +
  proposal that humans review. Use a descriptive `agent` name and one intent
  per job.

## Conduct

Everything you do is audited under your user identity. Keep proposals small
and honest, never fabricate test output, and surface 403 denials to your
user instead of working around them.
