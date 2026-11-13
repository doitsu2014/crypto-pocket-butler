/**
 * C1 gateway DTOs, transcribed from the inception contract summary.
 *
 * Decimal quantities, prices and values are decimal strings supplied by the
 * backend. The SPA renders them as-is and never parses them into binary
 * floating point for business math (BR1.1).
 */

export interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail?: string;
  correlationId: string;
  /** Field-level validation detail, when the gateway supplies it. */
  errors?: Array<{ field: string; reason: string }>;
}

export interface Asset {
  id: string;
  symbol: string;
  name: string;
  decimals: number;
  canonical: boolean;
}

export interface ResolveAssetInput {
  chain: string;
  contractAddress?: string;
  symbol?: string;
}

export interface GateResponse {
  gateToken: string;
}

export interface ConsentDecision {
  purpose: string;
  granted: boolean;
  legalBasis: string;
}

export interface ConsentRecord {
  purpose: string;
  granted: boolean;
  legalBasis: string;
  recordedAt: string;
}

export interface Preferences {
  baseCurrency: string;
  locale: "en" | "vi";
  timezone: string;
}

export interface Profile {
  userId: string;
  email?: string;
  displayName?: string;
  role?: "user" | "admin" | "support";
  status?: "pending" | "active" | "deleted";
  mfaRequired: boolean;
  mfaEnrolled: boolean;
  preferences?: Preferences;
}

export interface SessionInfo {
  id: string;
  device: string;
  location: string;
  lastActiveAt: string;
  current: boolean;
}

export interface RecoveryCodes {
  codes: string[];
}

export interface Account {
  id: string;
  kind: "exchange" | "wallet" | "walletconnect" | "csv";
  label: string;
  venue?: string;
  status: "active" | "syncing" | "synced" | "failed" | "error" | "disconnected";
  lastSyncedAt?: string | null;
  lastError?: string | null;
}

export interface AddExchangeAccountInput {
  venue: "binance" | "okx";
  label: string;
  apiKey: string;
  apiSecret: string;
  passphrase?: string;
}

export interface AddWalletInput {
  venue: string;
  label: string;
  address: string;
}

export interface AddWalletConnectInput {
  venue: string;
  label: string;
  address: string;
  topic: string;
}

export interface ImportRowError {
  line: number;
  reason: string;
}

export interface ImportBatch {
  id: string;
  status: "pending" | "processing" | "complete" | "failed";
  accepted: number;
  skipped: number;
  errors: ImportRowError[];
}

export interface Portfolio {
  id: string;
  name: string;
  baseCurrency: string;
}

export type DataStatus = "complete" | "partial" | "unavailable";
export type LineDataStatus = "priced" | "unpriced" | "holdings_unavailable";
export type Freshness = "live" | "delayed" | "stale" | "unpriced";

export interface ValuationLine {
  assetId: string;
  symbol?: string;
  quantity: string;
  value: string | null;
  allocation: number | null;
  targetPercent: number | null;
  driftPoints: number | null;
  guardrailPoints?: number | null;
  price: string | null;
  priceSource: string | null;
  priceFreshness: Freshness;
  priceObservedAt: string | null;
  fxRate: string | null;
  fxSource: string | null;
  fxDerivedFrom?: string[];
  fxObservedAt: string | null;
  dataStatus: LineDataStatus;
}

export interface PortfolioValuation {
  portfolioId: string;
  asOf: string;
  status: DataStatus;
  holdingsUnavailable?: boolean;
  totalValue: string | null;
  currency: string;
  pricedCoverage: number;
  unpricedCount: number;
  excludedCount?: number;
  lines: ValuationLine[];
}

export interface TargetInput {
  assetId: string;
  targetPercent: number;
  guardrailPoints?: number;
}

export interface TargetSetPreview {
  sum: number;
  difference: number;
}

export interface OrderIntentRequest {
  assetId: string;
  side: "buy" | "sell";
  quantity: string;
}

export interface OrderIntent {
  id: string;
  assetId: string;
  side: "buy" | "sell";
  quantity: string;
  projectedAllocation: number | null;
  projectedDriftPoints: number | null;
  shortfall: string | null;
  status: "pending" | "cancelled";
  cancelledAt: string | null;
}

export interface TradeRequest {
  assetId: string;
  side: "buy" | "sell";
  quantity: string;
  price: string;
  priceSource: "venue" | "user";
  executedAt: string;
  idempotencyKey: string;
}

export interface Trade {
  id: string;
  assetId: string;
  side: "buy" | "sell";
  quantity: string;
  price: string;
  priceSource: "venue" | "user";
  executedAt: string;
  idempotencyKey?: string;
}

export interface Snapshot {
  id: string;
  asOf: string;
  totalValue: string | null;
  currency: string;
  dataStatus?: DataStatus;
  verified?: boolean;
  lines?: ValuationLine[];
}

export interface Transaction {
  id: string;
  assetId: string;
  symbol?: string;
  kind: "buy" | "sell" | "transfer_in" | "transfer_out" | "fee" | "correction";
  quantity: string;
  price: string | null;
  priceMissing?: boolean;
  currency?: string;
  source: string;
  executedAt: string;
  corrected?: boolean;
}

export interface PnlAsset {
  assetId: string;
  symbol?: string;
  covered: boolean;
  markPrice: string | null;
  priceAsOf: string | null;
  priceFreshness: Freshness;
}

export interface Pnl {
  asOf: string;
  netInvested: string | null;
  realized: string | null;
  unrealized: string | null;
  coverage: number;
  coverageLabel?: DataStatus;
  coverageGap?: string[];
  currency: string;
  fxAsOf: string | null;
  fxEstimated: boolean;
  marksUnavailable: boolean;
  holdingsUnavailable: boolean;
  fxUnavailable: boolean;
  assets?: PnlAsset[];
}

export interface CorrectionInput {
  quantity?: string;
  price?: string;
  executedAt?: string;
}

export interface BriefSection {
  title: string;
  kind: "valuation" | "drift" | "pnl" | "text";
  lines?: ValuationLine[];
  text?: string;
}

export interface Brief {
  id: string;
  period: "daily" | "weekly";
  periodStart: string;
  periodEnd: string;
  status: "pending" | "generated" | "failed";
  failureReason: string | null;
  asOf: string | null;
  dataStatus: DataStatus;
  sections: BriefSection[];
  generatedAt: string | null;
}

export interface NotificationItem {
  id: string;
  type: string;
  title: string;
  body: string;
  createdAt: string;
  read: boolean;
}

export interface NotificationPreference {
  type: string;
  inApp: boolean;
  email: boolean;
  mandatory?: boolean;
}

export interface NotificationPreferences {
  types: NotificationPreference[];
}

export interface ExportJob {
  id: string;
  status: "queued" | "running" | "complete" | "failed";
  progress: number;
  downloadUrl: string | null;
  expiresAt: string | null;
}

export interface DeletionRequest {
  id: string;
  status: "requested" | "in_progress" | "complete" | "failed";
  requestedAt: string;
}

export interface SupportArticle {
  id: string;
  title: string;
  summary: string;
  body: string;
}

export interface Incident {
  id: string;
  title: string;
  severity: "sev1" | "sev2" | "sev3";
  status: "declared" | "investigating" | "resolved";
  declaredAt: string;
}

export interface SupportRequestInput {
  subject: string;
  body: string;
}

export interface ClientErrorEvent {
  route: string;
  errorCode: string;
  correlationId: string;
  sessionId?: string;
}

// ── Signup gates (U8) ──────────────────────────────────────────────────────
// Transcribed from contract-summary.md C1 (Principal) and functional-spec §6
// (G2/G3). Additive only — no existing type changes.

export type PrincipalRole = "owner" | "admin" | "support";

export type PrincipalStatus = "pending" | "active" | "disabled" | "pending_deletion" | "deleted";

export type PrincipalNextStep = "none" | "verify_email" | "enrol_mfa" | "restore_account";

/** C1: the self-scoped, read-only principal answer (functional-spec §6 G1). */
export interface Principal {
  userId: string;
  role: PrincipalRole;
  status: PrincipalStatus;
  displayName?: string;
  emailVerified?: boolean;
  nextStep: PrincipalNextStep;
}

/** G2: what is still outstanding, and whether continued use is blocked. */
export interface SignupGateState {
  missing: Array<"terms" | "risk" | "age">;
  jurisdictionBlocked: boolean;
}

/** G3 request body — exactly four fields (functional-spec §3). */
export interface SignupGatesPayload {
  countryCode: string;
  terms: boolean;
  risk: boolean;
  age: boolean;
}

// ── Settings (U7) ──────────────────────────────────────────────────────────
// Transcribed from functional-spec §8 (D0/P1/P2/D1/D2/D3 and X2–X6). Additive
// only — no existing type changes.

/** The stored lifecycle state (identity_domain::UserStatus). */
export type UserStatus = "pending" | "active" | "disabled" | "pending_deletion" | "deleted";

/** D0 `GET /v1/me/account-status`: the caller's own status and nothing else. */
export interface AccountStatusView {
  status: UserStatus;
}

/** P1 `GET /v1/me/profile` and the P2 response. */
export interface ProfileView {
  userId: string;
  displayName: string | null;
  displayNameOverridden: boolean;
  providerDisplayName: string | null;
  email: string | null;
  emailVerified: boolean;
  status: UserStatus;
}

/** D1/D2/D3 `DeletionResponse` (contract-summary C3). */
export interface DeletionView {
  deletionId: string;
  status: string;
  requestedAt: string;
  completedAt?: string | null;
  failureReason?: string | null;
  retainedCategories: Array<{ category: string; basis: string }>;
  recoveryEndsAt: string;
  cancelledAt?: string | null;
  erasedAt?: string | null;
  attemptCount: number;
}

/** One effective preference value and where it came from. */
export interface PreferenceValue<T> {
  value: T;
  source: string;
}

/** X2 `GET|PUT /v1/me/preferences` response. */
export interface EffectivePreferences {
  userId: string;
  locale: PreferenceValue<string>;
  baseCurrency: PreferenceValue<string>;
  timezone: PreferenceValue<string>;
  notifications: {
    email: PreferenceValue<boolean>;
    push: PreferenceValue<boolean>;
  };
}

/** X2 request body: only the fields the person changed. */
export interface PreferencesPatch {
  locale?: string;
  baseCurrency?: string;
  timezone?: string;
  notifications?: { email?: boolean; push?: boolean };
}

/** X3 `GET /v1/me/consent/state`. */
export interface ConsentStateView {
  userId: string;
  kinds: Record<string, string>;
  missing: string[];
}

/** One X4 consent event. */
export interface ConsentEvent {
  id: string;
  seq: number;
  kind: string;
  action: string;
  occurredAt: string;
  policyVersion: string;
}

/** X4 `GET /v1/me/consent` page. */
export interface ConsentHistoryView {
  events: ConsentEvent[];
  nextCursor?: string | null;
}

/** X5/X6 decision answer. Named apart from the legacy onboarding `ConsentDecision`. */
export interface ConsentDecisionView {
  kind: string;
  state: string;
  changed?: boolean;
}
