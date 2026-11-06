//! A usable price is strictly positive. Zero is not a price (BR2.2).

use rust_decimal::Decimal;

use crate::error::DomainError;

/// A usable, strictly positive price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(Decimal);

impl Price {
    /// Creates a usable price.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidPrice`] for zero, negative, or non-finite
    /// values. A provider that cannot price an asset must return `None`, never
    /// a zero.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if !value.is_sign_positive() || value.is_zero() {
            return Err(DomainError::InvalidPrice(format!(
                "price must be greater than zero, got {value}"
            )));
        }
        Ok(Self(value))
    }

    /// The decimal value.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}
