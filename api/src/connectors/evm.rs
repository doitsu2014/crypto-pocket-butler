use super::{Balance, ExchangeConnector};
use crate::concurrency::RateLimiter;
use crate::helpers::balance_normalization::normalize_token_balance;
use alloy::{
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
    sol,
};
use async_trait::async_trait;
use futures::future::join_all;
use std::collections::HashMap;
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

/// A supported EVM chain, loaded from the `evm_chains` database table.
///
/// Using a struct instead of an enum makes the set of supported chains fully
/// data-driven: adding a new chain only requires inserting a row in `evm_chains`
/// — no code change needed.
#[derive(Debug, Clone)]
pub struct EvmChain {
    chain_id: String,
    default_rpc_url: String,
    native_symbol: String,
}

impl EvmChain {
    /// Construct an [`EvmChain`] from raw parts (e.g. loaded from the database).
    pub fn new(
        chain_id: impl Into<String>,
        rpc_url: impl Into<String>,
        native_symbol: impl Into<String>,
    ) -> Self {
        Self {
            chain_id: chain_id.into(),
            default_rpc_url: rpc_url.into(),
            native_symbol: native_symbol.into(),
        }
    }

    /// Chain identifier, matching the `chain_id` column in the `evm_chains` table
    /// (e.g. `"ethereum"`, `"hyper_liquid"`).
    pub fn name(&self) -> &str {
        &self.chain_id
    }

    /// Default RPC URL for this chain.  May be overridden per-connector via
    /// `EvmConnector::new_with_tokens`'s `custom_rpc_urls` argument.
    pub fn rpc_url(&self) -> &str {
        &self.default_rpc_url
    }

    /// Symbol of the chain's native token (e.g. `"ETH"`, `"BNB"`, `"HYPE"`).
    pub fn native_symbol(&self) -> &str {
        &self.native_symbol
    }

    /// Hardcoded fallback chain list used when the database is unreachable.
    ///
    /// Callers should prefer loading chains from the `evm_chains` DB table so that
    /// any chain added at runtime is automatically supported.
    pub fn defaults() -> Vec<EvmChain> {
        vec![
            EvmChain::new("ethereum", "https://eth.llamarpc.com", "ETH"),
            EvmChain::new("arbitrum", "https://arbitrum.llamarpc.com", "ETH"),
            EvmChain::new("optimism", "https://optimism.llamarpc.com", "ETH"),
            EvmChain::new("base", "https://base.llamarpc.com", "ETH"),
            EvmChain::new("bsc", "https://bsc-dataseed.bnbchain.org", "BNB"),
        ]
    }
}

/// Common ERC-20 token addresses by chain
/// 
/// This function returns a predefined list of well-known tokens to check for balances
/// across all supported EVM chains. Covers stablecoins, wrapped assets, DeFi governance
/// tokens, liquid staking tokens, and other high-market-cap ERC-20 tokens.
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
/// ```text
/// "ethereum" => vec![
///     ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
///     ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
///     // Add more tokens here
/// ],
/// ```
pub fn get_common_tokens(chain: &EvmChain) -> Vec<(&'static str, &'static str)> {
    match chain.name() {
        "ethereum" => vec![
            // Stablecoins
            ("USDT",  "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
            ("USDC",  "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            ("DAI",   "0x6B175474E89094C44Da98b954EedeAC495271d0F"),
            ("FRAX",  "0x853d955aCEf822Db058eb8505911ED77F175b99e"),
            ("LUSD",  "0x5f98805A4E8be255a32880FDeC7F6728C6568bA0"),
            // Wrapped assets
            ("WETH",  "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
            ("WBTC",  "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"),
            // Liquid staking tokens
            ("STETH", "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84"),
            ("WSTETH","0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0"),
            ("RETH",  "0xae78736Cd615f374D3085123A210448E74Fc6393"),
            ("CBETH", "0xBe9895146f7AF43049ca1c1AE358B0541Ea49704"),
            // DeFi blue chips
            ("LINK",  "0x514910771AF9Ca656af840dff83E8264EcF986CA"),
            ("UNI",   "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"),
            ("AAVE",  "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9"),
            ("MKR",   "0x9f8F72aA9304c8B593d555F12eF6589cC3A579A2"),
            ("COMP",  "0xc00e94Cb662C3520282E6f5717214004A7f26888"),
            ("CRV",   "0xD533a949740bb3306d119CC777fa900bA034cd52"),
            ("LDO",   "0x5A98FcBEA516Cf06857215779Fd812CA3beF1B32"),
            ("SNX",   "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F"),
            ("BAL",   "0xba100000625a3754423978a60c9317c58a424e3D"),
            ("1INCH", "0x111111111117dC0aa78b770fA6A738034120C302"),
            ("ENS",   "0xC18360217D8F7Ab5e7c516566761Ea12Ce7F9D72"),
        ],
        "arbitrum" => vec![
            // Stablecoins
            ("USDT",  "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"),
            ("USDC",  "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"),
            ("DAI",   "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"),
            ("FRAX",  "0x17FC002b466eEc40DaE837Fc4bE5c67993ddBd6F"),
            ("LUSD",  "0x93b346b6BC2548dA6A1E7d98E9a421B42541425b"),
            // Wrapped assets
            ("WETH",  "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"),
            ("WBTC",  "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f"),
            // Liquid staking
            ("WSTETH","0x5979D7b546E38E414F7E9822514be443A4800529"),
            ("RETH",  "0xEC70Dcb4A1EFa46b8F2D97C310C9c4790ba5ffA8"),
            // DeFi tokens on Arbitrum
            ("LINK",  "0xf97f4df75117a78c1A5a0DBb814Af92458539FB4"),
            ("UNI",   "0xFa7F8980b0f1E64A2062791cc3b0871572f1F7f0"),
            ("AAVE",  "0xba5DdD1f9d7F570dc94a51479a000E3BCE967196"),
            ("CRV",   "0x11cDb42B0EB46D95f990BeDD4695A6e3fA034978"),
            ("GMX",   "0xfc5A1A6EB076a2C7aD06eD22C90d7E710E35ad0a"),
            ("ARB",   "0x912CE59144191C1204E64559FE8253a0e49E6548"),
        ],
        "optimism" => vec![
            // Stablecoins
            ("USDT",  "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58"),
            ("USDC",  "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85"),
            ("DAI",   "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"),
            ("FRAX",  "0x2E3D870790dC77A83DD1d18184Acc7439A53f475"),
            ("LUSD",  "0xc40F949F8a4e094D1b49a23ea9241D289B7b2819"),
            // Wrapped assets
            ("WETH",  "0x4200000000000000000000000000000000000006"),
            ("WBTC",  "0x68f180fcCe6836688e9084f035309E29Bf0A2095"),
            // Liquid staking
            ("WSTETH","0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb"),
            ("RETH",  "0x9Bcef72be871e61ED4fBbc7630889beE758eb81D"),
            // DeFi tokens on Optimism
            ("LINK",  "0x350a791Bfc2C21F9Ed5d10980Dad2e2638ffa7f6"),
            ("UNI",   "0x6fd9d7AD17242c41f7131d257212c54A0e816691"),
            ("AAVE",  "0x76FB31fb4af56892A25e32cFC43De717950c9278"),
            ("CRV",   "0x0994206dfE8De6Ec6920FF4D779B0d950605Fb53"),
            ("OP",    "0x4200000000000000000000000000000000000042"),
            ("SNX",   "0x8700dAec35aF8Ff88c16BdF0418774CB3D7599B4"),
        ],
        "base" => vec![
            // Stablecoins
            ("USDC",  "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
            ("DAI",   "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb"),
            ("USDT",  "0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2"),
            // Wrapped assets
            ("WETH",  "0x4200000000000000000000000000000000000006"),
            ("WBTC",  "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf"),
            // Liquid staking
            ("WSTETH","0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452"),
            ("CBETH", "0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22"),
            ("RETH",  "0xB6fe221Fe9EeF5aBa221c348bA20A1Bf5e73624c"),
            // DeFi tokens on Base
            ("AERO",  "0x940181a94A35A4569E4529A3CDfB74e38FD98631"),
        ],
        "bsc" => vec![
            // Stablecoins
            ("USDT",  "0x55d398326f99059fF775485246999027B3197955"),
            ("USDC",  "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d"),
            ("DAI",   "0x1AF3F329e8BE154074D8769D1FFa4eE058B1DBc3"),
            ("BUSD",  "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56"),
            ("FDUSD", "0xc5f0f7b66764F6ec8C8Dff7BA683102295E16409"),
            // Wrapped assets
            ("WBNB",  "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"),
            ("BTCB",  "0x7130d2A12B9BCbFAe4f2634d864A1Ee1Ce3Ead9c"),
            ("ETH",   "0x2170Ed0880ac9A755fd29B2688956BD959F933F8"),
            // DeFi tokens on BSC
            ("CAKE",  "0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82"),
            ("XVS",   "0xcF6BB5389c92Bdda8a3747Ddb454cB7a64626C63"),
            ("ALPACA","0x8F0528cE5eF7B51152A59745bEfDD91D97091d2F"),
            ("LINK",  "0xF8A0BF9cF54Bb92F17374d9e9A321E6a111a51bD"),
        ],
        // For chains not yet in the built-in list, return an empty set.
        // Token discovery for those chains relies entirely on the `evm_tokens` DB table.
        _ => vec![],
    }
}

/// EVM wallet connector for fetching native and ERC-20 token balances
pub struct EvmConnector {
    wallet_address: Address,
    chains: Vec<EvmChain>,
    /// Per-chain token overrides sourced from the database.
    /// When `Some`, these are used instead of `get_common_tokens()`.
    /// Key: chain name (e.g. "ethereum"), Value: [(symbol, contract_address)]
    custom_tokens: Option<HashMap<String, Vec<(String, String)>>>,
    /// Per-chain RPC URL overrides sourced from the `evm_chains` DB table.
    /// Key: chain name (e.g. "ethereum"), Value: RPC URL string.
    /// Falls back to `EvmChain::rpc_url()` for chains not present in this map.
    rpc_urls: HashMap<String, String>,
}

impl EvmConnector {
    /// Create a new EVM connector for a wallet address using the built-in token list.
    ///
    /// # Arguments
    /// * `wallet_address` - The wallet address to fetch balances for (hex string)
    /// * `chains` - List of EVM chains to check (defaults to Ethereum mainnet if empty)
    pub fn new(wallet_address: String, chains: Vec<EvmChain>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::new_with_tokens(wallet_address, chains, None, None)
    }

    /// Create a new EVM connector with custom per-chain token lists and RPC URLs from the database.
    ///
    /// When `custom_tokens` is `Some`, it fully replaces `get_common_tokens()` for every chain
    /// that has an entry. Chains without an entry fall back to the built-in list.
    ///
    /// When `custom_rpc_urls` is `Some`, those URLs are used instead of the hardcoded defaults
    /// in `EvmChain::rpc_url()`. Chains not present in the map use their hardcoded default.
    ///
    /// # Arguments
    /// * `wallet_address` - The wallet address to fetch balances for (hex string)
    /// * `chains` - List of EVM chains to check (defaults to Ethereum mainnet if empty)
    /// * `custom_tokens` - Optional per-chain token map from the `evm_tokens` DB table
    /// * `custom_rpc_urls` - Optional per-chain RPC URL map from the `evm_chains` DB table
    pub fn new_with_tokens(
        wallet_address: String,
        chains: Vec<EvmChain>,
        custom_tokens: Option<HashMap<String, Vec<(String, String)>>>,
        custom_rpc_urls: Option<HashMap<String, String>>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let addr = wallet_address.parse::<Address>()
            .map_err(|e| format!("Invalid wallet address: {}", e))?;

        let chains = if chains.is_empty() {
            EvmChain::defaults()
        } else {
            chains
        };

        Ok(Self {
            wallet_address: addr,
            chains,
            custom_tokens,
            rpc_urls: custom_rpc_urls.unwrap_or_default(),
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
            // Resolve tokens for this chain: DB-sourced list takes priority over the built-in list
            let chain_tokens: Vec<(String, String)> = if let Some(ref custom) = self.custom_tokens {
                if let Some(tokens) = custom.get(chain.name()) {
                    tokens.clone()
                } else {
                    get_common_tokens(&chain)
                        .into_iter()
                        .map(|(s, a)| (s.to_string(), a.to_string()))
                        .collect()
                }
            } else {
                get_common_tokens(&chain)
                    .into_iter()
                    .map(|(s, a)| (s.to_string(), a.to_string()))
                    .collect()
            };
            // Resolve RPC URL: DB-sourced value takes priority over the hardcoded default
            let rpc_url = self.rpc_urls
                .get(chain.name())
                .cloned()
                .unwrap_or_else(|| chain.rpc_url().to_string());

            async move {
                // Acquire rate limit permit
                let _permit = rate_limiter.acquire().await.ok()?;

                tracing::info!("Checking {} chain ({} tokens)", chain.name(), chain_tokens.len());
                let mut chain_balances = Vec::new();

                // Fetch native balance
                match fetch_native_balance_for_chain(&wallet_address, &chain, &rpc_url).await {
                    Ok(Some(balance)) => chain_balances.push(balance),
                    Ok(None) => {},
                    Err(e) => {
                        tracing::error!("Failed to fetch native balance on {}: {}", chain.name(), e);
                    }
                }

                // Fetch token balances using the resolved token list
                match fetch_token_balances_for_chain(&wallet_address, &chain, &chain_tokens, &rpc_url).await {
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
    rpc_url: &str,
) -> Result<Option<Balance>, Box<dyn Error + Send + Sync>> {
    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let address: Address = wallet_address.parse()?;
    
    let balance = provider.get_balance(address).await?;
    
    if balance == U256::ZERO {
        return Ok(None);
    }
    
    let raw_balance_str = balance.to_string();
    // Normalize to human-readable decimal (18 decimals for native tokens)
    let normalized = normalize_token_balance(&raw_balance_str, 18)
        .unwrap_or_else(|_| raw_balance_str.clone());

    Ok(Some(Balance {
        asset: format!("{}-{}", chain.native_symbol(), chain.name()),
        quantity: normalized.clone(),
        available: normalized,
        frozen: "0".to_string(),
        // Native tokens typically have 18 decimals
        decimals: Some(18),
    }))
}

// Helper function to fetch token balances for a chain
async fn fetch_token_balances_for_chain(
    wallet_address: &str,
    chain: &EvmChain,
    token_list: &[(String, String)],
    rpc_url: &str,
) -> Result<Vec<Balance>, Box<dyn Error + Send + Sync>> {
    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let wallet_address: Address = wallet_address.parse()?;
    
    let mut balances = Vec::new();
    
    // Note: provider.clone() is cheap (Arc-based in alloy)
    for (symbol, token_address) in token_list {
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
                    let raw_balance_str = balance.to_string();
                    
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
                    
                    // Normalize to human-readable decimal using on-chain decimals (default 18)
                    let token_decimals = decimals.unwrap_or(18);
                    let normalized = normalize_token_balance(&raw_balance_str, token_decimals)
                        .unwrap_or_else(|_| raw_balance_str.clone());

                    balances.push(Balance {
                        asset: format!("{}-{}", symbol, chain.name()),
                        quantity: normalized.clone(),
                        available: normalized,
                        frozen: "0".to_string(),
                        decimals,
                    });
                    
                    tracing::debug!(
                        "Found {} {} on {} ({}) with {} decimals",
                        raw_balance_str,
                        symbol,
                        chain.name(),
                        token_address,
                        decimals.map_or("unknown".to_string(), |d| d.to_string())
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
        let eth = EvmChain::new("ethereum", "https://eth.llamarpc.com", "ETH");
        assert_eq!(eth.name(), "ethereum");
        assert_eq!(eth.native_symbol(), "ETH");
        assert!(!eth.rpc_url().is_empty());

        let bsc = EvmChain::new("bsc", "https://bsc-dataseed.bnbchain.org", "BNB");
        assert_eq!(bsc.name(), "bsc");
        assert_eq!(bsc.native_symbol(), "BNB");
        assert!(!bsc.rpc_url().is_empty());

        let hl = EvmChain::new("hyper_liquid", "https://rpc.hyperliquid.xyz/evm", "HYPE");
        assert_eq!(hl.name(), "hyper_liquid");
        assert_eq!(hl.native_symbol(), "HYPE");
        assert!(!hl.rpc_url().is_empty());
    }

    #[test]
    fn test_connector_creation() {
        let eth = EvmChain::new("ethereum", "https://eth.llamarpc.com", "ETH");

        // Valid address – default (built-in) token list
        let result = EvmConnector::new(
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
            vec![eth],
        );
        assert!(result.is_ok());

        // Invalid address
        let result = EvmConnector::new(
            "invalid-address".to_string(),
            vec![EvmChain::new("ethereum", "https://eth.llamarpc.com", "ETH")],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_connector_creation_with_custom_tokens() {
        let mut custom: HashMap<String, Vec<(String, String)>> = HashMap::new();
        custom.insert(
            "ethereum".to_string(),
            vec![("MYTOKEN".to_string(), "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string())],
        );

        let result = EvmConnector::new_with_tokens(
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
            vec![EvmChain::new("ethereum", "https://eth.llamarpc.com", "ETH")],
            Some(custom),
            None,
        );
        assert!(result.is_ok());
        let connector = result.unwrap();
        let custom_ref = connector.custom_tokens.as_ref().unwrap();
        assert!(custom_ref.contains_key("ethereum"));
        assert_eq!(custom_ref["ethereum"][0].0, "MYTOKEN");
    }

    #[test]
    fn test_common_tokens() {
        let eth = EvmChain::new("ethereum", "", "ETH");
        let eth_tokens = get_common_tokens(&eth);
        assert!(!eth_tokens.is_empty());
        assert!(eth_tokens.iter().any(|(symbol, _)| *symbol == "USDC"));
        assert!(eth_tokens.iter().any(|(symbol, _)| *symbol == "WBTC"));
        assert!(eth_tokens.iter().any(|(symbol, _)| *symbol == "LINK"));
        assert!(eth_tokens.iter().any(|(symbol, _)| *symbol == "UNI"));
        assert!(eth_tokens.iter().any(|(symbol, _)| *symbol == "AAVE"));
        assert!(eth_tokens.iter().any(|(symbol, _)| *symbol == "WSTETH"));

        let arb_tokens = get_common_tokens(&EvmChain::new("arbitrum", "", "ETH"));
        assert!(!arb_tokens.is_empty());
        assert!(arb_tokens.iter().any(|(symbol, _)| *symbol == "ARB"));
        assert!(arb_tokens.iter().any(|(symbol, _)| *symbol == "GMX"));

        let op_tokens = get_common_tokens(&EvmChain::new("optimism", "", "ETH"));
        assert!(!op_tokens.is_empty());
        assert!(op_tokens.iter().any(|(symbol, _)| *symbol == "OP"));

        let base_tokens = get_common_tokens(&EvmChain::new("base", "", "ETH"));
        assert!(!base_tokens.is_empty());
        assert!(base_tokens.iter().any(|(symbol, _)| *symbol == "CBETH"));

        let bsc_tokens = get_common_tokens(&EvmChain::new("bsc", "", "BNB"));
        assert!(!bsc_tokens.is_empty());
        assert!(bsc_tokens.iter().any(|(symbol, _)| *symbol == "USDT"));
        assert!(bsc_tokens.iter().any(|(symbol, _)| *symbol == "WBNB"));
        assert!(bsc_tokens.iter().any(|(symbol, _)| *symbol == "CAKE"));
        assert!(bsc_tokens.iter().any(|(symbol, _)| *symbol == "BTCB"));

        // Chains not in the built-in list return an empty vec; token data comes from DB
        let hl_tokens = get_common_tokens(&EvmChain::new("hyper_liquid", "", "HYPE"));
        assert!(hl_tokens.is_empty());
    }
}
