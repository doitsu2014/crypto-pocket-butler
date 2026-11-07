//! The signed, short-lived pre-sign-up gate token (WF6 step 4, C15).
//!
//! HS256 with a key held in Supabase Vault and mounted as a Docker secret. The
//! Auth pre-sign-up hook verifies it; a forged or expired token is refused.

use chrono::{DateTime, Duration, Utc};
use identity_application::ports::{GateGrant, GateTokenIssuer};
use identity_domain::GATE_TOKEN_TTL_MINUTES;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct GateClaims {
    jurisdiction: String,
    iat: i64,
    exp: i64,
}

/// Signs and verifies gate tokens with a shared HMAC key.
pub struct Hs256GateTokens {
    key: Vec<u8>,
}

impl Hs256GateTokens {
    /// Creates the issuer from the shared secret.
    #[must_use]
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self { key: secret.into() }
    }
}

impl GateTokenIssuer for Hs256GateTokens {
    fn issue(&self, jurisdiction: &str, now: DateTime<Utc>) -> Result<String, String> {
        let claims = GateClaims {
            jurisdiction: jurisdiction.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::minutes(GATE_TOKEN_TTL_MINUTES)).timestamp(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&self.key),
        )
        .map_err(|error| format!("gate token signing failed: {error}"))
    }

    fn verify(&self, token: &str, now: DateTime<Utc>) -> Result<GateGrant, String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false;
        let data = decode::<GateClaims>(token, &DecodingKey::from_secret(&self.key), &validation)
            .map_err(|error| format!("gate token is not valid: {error}"))?;
        let expires_at = DateTime::<Utc>::from_timestamp(data.claims.exp, 0)
            .ok_or_else(|| "gate token expiry is not a timestamp".to_string())?;
        let issued_at = DateTime::<Utc>::from_timestamp(data.claims.iat, 0)
            .ok_or_else(|| "gate token issue time is not a timestamp".to_string())?;
        if now >= expires_at {
            return Err("gate token has expired".to_string());
        }
        Ok(GateGrant {
            jurisdiction: data.claims.jurisdiction,
            issued_at,
            expires_at,
        })
    }
}
