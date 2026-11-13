//! Support-access infrastructure: the audit outbox and its backlog reading.
//!
//! Serves the support-grant rules BR3.1–BR3.13 and contract C5 (Support Access):
//! the grant mutation and its audit intent commit together (BR3.13), the intent
//! is drained to C10 at least once with an idempotency key, and the backlog
//! gauges are read when Prometheus scrapes `/metrics` (NFR1.8).

pub mod backlog;
pub mod drainer;
pub mod observation;
pub mod outbox;
pub mod sources;
