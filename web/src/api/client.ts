import { getAccessToken } from "../auth/session";
import type {
  Account,
  AddExchangeAccountInput,
  AddWalletConnectInput,
  AddWalletInput,
  Asset,
  Brief,
  ClientErrorEvent,
  ConsentDecision,
  ConsentRecord,
  CorrectionInput,
  DeletionRequest,
  ExportJob,
  GateResponse,
  ImportBatch,
  Incident,
  NotificationItem,
  NotificationPreferences,
  OrderIntent,
  OrderIntentRequest,
  Pnl,
  Portfolio,
  PortfolioValuation,
  Preferences,
  ProblemDetails,
  Profile,
  RecoveryCodes,
  ResolveAssetInput,
  SessionInfo,
  Snapshot,
  SupportArticle,
  SupportRequestInput,
  TargetInput,
  TargetSetPreview,
  Trade,
  TradeRequest,
  Transaction,
} from "./types";

export type {
  Account,
  AddExchangeAccountInput,
  AddWalletConnectInput,
  AddWalletInput,
  Asset,
  Brief,
  ClientErrorEvent,
  ConsentDecision,
  ConsentRecord,
  CorrectionInput,
  DeletionRequest,
  ExportJob,
  GateResponse,
  ImportBatch,
  Incident,
  NotificationItem,
  NotificationPreferences,
  OrderIntent,
  OrderIntentRequest,
  Pnl,
  Portfolio,
  PortfolioValuation,
  Preferences,
  ProblemDetails,
  Profile,
  RecoveryCodes,
  ResolveAssetInput,
  SessionInfo,
  Snapshot,
  SupportArticle,
  SupportRequestInput,
  TargetInput,
  TargetSetPreview,
  Trade,
  TradeRequest,
  Transaction,
} from "./types";

/** A non-success gateway response, carrying the RFC 7807 problem. */
export class ApiError extends Error {
  readonly name = "ApiError";

  constructor(readonly problem: ProblemDetails) {
    super(problem.title);
  }
}

/** The gateway could not be reached at all. */
export class NetworkError extends Error {
  readonly name = "NetworkError";

  constructor(message: string) {
    super(message);
  }
}

const GATEWAY_URL = (import.meta.env.VITE_GATEWAY_URL as string | undefined) ?? "/v1";

/** The default page size for every list call (NFR2.5). */
export const PAGE_SIZE = 50;

type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

type QueryValue = string | number | boolean | null | undefined;

export interface RequestOptions {
  method?: HttpMethod;
  body?: unknown;
  query?: Record<string, QueryValue>;
  /** Defaults to true; pass false only for the public telemetry and gate routes. */
  auth?: boolean;
  signal?: AbortSignal;
  fetchImpl?: typeof fetch;
}

export interface ClientConfig {
  baseUrl?: string;
  getToken?: () => string | null;
  fetchImpl?: typeof fetch;
}

function newCorrelationId(): string {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi && typeof cryptoApi.randomUUID === "function") {
    return cryptoApi.randomUUID();
  }
  return `web-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function buildUrl(baseUrl: string, path: string, query?: Record<string, QueryValue>): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value !== undefined && value !== null && value !== "") {
      params.set(key, String(value));
    }
  }
  const suffix = params.toString();
  return `${baseUrl}${path}${suffix ? `?${suffix}` : ""}`;
}

async function toProblem(response: Response, correlationId: string): Promise<ProblemDetails> {
  try {
    const body = (await response.json()) as Partial<ProblemDetails>;
    return {
      type: body.type ?? "about:blank",
      title: body.title ?? `Gateway returned ${response.status}`,
      status: body.status ?? response.status,
      detail: body.detail,
      correlationId: body.correlationId ?? correlationId,
      errors: body.errors,
    };
  } catch {
    return {
      type: "about:blank",
      title: `Gateway returned ${response.status}`,
      status: response.status,
      correlationId,
    };
  }
}

/**
 * The one module that owns every API Gateway call (BR1.2).
 *
 * Components receive data through query hooks; they never build URLs or call
 * fetch. Business math is never computed here or in any component.
 */
export class GatewayClient {
  private readonly baseUrl: string;
  private readonly token: () => string | null;
  private readonly fetchImpl?: typeof fetch;

  constructor(config: ClientConfig = {}) {
    this.baseUrl = config.baseUrl ?? GATEWAY_URL;
    this.token = config.getToken ?? getAccessToken;
    // Resolved per request rather than captured here, so a test that replaces
    // the global fetch after this client was constructed still takes effect.
    this.fetchImpl = config.fetchImpl;
  }

  /** The base URL the client is bound to (used by the origin test). */
  get base(): string {
    return this.baseUrl;
  }

  private async request<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const correlationId = newCorrelationId();
    const headers: Record<string, string> = {
      Accept: "application/json",
      "x-correlation-id": correlationId,
    };
    if (options.auth !== false) {
      const token = this.token();
      if (token) {
        headers.Authorization = `Bearer ${token}`;
      }
    }
    const isFormData = typeof FormData !== "undefined" && options.body instanceof FormData;
    if (options.body !== undefined && !isFormData) {
      headers["Content-Type"] = "application/json";
    }

    let response: Response;
    try {
      const requestInit: RequestInit = {
        method: options.method ?? "GET",
        headers,
        body:
          options.body === undefined
            ? undefined
            : isFormData
              ? (options.body as FormData)
              : JSON.stringify(options.body),
        signal: options.signal,
      };
      const url = buildUrl(this.baseUrl, path, options.query);
      const doFetch = options.fetchImpl ?? this.fetchImpl;
      response = doFetch
        ? await doFetch(url, requestInit)
        : await globalThis.fetch(url, requestInit);
    } catch (error) {
      throw new NetworkError(error instanceof Error ? error.message : "network failure");
    }

    if (!response.ok) {
      throw new ApiError(await toProblem(response, correlationId));
    }

    if (response.status === 204) {
      return undefined as T;
    }
    return (await response.json()) as T;
  }

  // ── Assets (U1) ──────────────────────────────────────────────────────────

  resolveAsset(input: ResolveAssetInput): Promise<Asset> {
    return this.request<Asset>("/assets/resolve", {
      query: {
        chain: input.chain,
        contractAddress: input.contractAddress,
        symbol: input.symbol,
      },
    });
  }

  // ── Auth and onboarding (U3) ─────────────────────────────────────────────

  createSignupGate(input: {
    email: string;
    jurisdiction: string;
    ageConfirmed: boolean;
  }): Promise<GateResponse> {
    return this.request<GateResponse>("/signup-gate", {
      method: "POST",
      body: input,
      auth: false,
    });
  }

  resendVerification(email: string): Promise<void> {
    return this.request<void>("/auth/resend-verification", {
      method: "POST",
      body: { email },
      auth: false,
    });
  }

  getConsent(): Promise<ConsentRecord[]> {
    return this.request<ConsentRecord[]>("/consent");
  }

  postConsent(decisions: ConsentDecision[]): Promise<void> {
    return this.request<void>("/consent", { method: "POST", body: { decisions } });
  }

  getPreferences(): Promise<Preferences> {
    return this.request<Preferences>("/preferences");
  }

  updatePreferences(preferences: Preferences): Promise<Preferences> {
    return this.request<Preferences>("/preferences", {
      method: "PUT",
      body: preferences,
    });
  }

  getProfile(): Promise<Profile> {
    return this.request<Profile>("/users/me");
  }

  updateProfile(patch: Partial<Pick<Profile, "displayName">>): Promise<Profile> {
    return this.request<Profile>("/users/me", { method: "PATCH", body: patch });
  }

  listSessions(): Promise<SessionInfo[]> {
    return this.request<SessionInfo[]>("/sessions");
  }

  revokeSession(sessionId: string): Promise<void> {
    return this.request<void>(`/sessions/${encodeURIComponent(sessionId)}`, {
      method: "DELETE",
    });
  }

  enrolMfa(): Promise<{ secret: string; otpauthUrl: string }> {
    return this.request<{ secret: string; otpauthUrl: string }>("/mfa/enrol", {
      method: "POST",
    });
  }

  recoveryCodes(): Promise<RecoveryCodes> {
    return this.request<RecoveryCodes>("/mfa/recovery", { method: "POST" });
  }

  // ── Accounts and imports (U4) ────────────────────────────────────────────

  addExchangeAccount(input: AddExchangeAccountInput): Promise<Account> {
    return this.request<Account>("/accounts", {
      method: "POST",
      body: {
        kind: "exchange",
        venue: input.venue,
        label: input.label,
        apiKey: input.apiKey,
        apiSecret: input.apiSecret,
        passphrase: input.passphrase,
      },
    });
  }

  addWallet(input: AddWalletInput): Promise<Account> {
    return this.request<Account>("/accounts", {
      method: "POST",
      body: { kind: "wallet", venue: input.venue, label: input.label, address: input.address },
    });
  }

  addWalletConnect(input: AddWalletConnectInput): Promise<Account> {
    return this.request<Account>("/accounts", {
      method: "POST",
      body: {
        kind: "walletconnect",
        venue: input.venue,
        label: input.label,
        address: input.address,
        topic: input.topic,
      },
    });
  }

  verifyAccount(accountId: string): Promise<{ status: Account["status"] }> {
    return this.request<{ status: Account["status"] }>(
      `/accounts/${encodeURIComponent(accountId)}/verify`,
      { method: "POST" },
    );
  }

  listAccounts(userId: string): Promise<Account[]> {
    return this.request<Account[]>(`/users/${encodeURIComponent(userId)}/accounts`);
  }

  resyncAccount(accountId: string): Promise<{ runId: string }> {
    return this.request<{ runId: string }>(`/accounts/${encodeURIComponent(accountId)}/resync`, {
      method: "POST",
    });
  }

  createImport(file: File): Promise<ImportBatch> {
    const body = new FormData();
    body.append("file", file);
    return this.request<ImportBatch>("/imports", {
      method: "POST",
      body,
      // fetch sets the multipart boundary; do not set Content-Type.
    });
  }

  getImport(importId: string): Promise<ImportBatch> {
    return this.request<ImportBatch>(`/imports/${encodeURIComponent(importId)}`);
  }

  // ── Portfolios (U6) ──────────────────────────────────────────────────────

  listPortfolios(): Promise<Portfolio[]> {
    return this.request<Portfolio[]>("/portfolios");
  }

  getPortfolio(portfolioId: string): Promise<PortfolioValuation> {
    return this.request<PortfolioValuation>(`/portfolios/${encodeURIComponent(portfolioId)}`);
  }

  createPortfolio(input: { name: string; baseCurrency: string }): Promise<Portfolio> {
    return this.request<Portfolio>("/portfolios", { method: "POST", body: input });
  }

  renamePortfolio(portfolioId: string, name: string): Promise<Portfolio> {
    return this.request<Portfolio>(`/portfolios/${encodeURIComponent(portfolioId)}`, {
      method: "PATCH",
      body: { name },
    });
  }

  assignAccount(portfolioId: string, accountId: string): Promise<void> {
    return this.request<void>(
      `/portfolios/${encodeURIComponent(portfolioId)}/accounts/${encodeURIComponent(accountId)}`,
      { method: "PUT" },
    );
  }

  removeAccount(portfolioId: string, accountId: string): Promise<void> {
    return this.request<void>(
      `/portfolios/${encodeURIComponent(portfolioId)}/accounts/${encodeURIComponent(accountId)}`,
      { method: "DELETE" },
    );
  }

  previewTargets(portfolioId: string, targets: TargetInput[]): Promise<TargetSetPreview> {
    return this.request<TargetSetPreview>(
      `/portfolios/${encodeURIComponent(portfolioId)}/targets`,
      { method: "PUT", query: { preview: true }, body: { targets } },
    );
  }

  saveTargets(portfolioId: string, targets: TargetInput[]): Promise<void> {
    return this.request<void>(`/portfolios/${encodeURIComponent(portfolioId)}/targets`, {
      method: "PUT",
      body: { targets },
    });
  }

  previewIntent(portfolioId: string, intent: OrderIntentRequest): Promise<OrderIntent> {
    return this.request<OrderIntent>(
      `/portfolios/${encodeURIComponent(portfolioId)}/order-intents`,
      { method: "POST", query: { preview: true }, body: intent },
    );
  }

  createIntent(portfolioId: string, intent: OrderIntentRequest): Promise<OrderIntent> {
    return this.request<OrderIntent>(
      `/portfolios/${encodeURIComponent(portfolioId)}/order-intents`,
      { method: "POST", body: intent },
    );
  }

  cancelIntent(portfolioId: string, intentId: string): Promise<OrderIntent> {
    return this.request<OrderIntent>(
      `/portfolios/${encodeURIComponent(portfolioId)}/order-intents/${encodeURIComponent(intentId)}/cancel`,
      { method: "POST" },
    );
  }

  recordTrade(portfolioId: string, trade: TradeRequest): Promise<Trade> {
    return this.request<Trade>(`/portfolios/${encodeURIComponent(portfolioId)}/trades`, {
      method: "POST",
      body: trade,
    });
  }

  listSnapshots(portfolioId: string, cursor?: string): Promise<Snapshot[]> {
    return this.request<Snapshot[]>(`/portfolios/${encodeURIComponent(portfolioId)}/snapshots`, {
      query: { limit: PAGE_SIZE, cursor },
    });
  }

  // ── Activity and P&L (U7) ────────────────────────────────────────────────

  listTransactions(userId: string, cursor?: string): Promise<Transaction[]> {
    return this.request<Transaction[]>(`/users/${encodeURIComponent(userId)}/transactions`, {
      query: { limit: PAGE_SIZE, cursor },
    });
  }

  getPnl(userId: string): Promise<Pnl> {
    return this.request<Pnl>(`/users/${encodeURIComponent(userId)}/pnl`);
  }

  correctTransaction(transactionId: string, correction: CorrectionInput): Promise<Transaction> {
    return this.request<Transaction>(
      `/transactions/${encodeURIComponent(transactionId)}/corrections`,
      { method: "POST", body: correction },
    );
  }

  // ── Briefs (U8) ──────────────────────────────────────────────────────────

  listBriefs(userId: string, period?: "daily" | "weekly", cursor?: string): Promise<Brief[]> {
    return this.request<Brief[]>(`/users/${encodeURIComponent(userId)}/briefs`, {
      query: { period, cursor, limit: PAGE_SIZE },
    });
  }

  getBrief(userId: string, briefId: string): Promise<Brief> {
    return this.request<Brief>(
      `/users/${encodeURIComponent(userId)}/briefs/${encodeURIComponent(briefId)}`,
    );
  }

  // ── Notifications (U9) ───────────────────────────────────────────────────

  listNotifications(userId: string, cursor?: string): Promise<NotificationItem[]> {
    return this.request<NotificationItem[]>(`/users/${encodeURIComponent(userId)}/notifications`, {
      query: { cursor, limit: PAGE_SIZE },
    });
  }

  getNotificationPreferences(userId: string): Promise<NotificationPreferences> {
    return this.request<NotificationPreferences>(
      `/users/${encodeURIComponent(userId)}/notifications/preferences`,
    );
  }

  updateNotificationPreferences(
    userId: string,
    preferences: NotificationPreferences,
  ): Promise<NotificationPreferences> {
    return this.request<NotificationPreferences>(
      `/users/${encodeURIComponent(userId)}/notifications/preferences`,
      { method: "PUT", body: preferences },
    );
  }

  // ── Privacy (U3) ─────────────────────────────────────────────────────────

  startExport(): Promise<ExportJob> {
    return this.request<ExportJob>("/exports", { method: "POST" });
  }

  getExport(exportId: string): Promise<ExportJob> {
    return this.request<ExportJob>(`/exports/${encodeURIComponent(exportId)}`);
  }

  requestDeletion(): Promise<DeletionRequest> {
    return this.request<DeletionRequest>("/privacy/deletions", { method: "POST" });
  }

  // ── Help and status (U1/U3) ──────────────────────────────────────────────

  listArticles(): Promise<SupportArticle[]> {
    return this.request<SupportArticle[]>("/support/articles");
  }

  createSupportRequest(input: SupportRequestInput): Promise<{ id: string }> {
    return this.request<{ id: string }>("/support/requests", {
      method: "POST",
      body: input,
    });
  }

  listIncidents(): Promise<Incident[]> {
    return this.request<Incident[]>("/support/incidents", {
      query: { active: true },
      auth: false,
    });
  }

  // ── Telemetry (public) ───────────────────────────────────────────────────

  postClientError(event: ClientErrorEvent): Promise<void> {
    return this.request<void>("/telemetry/client-errors", {
      method: "POST",
      body: event,
      auth: false,
    });
  }
}

/** The client the app uses; reads the in-memory session token per request. */
export const gatewayClient = new GatewayClient();

/** Compatibility helper used by the asset-resolver surface and its tests. */
export function resolveAsset(
  input: ResolveAssetInput,
  token: string | null,
  fetchImpl: typeof fetch = fetch,
): Promise<Asset> {
  return new GatewayClient({ getToken: () => token, fetchImpl }).resolveAsset(input);
}

/** True when the error is a gateway 401. */
export function isUnauthorized(error: unknown): boolean {
  return error instanceof ApiError && error.problem.status === 401;
}

/** True when the error is a gateway 403. */
export function isForbidden(error: unknown): boolean {
  return error instanceof ApiError && error.problem.status === 403;
}

/** True when the error is a gateway 409. */
export function isConflict(error: unknown): boolean {
  return error instanceof ApiError && error.problem.status === 409;
}

/** True when the error is a recoverable server failure (5xx). */
export function isServerFailure(error: unknown): boolean {
  return error instanceof ApiError && error.problem.status >= 500;
}

/** True when the gateway does not expose the route yet (deny-by-default 404 or 501). */
export function isUnimplemented(error: unknown): boolean {
  return (
    error instanceof ApiError && (error.problem.status === 404 || error.problem.status === 501)
  );
}
