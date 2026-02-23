use crate::connectors::{okx::OkxConnector, evm::{EvmConnector, EvmChain}, solana::SolanaConnector, ExchangeConnector};
use crate::entities::{accounts, evm_chains, evm_tokens, holdings, holding_transactions, solana_tokens};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
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
                    // Use SOLANA_RPC_URL env var; fall back to public mainnet endpoint
                    let rpc_url = std::env::var("SOLANA_RPC_URL")
                        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
                    let db_tokens = load_solana_tokens_from_db(db).await;
                    Box::new(SolanaConnector::new(
                        wallet_address.clone(),
                        rpc_url,
                        db_tokens,
                    ))
                }
                _ => {
                    // Load all active EVM chains from DB (carries chain_id, rpc_url, native_symbol)
                    let all_chains = load_evm_chains_from_db(db).await;

                    // Filter to the account's enabled_chains subset, or use all if unset
                    let chains = if let Some(enabled_chains_json) = &account.enabled_chains {
                        match serde_json::from_value::<Vec<String>>(enabled_chains_json.clone()) {
                            Ok(chain_names) => {
                                let filtered: Vec<EvmChain> = all_chains
                                    .iter()
                                    .filter(|c| chain_names.contains(&c.name().to_string()))
                                    .cloned()
                                    .collect();
                                if filtered.is_empty() {
                                    tracing::warn!(
                                        "None of enabled_chains {:?} matched active DB chains; using all",
                                        chain_names
                                    );
                                    all_chains
                                } else {
                                    filtered
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse enabled_chains: {}, using all chains", e);
                                all_chains
                            }
                        }
                    } else {
                        all_chains
                    };

                    // Load token list from DB; fall back to built-in list on error
                    let db_tokens = load_tokens_from_db(db).await;

                    // RPC URLs are already embedded in each EvmChain struct (loaded from DB above).
                    // No separate rpc_url override map is needed.
                    match EvmConnector::new_with_tokens(wallet_address.clone(), chains, db_tokens, None) {
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
    let holdings_json: Vec<serde_json::Value> = balances
        .iter()
        .map(|b| {
            json!({
                "asset": b.asset,
                "quantity": b.quantity,
            })
        })
        .collect();

    let holdings_count = holdings_json.len();

    // Persist structured holdings + transactions for audit trail.
    // Errors are logged internally but do not fail the sync – the JSON column update
    // below is the source of truth for backward-compatible clients.
    let source = account
        .exchange_name
        .as_deref()
        .unwrap_or(&account.account_type);
    upsert_holdings_with_transactions(db, account_id, &balances, source).await;

    // Update account's last_synced_at and holdings (JSON kept for backward compatibility)
    let mut account_update: accounts::ActiveModel = account.into();
    account_update.last_synced_at = ActiveValue::Set(Some(Utc::now().into()));
    account_update.holdings = ActiveValue::Set(Some(
        serde_json::to_value(&holdings_json)
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

/// Upsert holdings rows and append a transaction record for each changed balance.
///
/// For every balance:
/// 1. Look up (or create) the [`holdings::Model`] for `(account_id, asset_symbol)`.
/// 2. If the quantity changed, write a [`holding_transactions::Model`] audit entry and
///    update the `holdings.quantity`.
///
/// Errors are logged but do **not** fail the overall sync so that a partial DB
/// issue cannot prevent the account JSON from being updated.
async fn upsert_holdings_with_transactions(
    db: &DatabaseConnection,
    account_id: Uuid,
    balances: &[crate::connectors::Balance],
    source: &str,
) {
    let now = Utc::now();

    for balance in balances {
        let new_qty_str = balance.quantity.trim().to_string();

        // Find or create the holdings row for this (account_id, asset_symbol)
        let existing = holdings::Entity::find()
            .filter(holdings::Column::AccountId.eq(account_id))
            .filter(holdings::Column::AssetSymbol.eq(&balance.asset))
            .one(db)
            .await;

        let existing = match existing {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "Failed to query holding for account {} asset {}: {}",
                    account_id, balance.asset, e
                );
                continue;
            }
        };

        let (holding_id, old_qty_str) = match existing {
            Some(ref h) => (h.id, h.quantity.clone()),
            None => {
                // Create new holding row
                let new_holding = holdings::ActiveModel {
                    id: ActiveValue::Set(Uuid::new_v4()),
                    account_id: ActiveValue::Set(account_id),
                    asset_symbol: ActiveValue::Set(balance.asset.clone()),
                    quantity: ActiveValue::Set(new_qty_str.clone()),
                    created_at: ActiveValue::Set(now.into()),
                    updated_at: ActiveValue::Set(now.into()),
                };
                match new_holding.insert(db).await {
                    Ok(inserted) => (inserted.id, "0".to_string()),
                    Err(e) => {
                        tracing::warn!(
                            "Failed to insert holding for account {} asset {}: {}",
                            account_id, balance.asset, e
                        );
                        continue;
                    }
                }
            }
        };

        // Only write a transaction + update the row when the quantity has actually changed
        let old_qty = Decimal::from_str(&old_qty_str).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse old quantity '{}' for holding {}: {}", old_qty_str, holding_id, e);
            Decimal::ZERO
        });
        let new_qty = Decimal::from_str(&new_qty_str).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse new quantity '{}' for account {} asset {}: {}", new_qty_str, account_id, balance.asset, e);
            Decimal::ZERO
        });

        if old_qty == new_qty && existing.is_some() {
            // No change – skip writing a redundant transaction
            continue;
        }

        let qty_change = new_qty - old_qty;

        // Append audit transaction
        let tx = holding_transactions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            holding_id: ActiveValue::Set(holding_id),
            quantity_before: ActiveValue::Set(old_qty_str),
            quantity_after: ActiveValue::Set(new_qty_str.clone()),
            quantity_change: ActiveValue::Set(qty_change.to_string()),
            transaction_type: ActiveValue::Set("sync".to_string()),
            source: ActiveValue::Set(source.to_string()),
            metadata: ActiveValue::Set(None),
            created_at: ActiveValue::Set(now.into()),
            updated_at: ActiveValue::Set(now.into()),
        };

        if let Err(e) = tx.insert(db).await {
            tracing::warn!(
                "Failed to insert holding transaction for holding {}: {}",
                holding_id, e
            );
        }

        // Update the holding's current quantity if it already existed
        if let Some(h) = existing {
            let mut h_update: holdings::ActiveModel = h.into();
            h_update.quantity = ActiveValue::Set(new_qty_str);
            h_update.updated_at = ActiveValue::Set(now.into());
            if let Err(e) = h_update.update(db).await {
                tracing::warn!("Failed to update holding {}: {}", holding_id, e);
            }
        }
    }
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

/// Load all active EVM chains from the database as [`EvmChain`] structs.
///
/// Each returned struct carries the chain's `chain_id`, `rpc_url`, and `native_symbol`
/// directly from the `evm_chains` table, so no separate RPC URL map is needed.
///
/// Falls back to a small hardcoded set when the database is unreachable, ensuring
/// the sync job can still operate in degraded-DB conditions.
async fn load_evm_chains_from_db(db: &DatabaseConnection) -> Vec<EvmChain> {
    match evm_chains::Entity::find()
        .filter(evm_chains::Column::IsActive.eq(true))
        .all(db)
        .await
    {
        Ok(rows) if !rows.is_empty() => {
            tracing::info!("Loaded {} active EVM chains from DB", rows.len());
            rows.into_iter()
                .map(|r| EvmChain::new(r.chain_id, r.rpc_url, r.native_symbol))
                .collect()
        }
        Ok(_) => {
            tracing::warn!("evm_chains table is empty, using hardcoded defaults");
            EvmChain::defaults()
        }
        Err(e) => {
            tracing::warn!("Failed to load EVM chains from DB: {}, using hardcoded defaults", e);
            EvmChain::defaults()
        }
    }
}

/// Load active Solana SPL tokens from the database.
///
/// Returns `Some(vec)` of `(symbol, mint_address)` pairs when the table is reachable
/// and contains rows. Falls back to `None` on any DB error so the Solana connector
/// uses its built-in token list.
async fn load_solana_tokens_from_db(
    db: &DatabaseConnection,
) -> Option<Vec<(String, String)>> {
    match solana_tokens::Entity::find()
        .filter(solana_tokens::Column::IsActive.eq(true))
        .all(db)
        .await
    {
        Ok(rows) if !rows.is_empty() => {
            let pairs: Vec<(String, String)> = rows
                .into_iter()
                .map(|row| (row.symbol, row.mint_address))
                .collect();
            tracing::info!(
                "Loaded {} active Solana tokens from DB",
                pairs.len()
            );
            Some(pairs)
        }
        Ok(_) => {
            tracing::warn!("solana_tokens table is empty, falling back to built-in token list");
            None
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load Solana tokens from DB: {}, falling back to built-in token list",
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
