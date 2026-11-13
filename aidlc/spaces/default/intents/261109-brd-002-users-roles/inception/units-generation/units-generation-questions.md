# Units Generation — Questions

This stage groups the domain-design components into units of work and records the dependency topology. It does **not** choose a build order or critical path — Delivery Planning does that next. The existing system already has an Identity service, an API Gateway, and two frontend packages, so the units live inside that topology.

Answer each question with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [architect] What should the unit boundary strategy be?

- A. By component — one unit per domain component (six) plus the gateway routes, the SPA settings surface, the support console surface, and the admin surface
- B. By deployable — one Identity-service unit, one gateway unit, one SPA unit, and one support-console unit
- C. Hybrid — group the six components into three service-side units (identity core, support & admin, gates & lifecycle) plus the three UI surfaces
- X. Other (please specify)

[Answer]: A — By component, one unit per domain component plus one per UI surface.

## Q2. [architect] What granularity do you prefer?

- A. Fine-grained: one unit per component or surface, so each can be built and reviewed independently
- B. Coarse-grained: three to four units, fewer integration points
- C. Very coarse: one unit for all BRD-002 backend work and one for the UI
- X. Other (please specify)

[Answer]: A — Fine-grained: one unit per component or surface.

## Q3. [architect] Should independent units be allowed to proceed in parallel?

- A. Yes — record the DAG with parallel sets wherever no genuine dependency exists
- B. No — record a strict chain even where parallel work is possible
- C. Parallel only within the backend and within the UI
- X. Other (please specify)

[Answer]: A — Yes: record the DAG with parallel sets wherever no genuine dependency exists.

## Q4. [architect] How do the units integrate?

The system uses synchronous internal HTTP, no message queues, and the gateway is the only entry for the browser.

- A. Units integrate over the existing synchronous HTTP APIs and the gateway route table; the audit service receives events; no new transport
- B. Introduce an internal event bus for BRD-002 events
- C. Units share the identity schema directly
- X. Other (please specify)

[Answer]: A — Existing synchronous HTTP APIs and gateway routes; the audit service receives events; no new transport.
