/**
 * Centralized export for all TanStack Query hooks
 */

// Portfolio hooks
export {
  usePortfolios,
  usePortfolio,
  usePortfolioHoldings,
  useCreatePortfolio,
  useUpdatePortfolio,
  useDeletePortfolio,
  portfolioKeys,
} from "./usePortfolios";

// Account hooks
export {
  useAccounts,
  useAccount,
  useCreateAccount,
  useUpdateAccount,
  useDeleteAccount,
  useSyncAccount,
  accountKeys,
} from "./useAccounts";

// Snapshot hooks
export {
  useSnapshots,
  useCreateSnapshot,
  DEFAULT_SNAPSHOT_TYPE,
  snapshotKeys,
} from "./useSnapshots";

// Recommendation hooks
export {
  useRecommendations,
  useRecommendation,
  useGenerateRecommendations,
  useApproveRecommendation,
  useRejectRecommendation,
  useExecuteRecommendation,
  recommendationKeys,
} from "./useRecommendations";

// Chain hooks
export {
  useChains,
  chainKeys,
} from "./useChains";
