#!/usr/bin/env bash
# Smoke check for the Identity service and the C4 authorisation decision.
#
# Usage:
#   scripts/smoke-identity.sh <base-url>
#
# Environment:
#   C4_SMOKE_TOKEN  the "identity-smoke" service token that identity_service_tokens
#                   carries (a throwaway token minted by the same mechanism the
#                   callers use; never a production credential).
#   SMOKE_USER_JWT  a test user's Supabase JWT from the local stack.
#   SMOKE_USER_ID   that user's internal id (the C4 body's resourceOwnerId).
#
# It exits non-zero on the first mismatch. It adds nothing to the stack and
# writes no file.

set -euo pipefail

usage() {
    echo "usage: $0 <base-url>" >&2
    echo "requires C4_SMOKE_TOKEN, SMOKE_USER_JWT, and SMOKE_USER_ID" >&2
    exit 2
}

[ "$#" -eq 1 ] || usage
BASE="${1%/}"
: "${C4_SMOKE_TOKEN:?C4_SMOKE_TOKEN must hold the identity-smoke service token}"
: "${SMOKE_USER_JWT:?SMOKE_USER_JWT must hold a test user's Supabase JWT}"
: "${SMOKE_USER_ID:?SMOKE_USER_ID must hold that user's internal id}"

STATUS=""
BODY=""

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# call <method> <url> [curl arguments...]
# Sets STATUS to the HTTP status and BODY to the response body.
call() {
    local method="$1" url="$2"
    shift 2
    local raw
    raw=$(curl -sS -X "$method" "$url" "$@" -w $'\n%{http_code}') ||
        fail "$method $url could not be reached"
    STATUS="${raw##*$'\n'}"
    BODY="${raw%$'\n'*}"
}

echo "==> GET $BASE/healthz"
call GET "$BASE/healthz"
[ "$STATUS" = "200" ] || fail "healthz answered $STATUS: $BODY"
echo "    ok"

echo "==> GET $BASE/readyz"
call GET "$BASE/readyz"
if [ "$STATUS" != "200" ]; then
    # Readiness may name the status source as degraded while the outage cache
    # still serves; the decision calls below are the real gate.
    case "$BODY" in
        *not-ready* | *degraded*) echo "    warning: readiness reports a degraded dependency: $BODY" ;;
        *) fail "readyz answered $STATUS: $BODY" ;;
    esac
else
    echo "    ok"
fi

echo "==> POST $BASE/internal/authorize (the caller's own row)"
call POST "$BASE/internal/authorize" \
    -H "content-type: application/json" \
    -H "x-service-token: $C4_SMOKE_TOKEN" \
    -H "authorization: Bearer $SMOKE_USER_JWT" \
    --data "{\"operation\":\"principal:read\",\"resourceKind\":\"account\",\"scope\":\"resource\",\"resourceOwnerId\":\"$SMOKE_USER_ID\"}"
[ "$STATUS" = "200" ] || fail "the own-row decision answered $STATUS: $BODY"
case "$BODY" in
    *'"allowed":true'*) echo "    ok" ;;
    *) fail "the own-row decision did not allow: $BODY" ;;
esac

FOREIGN_ID="00000000-0000-4000-8000-000000000001"
if [ "$FOREIGN_ID" = "$SMOKE_USER_ID" ]; then
    FOREIGN_ID="00000000-0000-4000-8000-000000000002"
fi

echo "==> POST $BASE/internal/authorize (a foreign row)"
call POST "$BASE/internal/authorize" \
    -H "content-type: application/json" \
    -H "x-service-token: $C4_SMOKE_TOKEN" \
    -H "authorization: Bearer $SMOKE_USER_JWT" \
    --data "{\"operation\":\"principal:read\",\"resourceKind\":\"account\",\"scope\":\"resource\",\"resourceOwnerId\":\"$FOREIGN_ID\"}"
[ "$STATUS" = "404" ] || fail "the foreign-row decision answered $STATUS: $BODY"
case "$BODY" in
    *not_owner*) echo "    ok" ;;
    *) fail "the foreign-row denial did not name not_owner: $BODY" ;;
esac

echo "smoke-identity: ok"
