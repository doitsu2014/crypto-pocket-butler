use super::{Balance, ExchangeConnector};
use crate::concurrency::RateLimiter;
use alloy::{
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    sol,
};
use async_trait::async_trait;
use futures::future::join_all;
use std::error::Error;
use tracing;

// Generate ERC20 contract bindings
sol! {
    #[sol(rpc)]
    contract ERC20 {
        function balanceOf(address owner) public view returns (uint256);
        function decimals() public view returns (uint8);
        function symbol() public view returns (string);
    }
}

/// Supported EVM chains
#[derive(Debug, Clone)]
pub enum EvmChain {
    Ethereum,
    Arbitrum,
    Optimism,
    Base,
    BinanceSmartChain,
}

impl EvmChain {
    /// Get the default public RPC URL for this chain
    pub fn rpc_url(&self) -> &'static str {
        match self {
            EvmChain::Ethereum => "https://eth.llamarpc.com",
            EvmChain::Arbitrum => "https://arbitrum.llamarpc.com",
            EvmChain::Optimism => "https://optimism.llamarpc.com",
            EvmChain::Base => "https://base.llamarpc.com",
            EvmChain::BinanceSmartChain => "https://bsc-dataseed.bnbchain.org",
        }
    }

    /// Get the chain name
    pub fn name(&self) -> &'static str {
        match self {
            EvmChain::Ethereum => "ethereum",
            EvmChain::Arbitrum => "arbitrum",
            EvmChain::Optimism => "optimism",
            EvmChain::Base => "base",
            EvmChain::BinanceSmartChain => "bsc",
        }
    }

    /// Get the native asset symbol for this chain
    pub fn native_symbol(&self) -> &'static str {
        match self {
            EvmChain::Ethereum => "ETH",
            EvmChain::Arbitrum => "ETH",
            EvmChain::Optimism => "ETH",
            EvmChain::Base => "ETH",
            EvmChain::BinanceSmartChain => "BNB",
        }
    }
}

/// Common ERC-20 token addresses by chain
/// 
/// This function returns a predefined list of well-known tokens to check for balances.
/// 
/// # Current Limitations
/// 
/// This approach only checks a limited set of common tokens. It does **not** discover
/// all tokens in a wallet. Full token discovery would require:
/// 
/// 1. **Event Log Scanning**: Query all `Transfer` events to/from the wallet
/// 2. **Indexing Service**: Use The Graph, Alchemy, or Moralis APIs
/// 3. **Token Lists**: Reference community-maintained token lists
/// 
/// # Why Not Full Discovery?
/// 
/// - No complete on-chain registry of all ERC-20 tokens
/// - Event log queries are slow and expensive on public RPC nodes
/// - Public RPC endpoints have strict rate limits
/// - Would significantly increase sync time
/// 
/// # Extending Token Support
/// 
/// To add more tokens, simply add them to the appropriate chain's vector:
/// 
/// ```rust
/// EvmChain::Ethereum => vec![
///     ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
///     ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
///     // Add more tokens here
/// ],
/// ```
pub fn get_common_tokens(chain: &EvmChain) -> Vec<(&'static str, &'static str)> {
    match chain {
        EvmChain::Ethereum => vec![
            ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
            ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            ("DAI", "0x6B175474E89094C44Da98b954EedeAC495271d0F"),
            ("WETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        ],
        EvmChain::Arbitrum => vec![
            ("USDT", "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"),
            ("USDC", "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"),
            ("DAI", "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"),
            ("WETH", "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"),
        ],
        EvmChain::Optimism => vec![
            ("USDT", "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58"),
            ("USDC", "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85"),
            ("DAI", "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"),
            ("WETH", "0x4200000000000000000000000000000000000006"),
        ],
        EvmChain::Base => vec![
            ("USDC", "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
            ("DAI", "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb"),
            ("WETH", "0x4200000000000000000000000000000000000006"),
        ],
        EvmChain::BinanceSmartChain => vec![
            ("USDT", "0x55d398326f99059fF775485246999027B3197955"),
            ("USDC", "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d"),
            ("DAI", "0x1AF3F329e8BE154074D8769D1FFa4eE058B1DBc3"),
            ("WBNB", "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"),
        ],
    }
}

/// EVM wallet connector for fetching native and ERC-20 token balances
pub struct EvmConnector {
    wallet_address: Address,
    chains: Vec<EvmChain>,
}

impl EvmConnector {
    /// Create a new EVM connector for a wallet address
    /// 
    /// # Arguments
    /// * `wallet_address` - The wallet address to fetch balances for (hex string)
    /// * `chains` - List of EVM chains to check (defaults to Ethereum mainnet if empty)
    pub fn new(wallet_address: String, chains: Vec<EvmChain>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Parse the wallet address
        let addr = wallet_address.parse::<Address>()
            .map_err(|e| format!("Invalid wallet address: {}", e))?;
        
        let chains = if chains.is_empty() {
            vec![EvmChain::Ethereum]
        } else {
            chains
        };

        Ok(Self {
            wallet_address: addr,
            chains,
        })
    }
}

#[async_trait]
impl ExchangeConnector for EvmConnector {
    async fn fetch_spot_balances(&self) -> Result<Vec<Balance>, Box<dyn Error + Send + Sync>> {
        tracing::info!("Fetching balances for wallet {} across {} chains (parallel)", 
            self.wallet_address, self.chains.len());
        
        // Create rate limiter for RPC calls
        let rate_limiter = RateLimiter::evm_rpc();
        
        // Fetch balances from all chains in parallel
        let fetch_tasks: Vec<_> = self.chains.iter().map(|chain| {
            let chain = chain.clone();
            let wallet_address = format!("{:?}", self.wallet_address); // Convert Address to hex string
            let rate_limiter = rate_limiter.clone();
            
            async move {
                // Acquire rate limit permit
                let _permit = rate_limiter.acquire().await.ok()?;
                
                tracing::info!("Checking {} chain", chain.name());
                let mut chain_balances = Vec::new();
                
                // Fetch native balance
                match fetch_native_balance_for_chain(&wallet_address, &chain).await {
                    Ok(Some(balance)) => chain_balances.push(balance),
                    Ok(None) => {},
                    Err(e) => {
                        tracing::error!("Failed to fetch native balance on {}: {}", chain.name(), e);
                    }
                }
                
                // Fetch token balances
                match fetch_token_balances_for_chain(&wallet_address, &chain).await {
                    Ok(balances) => chain_balances.extend(balances),
                    Err(e) => {
                        tracing::error!("Failed to fetch token balances on {}: {}", chain.name(), e);
                    }
                }
                
                Some(chain_balances)
            }
        }).collect();
        
        // Wait for all chain queries to complete
        let results = join_all(fetch_tasks).await;
        
        // Flatten results
        let mut all_balances = Vec::new();
        for result in results {
            if let Some(balances) = result {
                all_balances.extend(balances);
            }
        }
        
        tracing::info!("Fetched {} total balances for wallet", all_balances.len());
        Ok(all_balances)
    }
}

// Helper function to fetch native balance for a chain
async fn fetch_native_balance_for_chain(
    wallet_address: &str,
    chain: &EvmChain,
) -> Result<Option<Balance>, Box<dyn Error + Send + Sync>> {
    let provider = ProviderBuilder::new().connect_http(chain.rpc_url().parse()?);
    let address: Address = wallet_address.parse()?;
    
    let balance = provider.get_balance(address).await?;
    
    if balance == U256::ZERO {
        return Ok(None);
    }
    
    let balance_str = balance.to_string();
    
    Ok(Some(Balance {
        asset: format!("{}-{}", chain.native_symbol(), chain.name()),
        quantity: balance_str.clone(),
        available: balance_str.clone(),
        frozen: "0".to_string(),
        // Native tokens typically have 18 decimals
        decimals: Some(18),
    }))
}

// Helper function to fetch token balances for a chain
async fn fetch_token_balances_for_chain(
    wallet_address: &str,
    chain: &EvmChain,
) -> Result<Vec<Balance>, Box<dyn Error + Send + Sync>> {
    let provider = ProviderBuilder::new().connect_http(chain.rpc_url().parse()?);
    let wallet_address: Address = wallet_address.parse()?;
    
    let token_addresses = get_common_tokens(chain);
    let mut balances = Vec::new();
    
    // Note: provider.clone() is cheap (Arc-based in alloy)
    for (symbol, token_address) in token_addresses {
        let contract_address: Address = match token_address.parse() {
            Ok(addr) => addr,
            Err(e) => {
                tracing::warn!("Failed to parse token address {}: {}", token_address, e);
                continue;
            }
        };
        
        let contract = ERC20::new(contract_address, provider.clone());
        
        match contract.balanceOf(wallet_address).call().await {
            Ok(balance) => {
                if balance > U256::ZERO {
                    let balance_str = balance.to_string();
                    
                    // Fetch decimals from the contract
                    let decimals = match contract.decimals().call().await {
                        Ok(d) => Some(d),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to fetch decimals for {} on {}: {}, defaulting to None",
                                symbol,
                                chain.name(),
                                e
                            );
                            None
                        }
                    };
                    
                    balances.push(Balance {
                        asset: format!("{}-{}", symbol, chain.name()),
                        quantity: balance_str.clone(),
                        available: balance_str.clone(),
                        frozen: "0".to_string(),
                        decimals,
                    });
                    
                    tracing::debug!(
                        "Found {} {} (decimals: {:?}) on {} ({})",
                        balance_str,
                        symbol,
                        decimals,
                        chain.name(),
                        token_address
                    );
                }
            }
            Err(e) => {
                tracing::debug!(
                    "Failed to check balance for {} on {}: {}",
                    symbol,
                    chain.name(),
                    e
                );
            }
        }
    }
    
    Ok(balances)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_properties() {
        let eth = EvmChain::Ethereum;
        assert_eq!(eth.name(), "ethereum");
        assert_eq!(eth.native_symbol(), "ETH");
        assert!(!eth.rpc_url().is_empty());
        
        let bsc = EvmChain::BinanceSmartChain;
        assert_eq!(bsc.name(), "bsc");
        assert_eq!(bsc.native_symbol(), "BNB");
        assert!(!bsc.rpc_url().is_empty());
    }

    #[test]
    fn test_connector_creation() {
        // Valid address
        let result = EvmConnector::new(
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
            vec![EvmChain::Ethereum],
        );
        assert!(result.is_ok());

        // Invalid address
        let result = EvmConnector::new(
            "invalid-address".to_string(),
            vec![EvmChain::Ethereum],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_common_tokens() {
        let eth_tokens = get_common_tokens(&EvmChain::Ethereum);
        assert!(!eth_tokens.is_empty());
        assert!(eth_tokens.iter().any(|(symbol, _)| *symbol == "USDC"));
        
        let arb_tokens = get_common_tokens(&EvmChain::Arbitrum);
        assert!(!arb_tokens.is_empty());
        
        let bsc_tokens = get_common_tokens(&EvmChain::BinanceSmartChain);
        assert!(!bsc_tokens.is_empty());
        assert!(bsc_tokens.iter().any(|(symbol, _)| *symbol == "USDT"));
        assert!(bsc_tokens.iter().any(|(symbol, _)| *symbol == "WBNB"));
    }
}
