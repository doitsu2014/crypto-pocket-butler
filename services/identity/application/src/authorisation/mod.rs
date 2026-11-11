//! The authorisation decision and the role-change use case (BR2.1–BR2.14).
//!
//! One implementation reachable two ways: in-process inside Identity, and
//! through the C4 `/internal/authorize` adapter. No shared crate.

pub mod catalogue;
pub mod decision;
pub mod ports;
pub mod role_change;
