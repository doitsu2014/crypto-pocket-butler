# Security Design — u11-job-orchestration

Sources: `nfr-requirements/security-requirements.md`, `functional-design/rules.md` BR6–BR7.

- **Identity.** `JwksValidator` validates the forwarded Supabase JWT in this service (signature,
  issuer, audience, expiry; 30 s leeway). The role is read from `app_metadata.role`; the top-level
  `role` and `user_metadata` are ignored. `admin` and `support` pass; anything else is 403
  (NFR9.1–NFR9.2).
- **Operator paths.** `/v1/jobs*` requires the JWT; `/v1/internal/smoke` and run targets use the
  service-token registry. The inbound registry seeds `smoke-reader` only; there is no user-data
  read in this service (NFR9.7).
- **Outbound token.** `OUTBOUND_TOKEN_FILE` is read once at startup; the value is held in memory
  and never logged, returned, or persisted. A missing or empty file refuses startup (NFR9.3).
- **Target allow-list.** The catalogue is a compile-time constant table (job → service URL env);
  an unknown job name is a startup failure, and no request field can influence the URL
  (NFR9.4).
- **Non-secret details.** The detail builder formats a fixed set of messages (`lease held`,
  `target unavailable`, …) and accepts only the provider's `runId` identifier from a 202 answer;
  it never interpolates a response body, header, or credential (NFR9.5).
- **Audit.** Manual and backfill actions append a C10 event with actor (`app_metadata` user id),
  action (`job.run.manual`, `job.backfill`), target (`job:{job}:{window}`), instant, and outcome.
  An abandoned run is a system event with the allow-listed action `system.job.run` and the job as
  target. A C10 refusal is logged and never blocks the action (NFR9.6).
- **Transport.** The `internal` Swarm overlay is created with `--opt encrypted`, so internal HTTP
  (including the service token) travels inside an encrypted overlay; the public edge terminates
  TLS (NFR9.8).
