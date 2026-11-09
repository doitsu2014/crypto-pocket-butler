# Infrastructure Design — u1-walking-skeleton

Most infrastructure choices are fixed by the project rules (Docker Swarm, production only,
Supabase Postgres, Vault, rolling updates). These questions settle the remaining ones. Answer
each with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [Platform engineer] What Swarm topology should this Unit deploy to?

- A. Three-manager Swarm with worker nodes, as the architecture's production target describes
- B. A single-node Swarm for now, scaled later
- C. A managed Kubernetes cluster instead
- X. Other (please specify)

[Answer]: B — Single node

## Q2. [Platform engineer] What should run the pipeline?

- A. GitHub Actions, running the same fmt, clippy, lint, and test commands developers run locally
- B. GitLab CI
- C. No CI yet; local checks only
- X. Other (please specify)

[Answer]: A — GitHub Actions

## Q3. [Platform engineer] How should images be built and stored?

- A. Built in CI, scanned, pushed to a private registry with digest-pinned references
- B. Public registry
- C. Built on the Swarm node, no registry
- X. Other (please specify)

[Answer]: B — Public registry
