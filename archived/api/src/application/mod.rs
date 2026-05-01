/// Application layer
///
/// Orchestrates domain logic, infrastructure, and API concerns.
///
/// ## Sub-modules
///
/// - [`services`]      — Application services that coordinate domain operations
/// - [`usecases`]      — Use case command objects called by transport handlers
/// - [`repositories`]  — Repository trait re-exports (from the domain layer)
/// - [`dto`]           — Data Transfer Objects for API request/response
/// - [`jobs`]          — Background jobs that orchestrate business logic
/// - [`concurrency`]   — Concurrency utilities for application-level orchestration

pub mod concurrency;
pub mod dto;
pub mod jobs;
pub mod repositories;
pub mod services;
pub mod usecases;
