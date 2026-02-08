// Usage:
//   CPB_KEYCLOAK_TOKEN_URL=... CPB_KEYCLOAK_CLIENT_ID=... CPB_KEYCLOAK_CLIENT_SECRET=... node get_token.mjs

const url = process.env.CPB_KEYCLOAK_TOKEN_URL;
const client_id = process.env.CPB_KEYCLOAK_CLIENT_ID;
const client_secret = process.env.CPB_KEYCLOAK_CLIENT_SECRET;

if (!url || !client_id || !client_secret) {
  console.error('Missing CPB_KEYCLOAK_TOKEN_URL / CPB_KEYCLOAK_CLIENT_ID / CPB_KEYCLOAK_CLIENT_SECRET');
  process.exit(2);
}

const body = new URLSearchParams({
  grant_type: 'client_credentials',
  client_id,
  client_secret,
});

const res = await fetch(url, {
  method: 'POST',
  headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
  body,
});

if (!res.ok) {
  console.error('Token request failed:', res.status, await res.text());
  process.exit(1);
}

const json = await res.json();
console.log(json.access_token);
