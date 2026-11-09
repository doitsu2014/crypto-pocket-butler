/**
 * The one module that owns every C2 support-API call.
 *
 * Components render; they never build URLs or call fetch themselves. Every
 * call carries the in-memory session's bearer token. There is deliberately no
 * audit write path here: the support API owns C10 (BR2.1) and the console is
 * UI-only.
 */

import {
  pickAccountMetadata,
  pickAccountSearchResult,
  pickIncident,
  pickSupportGrant,
  pickTicketList,
} from "../domain/metadata";
import type {
  AccountMetadata,
  AccountSearchInput,
  AccountSearchResult,
  GrantRequestInput,
  Incident,
  IncidentInput,
  IncidentPatch,
  MetadataReadInput,
  ProblemDetails,
  SupportAccessGrant,
  TicketList,
} from "./types";

/** A non-success gateway response, carrying the RFC 7807 problem. */
export class ApiError extends Error {
  readonly name = "ApiError";

  constructor(readonly problem: ProblemDetails) {
    super(problem.title);
  }

  get status(): number {
    return this.problem.status;
  }
}

/** The gateway could not be reached at all. */
export class NetworkError extends Error {
  readonly name = "NetworkError";

  constructor(message: string) {
    super(message);
  }
}

/**
 * BR1.3: account search is grant-scoped. A missing grant id is refused in the
 * client as well as by the support API.
 */
export class GrantScopeError extends Error {
  readonly name = "GrantScopeError";

  constructor() {
    super("An active support grant is required to search or read account metadata.");
  }
}

function newCorrelationId(): string {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi && typeof cryptoApi.randomUUID === "function") {
    return cryptoApi.randomUUID();
  }
  return `support-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function gatewayBase(): string {
  const configured = import.meta.env.VITE_GATEWAY_URL;
  return (configured?.trim() || "/v1").replace(/\/$/, "");
}

function absoluteUrl(path: string, params?: URLSearchParams): string {
  const relative = `${gatewayBase()}${path}${params ? `?${params.toString()}` : ""}`;
  const origin = globalThis.location?.origin ?? "http://localhost";
  return new URL(relative, origin).toString();
}

async function parseProblem(
  response: Response,
  fallbackCorrelationId: string,
): Promise<ProblemDetails> {
  const body = (await response.json().catch(() => null)) as Partial<ProblemDetails> | null;
  if (
    body &&
    typeof body.type === "string" &&
    typeof body.title === "string" &&
    typeof body.status === "number"
  ) {
    return {
      type: body.type,
      title: body.title,
      status: body.status,
      detail: typeof body.detail === "string" ? body.detail : undefined,
      correlationId:
        typeof body.correlationId === "string" ? body.correlationId : fallbackCorrelationId,
    };
  }
  return {
    type: "about:blank",
    title: `Gateway returned ${response.status}`,
    status: response.status,
    correlationId: fallbackCorrelationId,
  };
}

interface RequestOptions {
  method?: "GET" | "POST" | "PATCH" | "DELETE";
  token: string | null;
  body?: unknown;
  params?: URLSearchParams;
  headers?: Record<string, string>;
  fetchImpl?: typeof fetch;
}

async function request<T>(path: string, options: RequestOptions): Promise<T> {
  const headers: Record<string, string> = {
    Accept: "application/json",
    "x-correlation-id": newCorrelationId(),
    ...options.headers,
  };
  if (options.body !== undefined) {
    headers["Content-Type"] = "application/json";
  }
  if (options.token) {
    headers.Authorization = `Bearer ${options.token}`;
  }

  const fetchImpl = options.fetchImpl ?? fetch;
  let response: Response;
  try {
    response = await fetchImpl(absoluteUrl(path, options.params), {
      method: options.method ?? "GET",
      headers,
      body: options.body === undefined ? undefined : JSON.stringify(options.body),
    });
  } catch (error) {
    throw new NetworkError(error instanceof Error ? error.message : "network failure");
  }

  if (!response.ok) {
    throw new ApiError(await parseProblem(response, headers["x-correlation-id"]));
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

export async function requestSupportGrant(
  input: GrantRequestInput,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<SupportAccessGrant> {
  const raw = await request<unknown>("/support-grants", {
    method: "POST",
    token,
    body: input,
    fetchImpl,
  });
  return pickSupportGrant(raw);
}

export async function getSupportGrant(
  grantId: string,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<SupportAccessGrant> {
  const raw = await request<unknown>(`/support-grants/${encodeURIComponent(grantId)}`, {
    token,
    fetchImpl,
  });
  return pickSupportGrant(raw);
}

export async function approveSupportGrant(
  grantId: string,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<SupportAccessGrant> {
  const raw = await request<unknown>(
    `/support-grants/${encodeURIComponent(grantId)}/approve`,
    { method: "POST", token, fetchImpl },
  );
  return pickSupportGrant(raw);
}

export async function revokeSupportGrant(
  grantId: string,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<void> {
  await request(`/support-grants/${encodeURIComponent(grantId)}`, {
    method: "DELETE",
    token,
    fetchImpl,
  });
}

export async function searchAccounts(
  input: AccountSearchInput,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<AccountSearchResult> {
  const grantId = input.grantId?.trim();
  if (!grantId) {
    throw new GrantScopeError();
  }
  const params = new URLSearchParams({ grantId, query: input.query?.trim() ?? "" });
  const raw = await request<unknown>("/support/accounts", { token, params, fetchImpl });
  return pickAccountSearchResult(raw);
}

export async function fetchAccountMetadata(
  accountId: string,
  input: MetadataReadInput,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<AccountMetadata> {
  const grantId = input.grantId?.trim();
  if (!grantId) {
    throw new GrantScopeError();
  }
  // The free-text reason is a header, never a query parameter: it must not
  // reach access logs, browser history, or referrers. The support API hashes
  // it for the C10 audit event.
  const params = new URLSearchParams({ grantId });
  const raw = await request<unknown>(
    `/support/accounts/${encodeURIComponent(accountId)}/metadata`,
    {
      token,
      params,
      headers: { "X-Support-Reason": input.reason?.trim() ?? "" },
      fetchImpl,
    },
  );
  return pickAccountMetadata(raw);
}

export async function listTickets(
  grantId: string,
  status: string | undefined,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<TicketList> {
  const scopedGrantId = grantId?.trim();
  if (!scopedGrantId) {
    throw new GrantScopeError();
  }
  const params = new URLSearchParams({ grantId: scopedGrantId });
  if (status?.trim()) {
    params.set("status", status.trim());
  }
  const raw = await request<unknown>("/support/tickets", { token, params, fetchImpl });
  return pickTicketList(raw);
}

export async function declareIncident(
  input: IncidentInput,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<Incident> {
  const raw = await request<unknown>("/support/incidents", {
    method: "POST",
    token,
    body: input,
    fetchImpl,
  });
  return pickIncident(raw);
}

export async function updateIncident(
  incidentId: string,
  patch: IncidentPatch,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<Incident> {
  const raw = await request<unknown>(`/support/incidents/${encodeURIComponent(incidentId)}`, {
    method: "PATCH",
    token,
    body: patch,
    fetchImpl,
  });
  return pickIncident(raw);
}
