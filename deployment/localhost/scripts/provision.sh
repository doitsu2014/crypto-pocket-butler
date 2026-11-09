#!/usr/bin/env bash
# Applies the schema bootstrap and every service's versioned up migrations, then
# seeds the local service-token registries.
#
# Production runs an equivalent step from the release job with the migrator DSN.
# Here it runs once, as the postgres superuser, because the migrations create
# roles and grants that a plain service role cannot.
set -euo pipefail

: "${LOCAL_DB_PASSWORD:?LOCAL_DB_PASSWORD is required}"

db_host="${PGHOST:-db}"
db_port="${PGPORT:-5432}"
db_name="${PGDATABASE:-postgres}"
admin_user="${PGADMIN_USER:-supabase_admin}"
admin_password="${PGADMIN_PASSWORD:-${POSTGRES_PASSWORD:-postgres}}"

psql_for() {
  local user="$1"
  shift
  local password="$admin_password"
  if [[ "$user" != "$admin_user" ]]; then
    password="$LOCAL_DB_PASSWORD"
  fi
  PGPASSWORD="$password" psql \
    -h "$db_host" -p "$db_port" -U "$user" -d "$db_name" \
    -v ON_ERROR_STOP=1 --no-psqlrc "$@"
}

echo "waiting for postgres at ${db_host}:${db_port}"
until PGPASSWORD="$admin_password" psql -h "$db_host" -p "$db_port" -U "$admin_user" \
    -d "$db_name" -tAc 'SELECT 1' >/dev/null 2>&1; do
  sleep 2
done

echo "applying bootstrap roles and schemas"
psql_for "$admin_user" -v db_password="$LOCAL_DB_PASSWORD" -f /init/01-roles.sql
psql_for "$admin_user" -f /init/02-pgsodium.sql

apply_migrations() {
  local schema="$1"
  local directory="$2"
  local user="${3:-$admin_user}"
  local file
  for file in $(find "$directory" -maxdepth 1 -name '*.up.sql' | sort); do
    echo "applying ${file#/services/} (schema ${schema}, role ${user})"
    psql_for "$user" --single-transaction <<SQL
SET search_path TO ${schema};
\i ${file}
SQL
  done
}

apply_migrations identity          /services/identity/migrations
apply_migrations account           /services/account/migrations
apply_migrations analytics         /services/analytics/migrations
apply_migrations audit             /services/audit/migrations
apply_migrations job_orchestration /services/job-orchestration/migrations
apply_migrations ledger            /services/ledger/migrations
apply_migrations market_data       /services/market-data/migrations
apply_migrations notification      /services/notification/migrations
apply_migrations portfolio         /services/portfolio/migrations
apply_migrations reporting         /services/reporting/migrations

# The Asset Registry migration ships no role or grants, so it runs as the role
# that owns the registry schema.
apply_migrations registry /services/asset-registry/migrations registry_svc

echo "seeding local service-token registries"
psql_for "$admin_user" -v tokens="$(cat /run/tokens/token-hashes.json)" -f /init/03-tokens.sql

echo "provisioning complete"
