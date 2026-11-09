import { vi } from "vitest";

/**
 * A small MSW-style gateway stub for component tests.
 *
 * Handlers are keyed by "METHOD /path"; a request with no handler receives the
 * gateway's deny-by-default 404 problem, which is how the real gateway answers
 * an operation it does not declare yet.
 */

export interface HandlerContext {
  url: URL;
  init: RequestInit;
  body: unknown;
}

export type Handler = (context: HandlerContext) => {
  status?: number;
  body?: unknown;
} | Response;

function parseBody(body: BodyInit | null | undefined): unknown {
  if (typeof body !== "string") {
    return body ?? null;
  }
  try {
    return JSON.parse(body);
  } catch {
    return body;
  }
}

export function jsonResponse(status: number, body: unknown): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

export function problem(status: number, detail: string): unknown {
  return {
    type: `https://crypto-pocket-butler.example/problems/${status}`,
    title: `Gateway ${status}`,
    status,
    detail,
    correlationId: `corr-${status}`,
  };
}

export function stubGateway(handlers: Record<string, Handler>) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init: RequestInit = {}) => {
    const raw =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    const url = new URL(raw, "https://app.test");
    const method = (init.method ?? "GET").toUpperCase();
    const handler = handlers[`${method} ${url.pathname}`];
    if (!handler) {
      return jsonResponse(404, problem(404, `${method} ${url.pathname} is not declared`));
    }
    const result = handler({ url, init, body: parseBody(init.body) });
    if (result instanceof Object && "ok" in result && "json" in result) {
      return result as Response;
    }
    const plain = result as { status?: number; body?: unknown };
    return jsonResponse(plain.status ?? 200, plain.body);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}
