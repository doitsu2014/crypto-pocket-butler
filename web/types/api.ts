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

/** Per-account breakdown of a single asset holding */
export interface AccountHoldingDetail {
  account_id: string;
  account_name: string;
  /** Normalized (human-readable) quantity, e.g. "1.5" */
  quantity: string;
  /** Normalized available quantity */
  available: string;
  /** Normalized frozen quantity */
  frozen: string;
  /** Token decimal places metadata (e.g. 18 for ETH, 6 for USDC) */
  decimals?: number;
  /** Same as quantity – kept for backwards compatibility */
  normalized_quantity?: string;
}

/** Aggregated holding for a single asset across all accounts */
export interface AssetHolding {
  asset: string;
  /** Chain label when asset is chain-specific (e.g. "ethereum", "bsc", "solana").
   *  Absent for exchange accounts (OKX) where no chain context applies. */
  chain?: string;
  /** Normalized (human-readable) total quantity, e.g. "1.5" for 1.5 ETH */
  total_quantity: string;
  /** Normalized total available quantity */
  total_available: string;
  /** Normalized total frozen quantity */
  total_frozen: string;
  /** Token decimal places metadata */
  decimals?: number;
  /** Same as total_quantity – kept for backwards compatibility */
  normalized_quantity?: string;
  /** Current price per unit in USD */
  price_usd: number;
  /** Total value in USD (normalized_quantity × price_usd) */
  value_usd: number;
  /** Per-account breakdown */
  accounts: AccountHoldingDetail[];
}

export interface AllocationItem {
  asset: string;
  /** Chain label – mirrors AssetHolding.chain for per-(asset, chain) percentage lookup */
  chain?: string;
  value_usd: number;
  percentage: number;
}

export interface PortfolioHoldingsResponse {
  portfolio_id: string;
  total_value_usd: number;
  holdings: AssetHolding[];
  allocation: AllocationItem[];
  as_of: string;
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
  /** Normalized (human-readable) quantity, e.g. "1.5" for 1.5 ETH */
  quantity: string;
  /** Token decimal places metadata (e.g. 18 for ETH, 6 for USDC) */
  decimals?: number;
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

export interface CreateSolanaWalletAccountInput {
  name: string;
  account_type: "wallet";
  wallet_address: string;
  exchange_name: "solana";
}

export interface CreateExchangeAccountInput {
  name: string;
  account_type: "exchange";
  exchange_name: string;
  api_key: string;
  api_secret: string;
  passphrase?: string;
}

export type CreateAccountInput =
  | CreateWalletAccountInput
  | CreateSolanaWalletAccountInput
  | CreateExchangeAccountInput;

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
  /** Snapshot type (e.g., 'manual', 'eod', 'hourly'). Defaults to 'manual' if not specified. */
  snapshot_type?: string;
  /** Snapshot date in ISO 8601 format (YYYY-MM-DD). Defaults to current date if not specified. */
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

// ============================================================================
// EVM Token Types
// ============================================================================

export interface EvmToken {
  id: string;
  chain: string;
  symbol: string;
  contract_address: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateEvmTokenInput {
  chain: string;
  symbol: string;
  contract_address: string;
  is_active?: boolean;
}

export interface UpdateEvmTokenInput {
  symbol?: string;
  is_active?: boolean;
}

// ============================================================================
// EVM Chain Types
// ============================================================================

export interface EvmChain {
  id: string;
  chain_id: string;
  name: string;
  rpc_url: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateEvmChainInput {
  chain_id: string;
  name: string;
  rpc_url: string;
  is_active?: boolean;
}

export interface UpdateEvmChainInput {
  name?: string;
  rpc_url?: string;
  is_active?: boolean;
}

// ============================================================================
// Solana Token Types
// ============================================================================

export interface SolanaToken {
  id: string;
  symbol: string;
  mint_address: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateSolanaTokenInput {
  symbol: string;
  mint_address: string;
  is_active?: boolean;
}

export interface UpdateSolanaTokenInput {
  symbol?: string;
  is_active?: boolean;
}
