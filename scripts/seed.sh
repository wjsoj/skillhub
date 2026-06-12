#!/usr/bin/env bash
# Seed demo data + reindex embeddings.
# Usage: scripts/seed.sh  (assumes pg @ 127.0.0.1:15432, api @ 127.0.0.1:8088)
set -euo pipefail

PGURI="${PGURI:-postgres://skillhub:skillhub@127.0.0.1:15432/skillhub}"
API="${SKILLHUB_API:-http://127.0.0.1:8088}"

echo "→ applying seed.sql"
SQL_FILE="$(dirname "$0")/seed.sql"
if command -v psql >/dev/null 2>&1; then
  psql "$PGURI" -v ON_ERROR_STOP=1 -f "$SQL_FILE"
else
  # Fall back to running psql inside the Postgres container.
  docker exec -i skillhub-pg psql -U skillhub -d skillhub -v ON_ERROR_STOP=1 < "$SQL_FILE"
fi

echo "→ reindex embeddings"
curl -sS -X POST "$API/api/v1/admin/reindex-embeddings" \
  -H "X-Mock-User-Id: 00000000-0000-0000-0000-000000000009" \
  | head -c 200
echo
echo "✓ seed complete"
