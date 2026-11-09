// Generates the local-only material the stack needs: an RSA JWT signing key,
// the Supabase anon/service-role keys, the internal service tokens, and the
// file-backed secrets the containers mount.
//
// Nothing here is a production secret. Every value is generated fresh on one
// developer machine, written under deployment/localhost/.env and
// deployment/localhost/secrets/ (both gitignored), and never reused anywhere
// else. Run scripts/init-env.sh rather than invoking this directly.

import { createHash, createSign, generateKeyPairSync, randomBytes } from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const secrets = join(root, "secrets");
const outbound = join(secrets, "outbound");

mkdirSync(outbound, { recursive: true, mode: 0o700 });
mkdirSync(join(root, "supabase"), { recursive: true });

const base64url = (value) =>
  Buffer.from(value)
    .toString("base64")
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");

const issuer = "http://127.0.0.1:54321";
const audience = "authenticated";
const now = Math.floor(Date.now() / 1000);
const expiry = now + 10 * 365 * 24 * 60 * 60;

// One RSA signing key for the whole local stack. Supabase Auth signs every
// access token with it and publishes the public half at
// http://127.0.0.1:54321/auth/v1/.well-known/jwks.json, which is where the
// services fetch it from. The services only accept RS256, so a symmetric
// Supabase secret alone would not work.
const { privateKey, publicKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
const kid = `local-${randomBytes(6).toString("hex")}`;
const privateJwk = {
  ...privateKey.export({ format: "jwk" }),
  kid,
  alg: "RS256",
  use: "sig",
  key_ops: ["sign"],
};
const publicJwk = {
  ...publicKey.export({ format: "jwk" }),
  kid,
  alg: "RS256",
  use: "sig",
  key_ops: ["verify"],
};
const privateJwks = JSON.stringify([privateJwk]);
const publicJwks = JSON.stringify({ keys: [publicJwk] });

function signJwt(role) {
  const header = { alg: "RS256", typ: "JWT", kid };
  const payload = { iss: issuer, aud: audience, role, iat: now, exp: expiry };
  const data = `${base64url(JSON.stringify(header))}.${base64url(JSON.stringify(payload))}`;
  const signature = createSign("RSA-SHA256").update(data).sign(privateKey);
  return `${data}.${base64url(signature)}`;
}

const anonKey = signJwt("anon");
const serviceRoleKey = signJwt("service_role");

// One token per internal caller. The consuming service hashes the token and
// stores the digest; the caller presents the raw token.
const callers = [
  "account",
  "analytics",
  "audit",
  "gateway",
  "identity",
  "job-orchestration",
  "ledger",
  "market-data",
  "notification",
  "portfolio",
  "reporting",
  "smoke-reader",
];
const tokens = Object.fromEntries(
  callers.map((caller) => [caller, randomBytes(32).toString("hex")]),
);
const hashes = Object.fromEntries(
  Object.entries(tokens).map(([caller, token]) => [
    caller,
    createHash("sha256").update(token).digest("hex"),
  ]),
);

const gateTokenKey = randomBytes(32).toString("hex");
const jwtSecret = randomBytes(32).toString("hex");
const emailKey = randomBytes(32).toString("hex");
const metaCryptoKey = randomBytes(16).toString("hex");
const adminPassword = `Cpb-${randomBytes(9).toString("base64url")}!7`;

writeFileSync(join(secrets, "jwks.json"), `${publicJwks}\n`, { mode: 0o644 });
writeFileSync(join(secrets, "jwt-private.jwks.json"), `${privateJwks}\n`, { mode: 0o644 });
writeFileSync(join(secrets, "service-tokens.json"), `${JSON.stringify(tokens, null, 2)}\n`, {
  mode: 0o644,
});
writeFileSync(join(secrets, "token-hashes.json"), `${JSON.stringify(hashes, null, 2)}\n`, {
  mode: 0o644,
});

// Raw outbound tokens for the services whose client adapter reads a single
// token from a file rather than a caller map.
const rawOutbound = [
  "analytics",
  "job-orchestration",
  "ledger",
  "notification",
  "portfolio",
  "reporting",
];
for (const caller of rawOutbound) {
  writeFileSync(join(outbound, `${caller}.token`), `${tokens[caller]}\n`, { mode: 0o644 });
}

// Kong declarative configuration. The API keys are interpolated here so the
// container neither needs shell templating nor reads an unset variable.
const kong = `_format_version: "1.1"
_transform: true

consumers:
  - username: anon
    keyauth_credentials:
      - key: ${anonKey}
  - username: service_role
    keyauth_credentials:
      - key: ${serviceRoleKey}

acls:
  - consumer: anon
    group: anon
  - consumer: service_role
    group: admin

services:
  - name: auth-v1
    url: http://auth:9999/
    routes:
      - name: auth-v1-all
        strip_path: true
        paths:
          - /auth/v1/
    plugins:
      - name: cors

  - name: rest-v1
    url: http://rest:3000/
    routes:
      - name: rest-v1-all
        strip_path: true
        paths:
          - /rest/v1/
    plugins:
      - name: cors
      - name: key-auth
        config:
          hide_credentials: true
      - name: acl
        config:
          hide_groups_header: true
          allow:
            - anon
            - admin

  - name: storage-v1
    url: http://storage:5000/
    routes:
      - name: storage-v1-all
        strip_path: true
        paths:
          - /storage/v1/
    plugins:
      - name: cors
`;
writeFileSync(join(root, "supabase", "kong.generated.yml"), kong, { mode: 0o644 });

const env = `# Generated by scripts/init-env.sh on ${new Date().toISOString()}.
# Local development only. Never commit this file and never reuse these values.
POSTGRES_PASSWORD=postgres
LOCAL_DB_PASSWORD=localdev

SUPABASE_URL=${issuer}
SUPABASE_JWT_ISSUER=${issuer}
SUPABASE_JWT_AUDIENCE=${audience}
SUPABASE_JWT_SECRET=${jwtSecret}
SUPABASE_ANON_KEY=${anonKey}
SUPABASE_SERVICE_ROLE_KEY=${serviceRoleKey}
GOTRUE_JWT_KEYS='${privateJwks}'
SUPABASE_PUBLIC_JWKS='${publicJwks}'

IDENTITY_GATE_TOKEN_KEY=${gateTokenKey}
NOTIFICATION_EMAIL_KEY=${emailKey}
STUDIO_PG_META_CRYPTO_KEY=${metaCryptoKey}

ADMIN_EMAIL=admin@cpb.local
ADMIN_PASSWORD=${adminPassword}

# External endpoints the account service requires at startup. Replace them with
# your own RPC providers when you need real balances.
EVM_RPC_URL=https://ethereum-rpc.publicnode.com
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
`;
writeFileSync(join(root, ".env"), env, { mode: 0o600 });

process.stdout.write(
  [
    "Generated deployment/localhost/.env and deployment/localhost/secrets/.",
    "Admin login: admin@cpb.local (the generated password is ADMIN_PASSWORD in .env).",
    "",
  ].join("\n"),
);
