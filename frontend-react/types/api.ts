/**
 * Centralized TypeScript type definitions for API entities
 * Used across hooks and components for type safety
 */

// ============================================================================
// Portfolio Types
// ============================================================================

export interface Portfolio {
  id: string;
  user_id: string;
  name: string;
  description?: string;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

export interface AssetHolding {
  asset: string;
  quantity: string;
  avg_cost_usd: string;
  current_value_usd: string;
  percentage: number;
  pnl_usd?: string;
  pnl_percentage?: number;
}

export interface PortfolioHoldingsResponse {
  portfolio: Portfolio;
  holdings: AssetHolding[];
  total_value_usd: string;
  last_updated: string;
}

export interface CreatePortfolioInput {
  name: string;
  description?: string;
  is_default?: boolean;
}

// ============================================================================
// Account Types
// ============================================================================

export interface AccountHolding {
  asset: string;
  quantity: string;
}

export interface Account {
  id: string;
  user_id: string;
  name: string;
  account_type: string;
  exchange_name?: string;
  wallet_address?: string;
  enabled_chains?: string[];
  is_active: boolean;
  last_synced_at?: string;
  holdings?: AccountHolding[];
  created_at: string;
  updated_at: string;
}

export interface CreateWalletAccountInput {
  name: string;
  account_type: "wallet";
  wallet_address: string;
  enabled_chains?: string[];
}

export interface CreateExchangeAccountInput {
  name: string;
  account_type: "exchange";
  exchange_name: string;
  api_key: string;
  api_secret: string;
  passphrase?: string;
}

export type CreateAccountInput = CreateWalletAccountInput | CreateExchangeAccountInput;

export interface SyncAccountResult {
  account_id: string;
  success: boolean;
  error?: string;
  holdings_count: number;
}

// ============================================================================
// Snapshot Types
// ============================================================================

export interface Snapshot {
  id: string;
  portfolio_id: string;
  snapshot_date: string;
  snapshot_type: string;
  total_value_usd: string;
  holdings: unknown; // Can be typed more specifically if needed
  metadata?: unknown;
  created_at: string;
}

export interface ListSnapshotsResponse {
  portfolio_id: string;
  snapshots: Snapshot[];
  total_count: number;
}

export interface SnapshotsQueryParams {
  snapshot_type?: string;
  start_date?: string;
  end_date?: string;
  limit?: number;
}

export interface CreateSnapshotInput {
  snapshot_type?: string;
  snapshot_date?: string;
}

export interface SnapshotResult {
  portfolio_id: string;
  snapshot_id?: string;
  success: boolean;
  error?: string;
  holdings_count: number;
  total_value_usd: string; // String for precise decimal handling (matches backend BigDecimal serialization)
}

// ============================================================================
// Recommendation Types
// ============================================================================

export interface ProposedOrder {
  action: string;
  asset: string;
  quantity: string;
  estimated_price: string;
  estimated_value_usd: string;
}

export interface Recommendation {
  id: string;
  portfolio_id: string;
  status: string;
  recommendation_type: string;
  rationale: string;
  proposed_orders: ProposedOrder[];
  expected_impact?: string;
  metadata?: {
    risk_score?: number;
    confidence?: number;
    [key: string]: unknown;
  };
  created_at: string;
  updated_at: string;
  executed_at?: string;
}

export interface ListRecommendationsResponse {
  portfolio_id: string;
  recommendations: Recommendation[];
  total_count: number;
}

export interface RecommendationsQueryParams {
  status?: string;
  recommendation_type?: string;
  limit?: number;
}

export interface RecommendationDetailResponse {
  recommendation: Recommendation;
  portfolio: Portfolio;
}

// ============================================================================
// Chain Types
// ============================================================================

export interface Chain {
  id: string;
  name: string;
  native_symbol: string;
}
