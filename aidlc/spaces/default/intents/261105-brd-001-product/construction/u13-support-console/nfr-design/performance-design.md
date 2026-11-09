# Performance Design — u13-support-console

Inputs: `nfr-requirements/performance-requirements.md`, `functional-design/functional-spec.md`.

- Static Vite bundle: code-split by route (search, metadata, grants, incidents); the initial shell
  stays under 200 KB gzipped.
- TanStack Query caches search results for 30 s and metadata per account for 10 s; the cache is
  cleared on sign-out, on grant expiry or revocation, and on a 403.
- The Supabase client uses an in-memory storage adapter with `persistSession: false`.
- The grant banner schedules a refresh at `expiresAt` (and polls every 30 s as a fallback); reads
  attempted in the up-to-30-second window after expiry are refused by the server, which is accepted.
- Search results are paginated at 25; metadata reads are one request.
- Incident submit is one request; the C9 outcome is shown from the response.

| Operation | Budget | Element |
|---|---|---|
| Initial load | ≤ 2 s | Route-split bundle |
| Search | p95 ≤ 300 ms | C2 query + 30 s cache |
| Metadata | p95 ≤ 500 ms | Single C2 read |
| Grant action | p95 ≤ 400 ms | Single C2 call |
| SEV-1 notice | 15 min | Support API + Notification |

NFR references: NFR2.1–NFR2.6.
