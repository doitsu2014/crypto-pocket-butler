use super::{Balance, ExchangeConnector};
use crate::concurrency::RateLimiter;
use crate::helpers::balance_normalization::normalize_token_balance;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use tracing;

/// SOL native token has 9 decimal places (1 SOL = 1_000_000_000 lamports)
pub const SOLANA_NATIVE_DECIMALS: u8 = 9;

/// SPL Token program ID — the standard Solana token program
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Well-known SPL token mint addresses (symbol, mint_address)
///
/// Only a curated list is tracked. Unrecognized mints are skipped during sync.
/// Extend via the admin UI or by adding entries here.
pub fn get_common_solana_tokens() -> Vec<(&'static str, &'static str)> {
    vec![
        // Stablecoins
        ("USDC",   "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        ("USDT",   "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"),
        // Liquid staking / wrapped
        ("MSOL",   "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"),  // Marinade SOL
        ("STSOL",  "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj"),  // Lido staked SOL
        ("JITOSOL", "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn"), // Jito SOL
        // DeFi / blue chips
        ("RAY",    "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"),  // Raydium
        ("JUP",    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"),   // Jupiter
        ("PYTH",   "HZ1JovNiVvGrk8Zas8vbMGMBBHHNzFn2Gb8E9Z4vNxBL"),  // Pyth Network
        ("JTO",    "jtojtomepa8beP8AuQc6eXt5FriJwfFMwjx2v2f9mUL"),   // Jito governance
        // Meme tokens
        ("BONK",   "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),  // Bonk
        ("WIF",    "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm"),  // dogwifhat
    ]
}

// ── JSON-RPC response types ────────────────────────────────────────────────

#[derive(Deserialize)]
struct RpcResponse<T> {
    result: T,
}

#[derive(Deserialize)]
struct BalanceResult {
    value: u64,
}

#[derive(Deserialize)]
struct TokenAccountsResult {
    value: Vec<TokenAccountEntry>,
}

#[derive(Deserialize)]
struct TokenAccountEntry {
    account: TokenAccount,
}

#[derive(Deserialize)]
struct TokenAccount {
    data: TokenAccountData,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TokenAccountData {
    Parsed { parsed: ParsedTokenInfo },
    #[allow(dead_code)]
    Other(serde_json::Value),
}

#[derive(Deserialize)]
struct ParsedTokenInfo {
    info: TokenInfo,
}

#[derive(Deserialize)]
struct TokenInfo {
    mint: String,
    #[serde(rename = "tokenAmount")]
    token_amount: TokenAmount,
}

#[derive(Deserialize)]
struct TokenAmount {
    #[serde(rename = "uiAmountString")]
    ui_amount_string: String,
    amount: String,
}

// ── SolanaConnector ────────────────────────────────────────────────────────

/// Solana wallet connector that fetches native SOL and SPL token balances
/// using direct JSON-RPC calls (no Solana SDK dependency).
///
/// Uses two RPC calls:
/// 1. `getBalance` — native SOL balance in lamports
/// 2. `getTokenAccountsByOwner` — all SPL token accounts in one call
pub struct SolanaConnector {
    wallet_address: String,
    rpc_url: String,
    /// Mapping from mint address → symbol for tokens we recognise.
    /// Built from DB-sourced list if available, otherwise from `get_common_solana_tokens()`.
    token_map: HashMap<String, String>,
    http_client: Client,
}

impl SolanaConnector {
    /// Create a new Solana connector.
    ///
    /// # Arguments
    /// * `wallet_address` — Base58 Solana public key
    /// * `rpc_url` — Solana JSON-RPC endpoint (e.g., `https://api.mainnet-beta.solana.com`)
    /// * `custom_tokens` — Optional DB-sourced list of `(symbol, mint_address)` pairs.
    ///   When `Some`, replaces the built-in `get_common_solana_tokens()` list entirely.
    pub fn new(
        wallet_address: String,
        rpc_url: String,
        custom_tokens: Option<Vec<(String, String)>>,
    ) -> Self {
        let token_map: HashMap<String, String> = match custom_tokens {
            Some(tokens) => tokens.into_iter().map(|(sym, mint)| (mint, sym)).collect(),
            None => get_common_solana_tokens()
                .into_iter()
                .map(|(sym, mint)| (mint.to_string(), sym.to_string()))
                .collect(),
        };

        Self {
            wallet_address,
            rpc_url,
            token_map,
            http_client: Client::new(),
        }
    }

    /// Fetch native SOL balance via `getBalance`.
    async fn fetch_sol_balance(&self) -> Result<Option<Balance>, Box<dyn Error + Send + Sync>> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [self.wallet_address]
        });

        let response: RpcResponse<BalanceResult> = self
            .http_client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        let lamports = response.result.value;
        if lamports == 0 {
            return Ok(None);
        }

        let raw = lamports.to_string();
        let normalized = normalize_token_balance(&raw, SOLANA_NATIVE_DECIMALS)
            .unwrap_or_else(|_| raw.clone());

        Ok(Some(Balance {
            asset: "SOL-solana".to_string(),
            quantity: normalized.clone(),
            available: normalized,
            frozen: "0".to_string(),
            decimals: Some(SOLANA_NATIVE_DECIMALS),
        }))
    }

    /// Fetch all SPL token balances via `getTokenAccountsByOwner`.
    /// Returns only tokens whose mint address is in `self.token_map`.
    async fn fetch_spl_balances(&self) -> Result<Vec<Balance>, Box<dyn Error + Send + Sync>> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                self.wallet_address,
                { "programId": TOKEN_PROGRAM_ID },
                { "encoding": "jsonParsed" }
            ]
        });

        let response: RpcResponse<TokenAccountsResult> = self
            .http_client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        let mut balances = Vec::new();

        for entry in response.result.value {
            let parsed = match entry.account.data {
                TokenAccountData::Parsed { parsed } => parsed,
                TokenAccountData::Other(_) => continue,
            };

            let mint = &parsed.info.mint;
            let symbol = match self.token_map.get(mint) {
                Some(sym) => sym.clone(),
                None => {
                    tracing::debug!("Skipping unrecognised mint: {}", mint);
                    continue;
                }
            };

            // uiAmountString is already normalised by the RPC (e.g. "1.5")
            let ui_amount = &parsed.info.token_amount.ui_amount_string;
            let raw_amount = &parsed.info.token_amount.amount;

            // Skip zero balances
            if ui_amount == "0" || raw_amount == "0" {
                continue;
            }

            balances.push(Balance {
                asset: format!("{}-solana", symbol),
                quantity: ui_amount.clone(),
                available: ui_amount.clone(),
                frozen: "0".to_string(),
                decimals: None,
            });

            tracing::debug!("Found {} {} on solana (mint: {})", ui_amount, symbol, mint);
        }

        Ok(balances)
    }
}

#[async_trait]
impl ExchangeConnector for SolanaConnector {
    async fn fetch_spot_balances(&self) -> Result<Vec<Balance>, Box<dyn Error + Send + Sync>> {
        tracing::info!(
            "Fetching Solana balances for wallet {} ({} known tokens)",
            self.wallet_address,
            self.token_map.len()
        );

        let rate_limiter = RateLimiter::solana_rpc();
        let _permit = rate_limiter.acquire().await?;

        let mut all_balances = Vec::new();

        // Fetch native SOL balance
        match self.fetch_sol_balance().await {
            Ok(Some(balance)) => all_balances.push(balance),
            Ok(None) => {}
            Err(e) => tracing::error!("Failed to fetch SOL balance: {}", e),
        }

        // Fetch SPL token balances
        match self.fetch_spl_balances().await {
            Ok(balances) => all_balances.extend(balances),
            Err(e) => tracing::error!("Failed to fetch SPL token balances: {}", e),
        }

        tracing::info!(
            "Fetched {} total balances for Solana wallet {}",
            all_balances.len(),
            self.wallet_address
        );

        Ok(all_balances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_solana_tokens_not_empty() {
        let tokens = get_common_solana_tokens();
        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|(sym, _)| *sym == "USDC"));
        assert!(tokens.iter().any(|(sym, _)| *sym == "USDT"));
        assert!(tokens.iter().any(|(sym, _)| *sym == "BONK"));
        assert!(tokens.iter().any(|(sym, _)| *sym == "JUP"));
    }

    #[test]
    fn test_connector_creation_with_builtin_tokens() {
        let connector = SolanaConnector::new(
            "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".to_string(),
            "https://api.mainnet-beta.solana.com".to_string(),
            None,
        );
        assert!(!connector.token_map.is_empty());
        // USDC mint should be in map
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        assert_eq!(connector.token_map.get(usdc_mint).map(|s| s.as_str()), Some("USDC"));
    }

    #[test]
    fn test_connector_creation_with_custom_tokens() {
        let custom = vec![
            ("MYTOKEN".to_string(), "SomeMintAddress11111111111111111111111111111".to_string()),
        ];
        let connector = SolanaConnector::new(
            "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".to_string(),
            "https://api.mainnet-beta.solana.com".to_string(),
            Some(custom),
        );
        assert_eq!(connector.token_map.len(), 1);
        assert!(connector.token_map.contains_key("SomeMintAddress11111111111111111111111111111"));
    }

    #[test]
    fn test_solana_native_decimals() {
        // 1 SOL = 1_000_000_000 lamports
        let normalized = normalize_token_balance("1000000000", SOLANA_NATIVE_DECIMALS).unwrap();
        assert_eq!(normalized, "1");

        let normalized = normalize_token_balance("1500000000", SOLANA_NATIVE_DECIMALS).unwrap();
        assert_eq!(normalized, "1.50");
    }
}
