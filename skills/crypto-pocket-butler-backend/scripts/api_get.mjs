// Minimal API GET helper.
// Usage:
//   CPB_API_BASE_URL=... CPB_ACCESS_TOKEN=... node api_get.mjs /v1/portfolios

const base = process.env.CPB_API_BASE_URL;
const token = process.env.CPB_ACCESS_TOKEN;
const path = process.argv[2];

if (!base || !token || !path) {
  console.error('Usage: CPB_API_BASE_URL=... CPB_ACCESS_TOKEN=... node api_get.mjs /path');
  process.exit(2);
}

const res = await fetch(new URL(path, base), {
  headers: { Authorization: `Bearer ${token}` },
});

const text = await res.text();
if (!res.ok) {
  console.error(res.status, res.statusText, text);
  process.exit(1);
}
console.log(text);
