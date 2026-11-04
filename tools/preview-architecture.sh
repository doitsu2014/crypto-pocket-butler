#!/usr/bin/env bash
# Preview the C4 model in docs/architecture/workspace.dsl via Structurizr Local.
# Usage: tools/preview-architecture.sh [port]   (default 8080)
set -euo pipefail

port="${1:-8080}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

docker rm -f structurizr >/dev/null 2>&1 || true
docker run -d --rm --name structurizr \
  -p "${port}:8080" \
  -v "${root}/docs/architecture:/usr/local/structurizr" \
  structurizr/structurizr local >/dev/null

echo "Structurizr Local: http://localhost:${port}"
echo "Stop with: docker stop structurizr (rebuild by re-running this script)"