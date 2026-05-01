/// Portfolio DTOs — request and response types for Portfolio API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Request to create a new portfolio.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePortfolioDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request to update a portfolio.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdatePortfolioDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request to add an account to a portfolio.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddAccountToPortfolioDto {
    pub account_id: Uuid,
}

/// API response for a portfolio.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PortfolioResponseDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_constructed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
