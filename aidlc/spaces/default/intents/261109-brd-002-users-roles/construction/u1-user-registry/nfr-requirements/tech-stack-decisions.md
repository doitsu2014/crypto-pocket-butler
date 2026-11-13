# Tech Stack Decisions — u1-user-registry

**Upstream:** `performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `functional-spec.md`, `technology-stack.md`.

| Decision | Choice | Rationale | Alternatives rejected |
|---|---|---|---|
| Language and web framework | Rust 2021 with Axum and tower | Project mandate; four-crate workspace inside Identity | Other languages — forbidden |
| Async runtime | tokio | Project mandate | — |
| Persistence | sqlx against the Identity schema (`users`, `shredding_keys`) | Project mandate; one schema and role per service | An ORM — forbidden |
| Subject encryption | pgsodium deterministic ciphertext plus SHA-256 hash | Existing identity schema pattern; supports lookup and crypto-shredding | Plaintext storage — forbidden; non-deterministic ciphertext — no equality lookup |
| Admin search | Indexed exact match on email or internal id | Meets NFR2.3 without a search service | Full-text search — adds an index and a tokenizer for no requirement |
| Messaging | None | Project mandate: no message queues | Event bus — forbidden |
| Migrations | Versioned sqlx up/down pairs | Project mandate | — |
