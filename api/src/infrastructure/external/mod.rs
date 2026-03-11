/// Infrastructure external layer
///
/// Connectors to external services (exchanges, price feeds, blockchains).
///
/// These are the concrete adapters that implement `ExchangeConnector` and
/// other traits defined in `crate::connectors`.

pub use crate::connectors::{
    coinpaprika::CoinPaprikaConnector,
    evm::EvmConnector,
    okx::OkxConnector,
    Balance,
    ExchangeConnector,
};
