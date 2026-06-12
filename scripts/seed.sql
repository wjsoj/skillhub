-- Idempotent demo seed. Builds:
--   1 org · 3 departments (data-eng, finance, platform)
--   4 users · 3 namespaces · 4 fully-fleshed skills (manifest + README + tags)
-- so the UI has fixtures matching the IDs hard-coded in the React routes.

INSERT INTO organizations (id, slug, name)
VALUES ('10000000-0000-0000-0000-0000000000aa', 'acme', 'Acme Corp')
ON CONFLICT (id) DO NOTHING;

INSERT INTO departments (id, org_id, parent_id, slug, name)
VALUES
  ('20000000-0000-0000-0000-00000000d001', '10000000-0000-0000-0000-0000000000aa', NULL, 'platform', 'Platform'),
  ('20000000-0000-0000-0000-00000000d002', '10000000-0000-0000-0000-0000000000aa', '20000000-0000-0000-0000-00000000d001', 'data-eng', 'Data Engineering'),
  ('20000000-0000-0000-0000-00000000d003', '10000000-0000-0000-0000-0000000000aa', '20000000-0000-0000-0000-00000000d001', 'finance', 'Finance')
ON CONFLICT (id) DO NOTHING;

INSERT INTO department_closure (ancestor_id, descendant_id, depth) VALUES
  ('20000000-0000-0000-0000-00000000d001', '20000000-0000-0000-0000-00000000d001', 0),
  ('20000000-0000-0000-0000-00000000d002', '20000000-0000-0000-0000-00000000d002', 0),
  ('20000000-0000-0000-0000-00000000d003', '20000000-0000-0000-0000-00000000d003', 0),
  ('20000000-0000-0000-0000-00000000d001', '20000000-0000-0000-0000-00000000d002', 1),
  ('20000000-0000-0000-0000-00000000d001', '20000000-0000-0000-0000-00000000d003', 1)
ON CONFLICT DO NOTHING;

INSERT INTO users (id, username, email, display_name, is_super_admin) VALUES
  ('00000000-0000-0000-0000-000000000001', 'ada',    'ada@acme.dev',    'Ada Lovelace', false),
  ('00000000-0000-0000-0000-000000000002', 'bob',    'bob@acme.dev',    'Bob Builder',  false),
  ('00000000-0000-0000-0000-000000000003', 'carol',  'carol@acme.dev',  'Carol Quinn',  false),
  ('00000000-0000-0000-0000-000000000009', 'admin',  'admin@acme.dev',  'Platform Admin', true)
ON CONFLICT (id) DO NOTHING;

INSERT INTO department_memberships (department_id, user_id, role, granted_by) VALUES
  ('20000000-0000-0000-0000-00000000d002', '00000000-0000-0000-0000-000000000001', 'manager', '00000000-0000-0000-0000-000000000009'),
  ('20000000-0000-0000-0000-00000000d002', '00000000-0000-0000-0000-000000000002', 'member',  '00000000-0000-0000-0000-000000000009'),
  ('20000000-0000-0000-0000-00000000d003', '00000000-0000-0000-0000-000000000003', 'manager', '00000000-0000-0000-0000-000000000009')
ON CONFLICT (department_id, user_id) DO NOTHING;

INSERT INTO namespaces (id, slug, display_name, scope, department_id) VALUES
  ('30000000-0000-0000-0000-0000000000a1', 'data-eng', 'Data Engineering', 'team',   '20000000-0000-0000-0000-00000000d002'),
  ('30000000-0000-0000-0000-0000000000a2', 'finance',  'Finance',          'team',   '20000000-0000-0000-0000-00000000d003'),
  ('30000000-0000-0000-0000-0000000000a3', 'common',   'Common',           'global', '20000000-0000-0000-0000-00000000d001')
ON CONFLICT (id) DO NOTHING;

-- ───────────────────── Skill: pdf-parser ─────────────────────
INSERT INTO skills (id, namespace_id, slug, display_name, description, visibility,
                    manifest, readme, install_command, repository_url, tags, install_count)
VALUES (
  '11111111-1111-1111-1111-111111111111',
  '30000000-0000-0000-0000-0000000000a1',
  'pdf-parser', 'PDF parser',
  'Extract text and tables from PDF documents — with OCR fallback for scanned pages.',
  'team',
  $manifest$ {
    "version": "0.2.0",
    "license": "Apache-2.0",
    "author": "ada@acme.dev",
    "category": "document-processing",
    "entrypoint": "skill.py:run",
    "runtime": { "python": ">=3.10" },
    "inputs": [
      { "name": "path", "type": "string", "required": true, "description": "Path to a .pdf file" },
      { "name": "ocr",  "type": "boolean", "default": false,  "description": "Force OCR even when a text layer exists" },
      { "name": "pages","type": "string|null", "default": null, "description": "Page range, e.g. '1-5' or '3,7,9'" }
    ],
    "outputs": [
      { "name": "text",   "type": "string" },
      { "name": "tables", "type": "array<object>" },
      { "name": "meta",   "type": "object", "description": "Title, author, page count" }
    ],
    "dependencies": ["pdfplumber>=0.11", "tesseract-ocr>=5"],
    "files": [
      { "path": "SKILL.md",         "size": 4831, "kind": "doc" },
      { "path": "skill.py",         "size": 6204, "kind": "code" },
      { "path": "tests/fixtures/",  "size": null, "kind": "dir" },
      { "path": "tests/test_parser.py", "size": 2118, "kind": "test" },
      { "path": "pyproject.toml",   "size":  412, "kind": "config" }
    ]
  } $manifest$::jsonb,
  $readme$
# PDF parser

Extract **text** and **tables** from PDF documents — with an automatic OCR
fallback when a page has no text layer.

## Why this skill

Existing PDF tooling in the org assumes a text layer is always present.
Invoices and scanned contracts break it. This skill keeps a fast text-layer
path *and* drops to Tesseract OCR when the layer is empty.

## Triggers

Use when the user mentions:

- `.pdf` files
- "extract text", "pull tables", "OCR a scan", "read this invoice"
- A folder of PDFs to batch-process

## Capabilities

- Native text-layer extraction via `pdfplumber`
- Table detection with column heuristics
- OCR fallback (`tesseract`) under a configurable minimum-chars threshold
- Per-page selection and metadata (title, author, page count)

## Usage

```python
from pdf_parser import run

result = run(path="contract.pdf", ocr=False)
print(result["text"][:200])
for table in result["tables"]:
    print(table)
```

## Configuration

| Input  | Type           | Default | Notes                           |
| ------ | -------------- | ------- | ------------------------------- |
| path   | string         | —       | Local path or `file://` URL     |
| ocr    | boolean        | false   | Force OCR even with text layer  |
| pages  | string \| null | null    | Range syntax: `1-3,5` etc.      |

## Edge cases

- Encrypted PDFs return `error: "locked"` without throwing.
- Mixed-layer PDFs (some pages text, some scanned) auto-fall back per page.
- Hand-drawn tables are not extracted; we surface them as raw text only.

## Changelog

- **0.2.0** — OCR fallback, batch mode, per-page selection.
- **0.1.0** — Initial release: text + tables.
  $readme$,
  'clawhub install data-eng/pdf-parser',
  'https://git.acme.dev/data-eng/pdf-parser',
  ARRAY['pdf','text-extraction','tables','ocr','document-processing'],
  847
)
ON CONFLICT (id) DO UPDATE SET
  description = EXCLUDED.description,
  manifest = EXCLUDED.manifest,
  readme = EXCLUDED.readme,
  install_command = EXCLUDED.install_command,
  repository_url = EXCLUDED.repository_url,
  tags = EXCLUDED.tags,
  install_count = EXCLUDED.install_count;

-- ───────────────────── Skill: pdf-extract ─────────────────────
INSERT INTO skills (id, namespace_id, slug, display_name, description, visibility,
                    manifest, readme, install_command, repository_url, tags, install_count)
VALUES (
  '22222222-2222-2222-2222-222222222222',
  '30000000-0000-0000-0000-0000000000a1',
  'pdf-extract', 'PDF extract',
  'Pull structured data out of PDFs using deprecated v1 API (kept for backwards compatibility).',
  'team',
  $manifest$ {
    "version": "1.4.2",
    "license": "Apache-2.0",
    "author": "data-eng@acme.dev",
    "category": "document-processing",
    "deprecated": true,
    "deprecation_note": "Prefer data-eng/pdf-parser for new work. This skill remains for v1-API compatibility.",
    "entrypoint": "extract.py:extract",
    "runtime": { "python": ">=3.9" },
    "inputs": [
      { "name": "file", "type": "string", "required": true },
      { "name": "schema", "type": "object", "required": false, "description": "Optional Pydantic-style schema" }
    ],
    "outputs": [
      { "name": "records", "type": "array<object>" }
    ],
    "dependencies": ["pypdf2>=3", "regex>=2024"],
    "files": [
      { "path": "SKILL.md",  "size": 2103, "kind": "doc" },
      { "path": "extract.py","size": 3998, "kind": "code" },
      { "path": "schemas/",  "size": null, "kind": "dir" }
    ]
  } $manifest$::jsonb,
  $readme$
# PDF extract (deprecated)

> ⚠️ This skill is the v1-era PDF tool. New work should use
> [`data-eng/pdf-parser`](../pdf-parser). We keep this around for
> agents pinned to the v1 schema.

## What it does

Given a PDF and an optional schema, returns a flat list of records
matching keyed-out fields (invoice number, totals, line items).

## Why it stays

A handful of upstream jobs still expect the v1 record shape. Rather
than break them, we keep this skill installable but warn loudly in
the manifest (`deprecated: true`).

## Migration

```text
- v1 (this skill):  extract(file, schema) -> [records]
- v2 (pdf-parser):  run(path, ocr) -> {text, tables, meta}
```

The mapping is non-trivial — talk to data-eng before migrating.
  $readme$,
  'clawhub install data-eng/pdf-extract',
  'https://git.acme.dev/data-eng/pdf-extract',
  ARRAY['pdf','legacy','v1-compat'],
  312
)
ON CONFLICT (id) DO UPDATE SET
  description = EXCLUDED.description,
  manifest = EXCLUDED.manifest,
  readme = EXCLUDED.readme,
  install_command = EXCLUDED.install_command,
  repository_url = EXCLUDED.repository_url,
  tags = EXCLUDED.tags,
  install_count = EXCLUDED.install_count;

-- ───────────────────── Skill: finance-reconciler ─────────────────────
INSERT INTO skills (id, namespace_id, slug, display_name, description, visibility,
                    manifest, readme, install_command, repository_url, tags, install_count)
VALUES (
  '33333333-3333-3333-3333-333333333333',
  '30000000-0000-0000-0000-0000000000a2',
  'finance-reconciler', 'Finance reconciler',
  'Reconcile bank statements (CSV/OFX/MT940) against the internal ledger and produce a variance report.',
  'private',
  $manifest$ {
    "version": "0.5.1",
    "license": "Apache-2.0",
    "author": "carol@acme.dev",
    "category": "finance",
    "entrypoint": "reconcile.py:reconcile",
    "runtime": { "python": ">=3.11" },
    "inputs": [
      { "name": "statement", "type": "string", "required": true, "description": "Path to statement (csv/ofx/mt940)" },
      { "name": "ledger",    "type": "string", "required": true, "description": "Internal ledger export" },
      { "name": "tolerance", "type": "number", "default": 0.01,  "description": "Match tolerance in account currency" }
    ],
    "outputs": [
      { "name": "matched",   "type": "array<object>" },
      { "name": "unmatched", "type": "array<object>" },
      { "name": "variance",  "type": "number" }
    ],
    "dependencies": ["pandas>=2.2", "ofxparse>=0.21"],
    "compliance": { "data_classification": "confidential", "retention_days": 90 },
    "files": [
      { "path": "SKILL.md",     "size": 3221, "kind": "doc" },
      { "path": "reconcile.py", "size": 8420, "kind": "code" },
      { "path": "rules/",       "size": null, "kind": "dir" },
      { "path": "tests/",       "size": null, "kind": "dir" }
    ]
  } $manifest$::jsonb,
  $readme$
# Finance reconciler

Bank statement ⇆ internal ledger reconciliation, with a variance report.

## Inputs

- `statement`: CSV / OFX / MT940 — auto-detected by extension.
- `ledger`: internal export, expects columns `date, amount, ref, account`.
- `tolerance`: rounding slack (default `0.01` in account currency).

## Output

```json
{
  "matched":   [{"ref": "INV-1042", "amount": 1290.00}],
  "unmatched": [{"date": "2026-05-10", "amount": -45.00, "side": "statement"}],
  "variance":  -45.00
}
```

## Compliance

This skill is **confidential** (`data_classification: confidential`,
90-day retention). It must stay inside the `finance` namespace and is
read-only to anyone outside without an explicit cross-scope grant.

## Reviewers

- carol (owner)
- finance team admins
  $readme$,
  'clawhub install finance/finance-reconciler',
  'https://git.acme.dev/finance/reconciler',
  ARRAY['finance','reconciliation','compliance'],
  58
)
ON CONFLICT (id) DO UPDATE SET
  description = EXCLUDED.description,
  manifest = EXCLUDED.manifest,
  readme = EXCLUDED.readme,
  install_command = EXCLUDED.install_command,
  repository_url = EXCLUDED.repository_url,
  tags = EXCLUDED.tags,
  install_count = EXCLUDED.install_count;

-- ───────────────────── Skill: csv-clean ─────────────────────
INSERT INTO skills (id, namespace_id, slug, display_name, description, visibility,
                    manifest, readme, install_command, repository_url, tags, install_count)
VALUES (
  '44444444-4444-4444-4444-444444444444',
  '30000000-0000-0000-0000-0000000000a3',
  'csv-clean', 'CSV cleaner',
  'Normalise, deduplicate, and lint CSV/TSV files — including encoding sniff and header repair.',
  'global',
  $manifest$ {
    "version": "1.1.0",
    "license": "Apache-2.0",
    "author": "platform@acme.dev",
    "category": "data-quality",
    "entrypoint": "clean.py:clean",
    "runtime": { "python": ">=3.10" },
    "inputs": [
      { "name": "path", "type": "string", "required": true },
      { "name": "rules", "type": "object|null", "default": null, "description": "Override built-in rules" },
      { "name": "out",   "type": "string|null", "default": null, "description": "Output path (defaults to {path}.clean)" }
    ],
    "outputs": [
      { "name": "rows_in",  "type": "number" },
      { "name": "rows_out", "type": "number" },
      { "name": "warnings", "type": "array<string>" },
      { "name": "diff",     "type": "object" }
    ],
    "dependencies": ["polars>=1.0", "chardet>=5"],
    "files": [
      { "path": "SKILL.md",  "size": 2715, "kind": "doc" },
      { "path": "clean.py",  "size": 4983, "kind": "code" },
      { "path": "rules/",    "size": null, "kind": "dir" },
      { "path": "examples/", "size": null, "kind": "dir" }
    ]
  } $manifest$::jsonb,
  $readme$
# CSV cleaner

Heuristic-driven CSV / TSV cleanup. Sniffs encoding and delimiter,
repairs headers, deduplicates rows, and reports what changed.

## Highlights

- Auto-detects encoding (`utf-8`, `gbk`, `latin-1`) and delimiter (`,`, `;`, `\t`).
- Header repair: trims whitespace, deduplicates column names, infers types.
- Row dedupe with stable ordering.
- Emits a JSON diff alongside the cleaned file.

## Quick start

```bash
clawhub install common/csv-clean
csv-clean ./messy.csv --out ./clean.csv
```

```python
from csv_clean import clean
r = clean(path="messy.csv")
print(r["rows_in"], "→", r["rows_out"], "warnings:", r["warnings"])
```

## Rules

The built-in rule set lives in `rules/default.yml`. Override per-call:

```python
clean(path="x.csv", rules={"dedupe_on": ["email"], "trim_whitespace": true})
```
  $readme$,
  'clawhub install common/csv-clean',
  'https://git.acme.dev/platform/csv-clean',
  ARRAY['csv','tsv','data-quality','dedupe'],
  4218
)
ON CONFLICT (id) DO UPDATE SET
  description = EXCLUDED.description,
  manifest = EXCLUDED.manifest,
  readme = EXCLUDED.readme,
  install_command = EXCLUDED.install_command,
  repository_url = EXCLUDED.repository_url,
  tags = EXCLUDED.tags,
  install_count = EXCLUDED.install_count;
