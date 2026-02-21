use crate::connectors::{okx::OkxConnector, evm::{EvmConnector, EvmChain}, ExchangeConnector};
// TODO: Solana connector temporarily disabled due to dependency conflicts
// use crate::connectors::solana::SolanaConnector;
use crate::entities::{accounts, evm_chains, evm_tokens};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use tracing;
use uuid::Uuid;

/// Result of syncing an account
#[derive(Debug)]
pub struct SyncResult {
    pub account_id: Uuid,
    pub success: bool,
    pub error: Option<String>,
    pub holdings_count: usize,
}

/// Decrypt API credentials (placeholder - implement proper encryption/decryption)
/// 
/// SECURITY WARNING: This is a placeholder implementation that stores credentials in plain text.
/// Before production deployment, this MUST be replaced with proper encryption using:
/// - AWS KMS (Key Management Service)
/// - HashiCorp Vault
/// - Google Cloud KMS
/// - Azure Key Vault
/// or similar key management solution.
fn decrypt_credential(encrypted: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    // TODO: Implement proper decryption using a key management service
    // For now, assuming credentials are stored as-is (not recommended for production)
    Ok(encrypted.to_string())
}

/// Sync a single account and create a snapshot
pub async fn sync_account(
    db: &DatabaseConnection,
    account_id: Uuid,
) -> Result<SyncResult, Box<dyn Error + Send + Sync>> {
    tracing::info!("Starting sync for account {}", account_id);

    // Fetch account from database
    let account = accounts::Entity::find_by_id(account_id)
        .one(db)
        .await?
        .ok_or_else(|| format!("Account {} not found", account_id))?;

    // Check if account is active
    if !account.is_active {
        return Ok(SyncResult {
            account_id,
            success: false,
            error: Some("Account is not active".to_string()),
            holdings_count: 0,
        });
    }

    // Handle different account types
    let connector: Box<dyn ExchangeConnector> = match account.account_type.as_str() {
        "exchange" => {
            // Handle exchange accounts (OKX)
            let exchange_name = account
                .exchange_name
                .as_ref()
                .ok_or_else(|| "Exchange name not set")?;

            if exchange_name.to_lowercase() != "okx" {
                return Ok(SyncResult {
                    account_id,
                    success: false,
                    error: Some(format!("Unsupported exchange: {}", exchange_name)),
                    holdings_count: 0,
                });
            }

            // Get API credentials
            let api_key = account
                .api_key_encrypted
                .as_ref()
                .ok_or_else(|| "API key not set")?;
            let api_secret = account
                .api_secret_encrypted
                .as_ref()
                .ok_or_else(|| "API secret not set")?;
            let passphrase = account
                .passphrase_encrypted
                .as_ref()
                .ok_or_else(|| "Passphrase not set")?;

            // Decrypt credentials
            let api_key = decrypt_credential(api_key)?;
            let api_secret = decrypt_credential(api_secret)?;
            let passphrase = decrypt_credential(passphrase)?;

            // Create OKX connector
            Box::new(OkxConnector::new(api_key, api_secret, passphrase))
        }
        "wallet" => {
            // Handle wallet accounts (EVM or Solana)
            let wallet_address = account
                .wallet_address
                .as_ref()
                .ok_or_else(|| "Wallet address not set")?;

            // Check exchange_name to determine wallet type
            // If exchange_name is "solana", Solana connector would be used (not yet available)
            // Otherwise, use EVM connector for all other chains
            match account.exchange_name.as_deref() {
                Some("solana") => {
                    // Solana support coming soon - dependency conflicts being resolved
                    return Ok(SyncResult {
                        account_id,
                        success: false,
                        error: Some("Solana support temporarily unavailable - coming in next update".to_string()),
                        holdings_count: 0,
                    });
                }
                _ => {
                    // Default to EVM chains (Ethereum, Arbitrum, Optimism, Base, BSC)
                    // Use enabled_chains from account if specified, otherwise use all chains
                    let chains = if let Some(enabled_chains_json) = &account.enabled_chains {
                        // Parse enabled_chains from JSON
                        match serde_json::from_value::<Vec<String>>(enabled_chains_json.clone()) {
                            Ok(chain_names) => {
                                // Convert chain names to EvmChain enums
                                let mut chains = Vec::new();
                                for name in chain_names {
                                    match name.as_str() {
                                        "ethereum" => chains.push(EvmChain::Ethereum),
                                        "arbitrum" => chains.push(EvmChain::Arbitrum),
                                        "optimism" => chains.push(EvmChain::Optimism),
                                        "base" => chains.push(EvmChain::Base),
                                        "bsc" => chains.push(EvmChain::BinanceSmartChain),
                                        _ => {
                                            tracing::warn!("Unknown chain name: {}", name);
                                        }
                                    }
                                }
                                if chains.is_empty() {
                                    // If no valid chains were found, use all chains
                                    EvmChain::all()
                                } else {
                                    chains
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse enabled_chains: {}, using all chains", e);
                                EvmChain::all()
                            }
                        }
                    } else {
                        // No enabled_chains specified, use all chains
                        EvmChain::all()
                    };

                    // Load token list from DB; fall back to built-in list on error
                    let db_tokens = load_tokens_from_db(db).await;

                    // Load RPC URLs from DB; fall back to hardcoded defaults on error
                    let db_rpc_urls = load_rpc_urls_from_db(db).await;

                    // Create EVM connector with DB-sourced token list and RPC URLs
                    match EvmConnector::new_with_tokens(wallet_address.clone(), chains, db_tokens, db_rpc_urls) {
                        Ok(connector) => Box::new(connector),
                        Err(e) => {
                            return Ok(SyncResult {
                                account_id,
                                success: false,
                                error: Some(format!("Failed to create EVM connector: {}", e)),
                                holdings_count: 0,
                            });
                        }
                    }
                }
            }
        }
        other => {
            return Ok(SyncResult {
                account_id,
                success: false,
                error: Some(format!("Unsupported account type: {}", other)),
                holdings_count: 0,
            });
        }
    };

    // Fetch balances
    let balances = match connector.fetch_spot_balances().await {
        Ok(balances) => balances,
        Err(e) => {
            tracing::error!("Failed to fetch balances for account {}: {}", account_id, e);
            return Ok(SyncResult {
                account_id,
                success: false,
                error: Some(format!("Failed to fetch balances: {}", e)),
                holdings_count: 0,
            });
        }
    };

    tracing::info!(
        "Fetched {} balances for account {}",
        balances.len(),
        account_id
    );

    // Convert balances to holdings JSON format
    // IMPORTANT: Store ONLY asset symbol and quantity - NO price or valuation fields.
    // This is a core design principle: account holdings are quantity-only.
    // Valuation happens separately during portfolio construction using price reference data.
    // 
    // Note: The Balance struct may contain available/frozen fields (for internal use),
    // but these are intentionally excluded from persisted holdings JSON.
    // Do NOT add available/frozen/price/value/equity fields to the holdings JSON.
    let holdings: Vec<serde_json::Value> = balances
        .iter()
        .map(|b| {
            json!({
                "asset": b.asset,
                "quantity": b.quantity,
            })
        })
        .collect();

    let holdings_count = holdings.len();

    // Update account's last_synced_at and holdings
    let mut account_update: accounts::ActiveModel = account.into();
    account_update.last_synced_at = ActiveValue::Set(Some(Utc::now().into()));
    account_update.holdings = ActiveValue::Set(Some(
        serde_json::to_value(&holdings)
            .map_err(|e| format!("Failed to serialize holdings: {}", e))?
            .into()
    ));
    account_update.update(db).await?;

    tracing::info!(
        "Successfully synced account {} with {} holdings",
        account_id,
        holdings_count
    );

    Ok(SyncResult {
        account_id,
        success: true,
        error: None,
        holdings_count,
    })
}

/// Sync all active accounts for a user
pub async fn sync_user_accounts(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Vec<SyncResult>, Box<dyn Error + Send + Sync>> {
    tracing::info!("Starting sync for all accounts of user {}", user_id);

    // Fetch all active accounts for the user
    let accounts = accounts::Entity::find()
        .filter(accounts::Column::UserId.eq(user_id))
        .filter(accounts::Column::IsActive.eq(true))
        .all(db)
        .await?;

    tracing::info!("Found {} active accounts for user {}", accounts.len(), user_id);

    let mut results = Vec::new();

    for account in accounts {
        match sync_account(db, account.id).await {
            Ok(result) => results.push(result),
            Err(e) => {
                tracing::error!("Failed to sync account {}: {}", account.id, e);
                results.push(SyncResult {
                    account_id: account.id,
                    success: false,
                    error: Some(format!("Sync failed: {}", e)),
                    holdings_count: 0,
                });
            }
        }
    }

    tracing::info!(
        "Completed sync for user {}: {} successful, {} failed",
        user_id,
        results.iter().filter(|r| r.success).count(),
        results.iter().filter(|r| !r.success).count()
    );

    Ok(results)
}

/// Load active EVM tokens from the database grouped by chain name.
///
/// Returns `Some(map)` when the table is reachable and contains rows.
/// Falls back to `None` on any DB error so the EVM connector uses its built-in token list.
async fn load_tokens_from_db(
    db: &DatabaseConnection,
) -> Option<HashMap<String, Vec<(String, String)>>> {
    match evm_tokens::Entity::find()
        .filter(evm_tokens::Column::IsActive.eq(true))
        .all(db)
        .await
    {
        Ok(rows) if !rows.is_empty() => {
            let mut map: HashMap<String, Vec<(String, String)>> = HashMap::new();
            for row in rows {
                map.entry(row.chain)
                    .or_default()
                    .push((row.symbol, row.contract_address));
            }
            tracing::info!(
                "Loaded {} active EVM tokens from DB across {} chains",
                map.values().map(|v| v.len()).sum::<usize>(),
                map.len()
            );
            Some(map)
        }
        Ok(_) => {
            // Table exists but is empty – fall back to built-in list
            tracing::warn!("evm_tokens table is empty, falling back to built-in token list");
            None
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load EVM tokens from DB: {}, falling back to built-in token list",
                e
            );
            None
        }
    }
}

/// Load active EVM chain RPC URLs from the database.
///
/// Returns `Some(map)` where key = `chain_id` (e.g. "ethereum") and value = RPC URL string.
/// Falls back to `None` on any DB error so the EVM connector uses its hardcoded defaults.
async fn load_rpc_urls_from_db(
    db: &DatabaseConnection,
) -> Option<HashMap<String, String>> {
    match evm_chains::Entity::find()
        .filter(evm_chains::Column::IsActive.eq(true))
        .all(db)
        .await
    {
        Ok(rows) if !rows.is_empty() => {
            let map: HashMap<String, String> = rows
                .into_iter()
                .map(|row| (row.chain_id, row.rpc_url))
                .collect();
            tracing::info!(
                "Loaded RPC URLs for {} active EVM chains from DB",
                map.len()
            );
            Some(map)
        }
        Ok(_) => {
            tracing::warn!("evm_chains table is empty, using hardcoded RPC URLs");
            None
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load EVM chain RPC URLs from DB: {}, using hardcoded defaults",
                e
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_credential() {
        let encrypted = "test-credential";
        let decrypted = decrypt_credential(encrypted).unwrap();
        assert_eq!(decrypted, encrypted);
    }

    #[test]
    fn test_holdings_format_qty_only() {
        // Test that holdings JSON contains only asset and quantity, no price/value
        use crate::connectors::Balance;
        
        let balances = vec![
            Balance {
                asset: "BTC".to_string(),
                quantity: "1.5".to_string(),
                available: "1.2".to_string(),
                frozen: "0.3".to_string(),
                decimals: Some(8),
            },
            Balance {
                asset: "ETH".to_string(),
                quantity: "10.0".to_string(),
                available: "8.0".to_string(),
                frozen: "2.0".to_string(),
                decimals: Some(18),
            },
        ];

        // Simulate the holdings conversion logic from sync_account function
        let holdings: Vec<serde_json::Value> = balances
            .iter()
            .map(|b| {
                json!({
                    "asset": b.asset,
                    "quantity": b.quantity,
                })
            })
            .collect();

        // Verify each holding has exactly 2 fields: asset and quantity
        for holding in holdings {
            let obj = holding.as_object().expect("holding should be an object");
            
            // Must have exactly 2 fields
            assert_eq!(obj.len(), 2, "Holdings must have exactly 2 fields");
            
            // Must have asset field
            assert!(obj.contains_key("asset"), "Holdings must have 'asset' field");
            
            // Must have quantity field
            assert!(obj.contains_key("quantity"), "Holdings must have 'quantity' field");
            
            // Must NOT have price/value/available/frozen fields
            assert!(!obj.contains_key("price"), "Holdings must NOT have 'price' field");
            assert!(!obj.contains_key("price_usd"), "Holdings must NOT have 'price_usd' field");
            assert!(!obj.contains_key("value"), "Holdings must NOT have 'value' field");
            assert!(!obj.contains_key("value_usd"), "Holdings must NOT have 'value_usd' field");
            assert!(!obj.contains_key("available"), "Holdings must NOT have 'available' field");
            assert!(!obj.contains_key("frozen"), "Holdings must NOT have 'frozen' field");
            assert!(!obj.contains_key("equity"), "Holdings must NOT have 'equity' field");
        }
    }
}
