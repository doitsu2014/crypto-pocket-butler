//! Outbound clients for the services Portfolio consumes.

pub mod holdings;
pub mod identity;
pub mod ledger;
pub mod market_data;

pub use holdings::HttpHoldingsClient;
pub use identity::HttpIdentityPreferences;
pub use ledger::HttpLedgerClient;
pub use market_data::HttpMarketDataClient;
