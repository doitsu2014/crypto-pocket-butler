//! API Gateway domain: deny-by-default routing, correlation ids, callers, and
//! gateway errors. No HTTP, database, or runtime types cross this boundary.

pub mod correlation;
pub mod error;
pub mod method;
pub mod path;
pub mod principal;
pub mod route;

pub use correlation::CorrelationId;
pub use error::GatewayError;
pub use method::Method;
pub use path::PathPattern;
pub use principal::{Principal, Role};
pub use route::{Route, RouteTable, ServiceName};
