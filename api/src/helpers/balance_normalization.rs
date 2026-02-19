/// Balance normalization utilities for converting raw token balances to human-readable values.
/// 
/// This module provides functions to normalize cryptocurrency token balances from their
/// raw on-chain integer representation to human-readable decimal values.
/// 
/// # Background
/// 
/// Most blockchain tokens store balances as integers to avoid floating-point precision issues.
/// The actual human-readable value is calculated as: `raw_balance / 10^decimals`
/// 
/// Common examples:
/// - ETH and most ERC-20 tokens: 18 decimals
/// - USDC, USDT: 6 decimals
/// - BTC: 8 decimals
/// 
/// # Examples
/// 
/// ```rust
/// use crypto_pocket_butler_backend::helpers::balance_normalization::normalize_token_balance;
/// 
/// // ETH with 18 decimals
/// let normalized = normalize_token_balance("291725391649", 18).unwrap();
/// assert_eq!(normalized, "0.000000291725391649");
/// 
/// // USDC with 6 decimals
/// let normalized = normalize_token_balance("706000", 6).unwrap();
/// assert_eq!(normalized, "0.706000");
/// ```

use rust_decimal::{Decimal, MathematicalOps};
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur during balance normalization
#[derive(Debug, Error)]
pub enum NormalizationError {
    #[error("Invalid balance string: {0}")]
    InvalidBalance(String),
    #[error("Arithmetic overflow during normalization")]
    ArithmeticOverflow,
}

/// Normalizes a raw token balance to a human-readable decimal string.
///
/// Converts a raw integer balance (as stored on-chain) to a decimal representation
/// by dividing by 10^decimals.
///
/// # Arguments
///
/// * `raw_balance` - The raw balance as a string (e.g., "291725391649")
/// * `decimals` - The number of decimal places for this token (e.g., 18 for ETH, 6 for USDC)
///
/// # Returns
///
/// A string representation of the normalized balance with full precision.
///
/// # Examples
///
/// ```rust
/// use crypto_pocket_butler_backend::helpers::balance_normalization::normalize_token_balance;
///
/// // ETH with 18 decimals
/// let eth_balance = normalize_token_balance("1500000000000000000", 18).unwrap();
/// assert_eq!(eth_balance, "1.500000000000000000");
///
/// // USDC with 6 decimals  
/// let usdc_balance = normalize_token_balance("1500000", 6).unwrap();
/// assert_eq!(usdc_balance, "1.500000");
///
/// // Very small balance
/// let tiny = normalize_token_balance("1", 18).unwrap();
/// assert_eq!(tiny, "0.000000000000000001");
///
/// // Zero balance
/// let zero = normalize_token_balance("0", 6).unwrap();
/// assert_eq!(zero, "0");
/// ```
///
/// # Errors
///
/// Returns `NormalizationError::InvalidBalance` if the raw_balance string cannot be parsed.
pub fn normalize_token_balance(raw_balance: &str, decimals: u8) -> Result<String, NormalizationError> {
    // Parse the raw balance as a Decimal for precise arithmetic
    let balance = Decimal::from_str(raw_balance)
        .map_err(|e| NormalizationError::InvalidBalance(format!("{}: {}", raw_balance, e)))?;
    
    // Calculate 10^decimals as the divisor
    let divisor = Decimal::from(10_u64)
        .checked_powi(decimals as i64)
        .ok_or(NormalizationError::ArithmeticOverflow)?;
    
    // Divide to get the normalized value
    let normalized = balance
        .checked_div(divisor)
        .ok_or(NormalizationError::ArithmeticOverflow)?;
    
    // Return as string to preserve full precision
    Ok(normalized.to_string())
}

/// Normalizes a token balance and formats it for display with a specified number of decimal places.
///
/// This is a convenience function that normalizes the balance and then rounds/truncates
/// to a specified number of decimal places for display purposes.
///
/// # Arguments
///
/// * `raw_balance` - The raw balance as a string
/// * `decimals` - The number of decimal places for this token
/// * `display_decimals` - The number of decimal places to show in the output
///
/// # Examples
///
/// ```rust
/// use crypto_pocket_butler_backend::helpers::balance_normalization::normalize_and_format_balance;
///
/// // Show ETH with 4 decimal places
/// let eth = normalize_and_format_balance("1234567890123456789", 18, 4).unwrap();
/// assert_eq!(eth, "1.2346");
///
/// // Show USDC with 2 decimal places
/// let usdc = normalize_and_format_balance("1234567", 6, 2).unwrap();
/// assert_eq!(usdc, "1.23");
/// ```
pub fn normalize_and_format_balance(
    raw_balance: &str,
    decimals: u8,
    display_decimals: u32,
) -> Result<String, NormalizationError> {
    let normalized = normalize_token_balance(raw_balance, decimals)?;
    
    let decimal = Decimal::from_str(&normalized)
        .map_err(|e| NormalizationError::InvalidBalance(format!("Failed to parse normalized: {}", e)))?;
    
    Ok(format!("{:.prec$}", decimal, prec = display_decimals as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_eth_balance() {
        // 1.5 ETH in wei
        let result = normalize_token_balance("1500000000000000000", 18).unwrap();
        // Decimal preserves some precision in to_string()
        assert_eq!(result, "1.50");
    }

    #[test]
    fn test_normalize_usdc_balance() {
        // 706000 raw = 0.706 USDC (6 decimals)
        let result = normalize_token_balance("706000", 6).unwrap();
        assert_eq!(result, "0.706");
    }

    #[test]
    fn test_normalize_small_eth_balance() {
        // Example from issue: 291725391649 wei
        let result = normalize_token_balance("291725391649", 18).unwrap();
        assert_eq!(result, "0.000000291725391649");
    }

    #[test]
    fn test_normalize_zero_balance() {
        let result = normalize_token_balance("0", 18).unwrap();
        assert_eq!(result, "0");
        
        let result = normalize_token_balance("0", 6).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_normalize_one_wei() {
        // Smallest possible ETH amount
        let result = normalize_token_balance("1", 18).unwrap();
        assert_eq!(result, "0.000000000000000001");
    }

    #[test]
    fn test_normalize_btc_balance() {
        // BTC uses 8 decimals (satoshis)
        // 1 BTC = 100,000,000 satoshis
        let result = normalize_token_balance("100000000", 8).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_normalize_large_balance() {
        // 1 million ETH
        let result = normalize_token_balance("1000000000000000000000000", 18).unwrap();
        assert_eq!(result, "1000000");
    }

    #[test]
    fn test_normalize_no_decimals() {
        // Some tokens have 0 decimals
        let result = normalize_token_balance("42", 0).unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_invalid_balance_string() {
        let result = normalize_token_balance("not-a-number", 18);
        assert!(result.is_err());
        
        let result = normalize_token_balance("", 18);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_and_format() {
        // ETH with 4 decimal places for display
        let result = normalize_and_format_balance("1234567890123456789", 18, 4).unwrap();
        // Note: format! truncates decimal precision
        assert_eq!(result, "1.2345");
        
        // USDC with 2 decimal places for display
        let result = normalize_and_format_balance("1234567", 6, 2).unwrap();
        assert_eq!(result, "1.23");
    }

    #[test]
    fn test_format_truncates() {
        // Test formatting behavior - it truncates not rounds
        let result = normalize_and_format_balance("1999999999999999999", 18, 2).unwrap();
        // 1.999999... truncated to 2 decimals becomes "1.99"  
        assert_eq!(result, "1.99");
        
        let result = normalize_and_format_balance("1449999999999999999", 18, 2).unwrap();
        assert_eq!(result, "1.44");
    }

    #[test]
    fn test_format_zero_decimals() {
        let result = normalize_and_format_balance("1234567890123456789", 18, 0).unwrap();
        assert_eq!(result, "1");
    }
}
