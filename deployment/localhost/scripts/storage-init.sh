#!/usr/bin/env bash
# Creates the Identity export bucket once Supabase Storage has run its own
# migrations. Best effort: a failed export at runtime only affects the identity
# export flow, not the rest of the stack.
set -euo pipefail

: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required}"

psql_cmd() {
  PGPASSWORD="${POSTGRES_PASSWORD}" psql \
    -h "${PGHOST:-db}" -p "${PGPORT:-5432}" -U supabase_admin -d "${PGDATABASE:-postgres}" \
    -v ON_ERROR_STOP=1 --no-psqlrc "$@"
}

echo "waiting for the storage schema"
for _ in $(seq 1 60); do
  if psql_cmd -tAc "SELECT to_regclass('storage.buckets')" | grep -q storage.buckets; then
    break
  fi
  sleep 2
done

psql_cmd -c "INSERT INTO storage.buckets (id, name, public)
             VALUES ('identity-exports', 'identity-exports', false)
             ON CONFLICT (id) DO NOTHING;"

echo "storage bucket ready"
