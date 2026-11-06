/**
 * The one module that owns every API Gateway call.
 *
 * Components render; they never build URLs or call fetch themselves. Business
 * math is never computed here or in any component.
 */

export interface Asset {
  id: string;
  symbol: string;
  name: string;
  decimals: number;
  canonical: boolean;
}

export interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail?: string;
  correlationId: string;
}

export interface ResolveAssetInput {
  chain: string;
  contractAddress?: string;
  symbol?: string;
}

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

function newCorrelationId(): string {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi && typeof cryptoApi.randomUUID === "function") {
    return cryptoApi.randomUUID();
  }
  return `web-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

/**
 * Resolves a chain observation to a canonical asset.
 *
 * @throws {ApiError} when the gateway returns a problem response.
 * @throws {NetworkError} when the gateway is unreachable.
 */
export async function resolveAsset(
  input: ResolveAssetInput,
  token: string | null,
  fetchImpl: typeof fetch = fetch,
): Promise<Asset> {
  const params = new URLSearchParams({ chain: input.chain });
  if (input.contractAddress?.trim()) {
    params.set("contractAddress", input.contractAddress.trim());
  }
  if (input.symbol?.trim()) {
    params.set("symbol", input.symbol.trim());
  }

  const headers: Record<string, string> = {
    "x-correlation-id": newCorrelationId(),
  };
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  let response: Response;
  try {
    response = await fetchImpl(`${GATEWAY_URL}/assets/resolve?${params.toString()}`, { headers });
  } catch (error) {
    throw new NetworkError(error instanceof Error ? error.message : "network failure");
  }

  if (!response.ok) {
    const problem = (await response.json().catch(() => null)) as ProblemDetails | null;
    throw new ApiError(
      problem ?? {
        type: "about:blank",
        title: `Gateway returned ${response.status}`,
        status: response.status,
        correlationId: headers["x-correlation-id"],
      },
    );
  }

  return (await response.json()) as Asset;
}
