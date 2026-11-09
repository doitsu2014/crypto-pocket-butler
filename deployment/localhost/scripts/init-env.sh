#!/usr/bin/env bash
# Creates deployment/localhost/.env and deployment/localhost/secrets/ with
# freshly generated local-only keys and tokens. Safe to run twice: an existing
# .env is left untouched.
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -f .env ]]; then
  echo ".env already exists; leaving it untouched."
  echo "Delete deployment/localhost/.env and secrets/ to rotate every local key."
  exit 0
fi

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to generate the local JWT keys (the repository already uses Node for the web app)." >&2
  exit 1
fi

node ./scripts/generate-local-env.mjs
