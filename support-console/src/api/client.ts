/**
 * The one module that owns every C2 support-API call.
 *
 * Components render; they never build URLs or call fetch themselves. Every
 * call carries the in-memory session's bearer token. There is deliberately no
 * audit write path here: the support API owns C10 (BR2.1) and the console is
 * UI-only.
 */

import {
  pickIncident,
  pickSupportGrant,
  pickSupportMetadata,
  pickTicketList,
} from "../domain/metadata";
import type {
  AdminStatusBody,
  AdminStatusResult,
  AdminUserRecord,
  GrantRequestInput,
  Incident,
  IncidentInput,
  IncidentPatch,
  ProblemDetails,
  RoleChangeBody,
  RoleChangeResult,
  SupportAccessGrant,
  SupportMetadata,
  TicketList,
} from "./types";

/** A non-success gateway response, carrying the RFC 7807 problem. */
export class ApiError extends Error {
  readonly name = "ApiError";

  /** The `Retry-After` header in whole seconds, when the gateway sent one. */
  readonly retryAfterSeconds?: number;

  constructor(
    readonly problem: ProblemDetails,
    retryAfterSeconds?: number,
  ) {
    super(problem.title);
    this.retryAfterSeconds = retryAfterSeconds;
  }

  get status(): number {
    return this.problem.status;
  }
}

/** A call that did not answer within its own deadline (NFR2.2). */
export class DeadlineError extends Error {
  readonly name = "DeadlineError";

  constructor() {
    super("The request did not answer within its deadline.");
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
    const errors = Array.isArray(body.errors)
      ? body.errors.filter(
          (entry): entry is { field: string; reason: string } =>
            typeof entry?.field === "string" && typeof entry?.reason === "string",
        )
      : undefined;
    return {
      type: body.type,
      title: body.title,
      status: body.status,
      detail: typeof body.detail === "string" ? body.detail : undefined,
      correlationId:
        typeof body.correlationId === "string" ? body.correlationId : fallbackCorrelationId,
      ...(errors && errors.length > 0 ? { errors } : {}),
    };
  }
  return {
    type: "about:blank",
    title: `Gateway returned ${response.status}`,
    status: response.status,
    correlationId: fallbackCorrelationId,
  };
}

/**
 * Reads `Retry-After` once (performance design §6): integer seconds stay as
 * they are, an HTTP-date becomes whole seconds from now floored at zero, and a
 * missing or unparseable value stays `undefined`.
 */
function parseRetryAfter(response: Response): number | undefined {
  const header = response.headers.get("Retry-After")?.trim();
  if (!header) {
    return undefined;
  }
  if (/^\d+$/.test(header)) {
    return Number(header);
  }
  const when = Date.parse(header);
  if (Number.isNaN(when)) {
    return undefined;
  }
  return Math.max(0, Math.ceil((when - Date.now()) / 1000));
}

interface RequestOptions {
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  token: string | null;
  body?: unknown;
  params?: URLSearchParams;
  headers?: Record<string, string>;
  fetchImpl?: typeof fetch;
  /** Aborts the call when it does not settle in time (NFR2.2). */
  deadlineMs?: number;
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
  const controller = options.deadlineMs === undefined ? undefined : new AbortController();
  const timer =
    controller === undefined || options.deadlineMs === undefined
      ? undefined
      : setTimeout(() => controller.abort(), options.deadlineMs);

  try {
    let response: Response;
    try {
      response = await fetchImpl(absoluteUrl(path, options.params), {
        method: options.method ?? "GET",
        headers,
        body: options.body === undefined ? undefined : JSON.stringify(options.body),
        signal: controller?.signal,
      });
    } catch (error) {
      if (controller?.signal.aborted === true) {
        throw new DeadlineError();
      }
      throw new NetworkError(error instanceof Error ? error.message : "network failure");
    }

    if (!response.ok) {
      const problem = await parseProblem(response, headers["x-correlation-id"]);
      throw new ApiError(problem, parseRetryAfter(response));
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return (await response.json()) as T;
  } finally {
    if (timer !== undefined) {
      clearTimeout(timer);
    }
  }
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

export async function approveSupportGrant(
  grantId: string,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<SupportAccessGrant> {
  const raw = await request<unknown>(`/support-grants/${encodeURIComponent(grantId)}/approve`, {
    method: "POST",
    token,
    fetchImpl,
  });
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

/** S2: the signed-in agent's own grants, newest first (at most twenty). */
export async function listMyGrants(
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<SupportAccessGrant[]> {
  const raw = await request<unknown[]>("/support-grants/mine", {
    token,
    deadlineMs: 15_000,
    fetchImpl,
  });
  return raw.map(pickSupportGrant);
}

/**
 * S3: the closed metadata projection behind one active grant. The grant id is
 * the only scope, so a caller can never widen the read to another user.
 */
export async function fetchSupportMetadata(
  grantId: string,
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<SupportMetadata> {
  const scopedGrantId = grantId?.trim();
  if (!scopedGrantId) {
    throw new GrantScopeError();
  }
  const raw = await request<unknown>(
    `/support-grants/${encodeURIComponent(scopedGrantId)}/metadata`,
    {
      token,
      deadlineMs: 15_000,
      fetchImpl,
    },
  );
  return pickSupportMetadata(raw);
}

/** U10 W1 (D-S1): the exact-email lookup carries its address in the body. */
export async function lookupAdminUser(
  token: string | null,
  email: string,
  fetchImpl?: typeof fetch,
): Promise<AdminUserRecord> {
  return request<AdminUserRecord>("/admin/users/lookup", {
    method: "POST",
    token,
    body: { email },
    deadlineMs: 15_000,
    fetchImpl,
  });
}

/** U10 W2: an administrator changes one user's role. */
export async function changeUserRole(
  token: string | null,
  userId: string,
  body: RoleChangeBody,
  fetchImpl?: typeof fetch,
): Promise<RoleChangeResult> {
  return request<RoleChangeResult>(`/admin/users/${encodeURIComponent(userId)}/role`, {
    method: "PUT",
    token,
    body,
    deadlineMs: 15_000,
    fetchImpl,
  });
}

/** U10 W2: an administrator disables one account. */
export async function disableUser(
  token: string | null,
  userId: string,
  body: AdminStatusBody,
  fetchImpl?: typeof fetch,
): Promise<AdminStatusResult> {
  return request<AdminStatusResult>(`/admin/users/${encodeURIComponent(userId)}/disable`, {
    method: "POST",
    token,
    body,
    deadlineMs: 15_000,
    fetchImpl,
  });
}

/** U10 W3: an administrator restores a disabled account. */
export async function restoreUser(
  token: string | null,
  userId: string,
  body: AdminStatusBody,
  fetchImpl?: typeof fetch,
): Promise<AdminStatusResult> {
  return request<AdminStatusResult>(`/admin/users/${encodeURIComponent(userId)}/restore`, {
    method: "POST",
    token,
    body,
    deadlineMs: 15_000,
    fetchImpl,
  });
}

/** U10 W5: the grants awaiting approval, newest first. */
export async function listPendingGrants(
  token: string | null,
  fetchImpl?: typeof fetch,
): Promise<SupportAccessGrant[]> {
  const params = new URLSearchParams({ status: "pending_approval" });
  const raw = await request<unknown[]>("/support-grants", {
    token,
    params,
    deadlineMs: 15_000,
    fetchImpl,
  });
  return raw.map(pickSupportGrant);
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
