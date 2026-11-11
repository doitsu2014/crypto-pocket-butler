import { delay, http, HttpResponse } from "msw";
import { afterAll, afterEach, beforeAll, describe, expect, it } from "vitest";
import {
  API,
  gateState,
  hungGet,
  noRecordResponse,
  principalFixture,
  problemBody,
  problemResponse,
} from "../features/signup-gates/test/handlers";
import { server } from "../features/signup-gates/test/server";
import {
  ApiError,
  DeadlineAbort,
  GATE_DEADLINE_MS,
  GatewayClient,
  NetworkError,
  gateFailureVariant,
} from "./client";
import type { SignupGatesPayload } from "./types";

/**
 * Transport additions for the sign-up gates feature (plan step 4;
 * functional-spec §6 G1/G2/G3; security-design §5 allowlist mapping).
 *
 * These are additive to `client.ts`: existing methods and their tests stay
 * untouched. MSW stands in for the gateway, which does not carry these routes
 * yet (plan decision 1) — the client is built against the designed contract.
 */

const payload: SignupGatesPayload = {
  countryCode: "VN",
  terms: true,
  risk: true,
  age: true,
};

function client(deadlineMs?: number) {
  return new GatewayClient({ getToken: () => null, deadlineMs });
}

async function rejectionOf(promise: Promise<unknown>): Promise<unknown> {
  try {
    await promise;
  } catch (error) {
    return error;
  }
  throw new Error("expected the call to reject");
}

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

describe("getPrincipalReadOnly", () => {
  it("resolves the C1 principal on 200", async () => {
    await expect(client().getPrincipalReadOnly()).resolves.toEqual(principalFixture);
  });

  it("resolves the null sentinel on the defined no-record answer", async () => {
    server.use(http.get(`${API}/me/principal`, () => noRecordResponse()));

    await expect(client().getPrincipalReadOnly()).resolves.toBeNull();
  });

  it("rejects with a sanitised ApiError on a gateway deny-by-default 404", async () => {
    server.use(
      http.get(`${API}/me/principal`, () =>
        problemResponse(404, {
          type: "https://crypto-pocket-butler.example/problems/not-found",
          title: "Not found",
        }),
      ),
    );

    const error = await rejectionOf(client().getPrincipalReadOnly());

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).problem.status).toBe(404);
  });

  it("rejects a bare 404 that carries no problem type instead of treating it as no record", async () => {
    server.use(http.get(`${API}/me/principal`, () => new HttpResponse(null, { status: 404 })));

    const error = await rejectionOf(client().getPrincipalReadOnly());

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).problem.status).toBe(404);
  });

  it("rejects with a sanitised ApiError on 500, dropping the server detail", async () => {
    server.use(http.get(`${API}/me/principal`, () => problemResponse(500)));

    const error = await rejectionOf(client().getPrincipalReadOnly());

    expect(error).toBeInstanceOf(ApiError);
    const problem = (error as ApiError).problem;
    expect(problem.status).toBe(500);
    expect(problem.correlationId).toBe("corr-500");
    expect(problem.detail).toBeUndefined();
  });
});

describe("getSignupGateState", () => {
  it("resolves the gate state on 200", async () => {
    server.use(
      http.get(`${API}/signup/gate-state`, () =>
        HttpResponse.json(gateState({ missing: ["terms"], jurisdictionBlocked: false })),
      ),
    );

    await expect(client().getSignupGateState()).resolves.toEqual({
      missing: ["terms"],
      jurisdictionBlocked: false,
    });
  });

  it("resolves the null sentinel on the defined no-record answer", async () => {
    server.use(http.get(`${API}/signup/gate-state`, () => noRecordResponse()));

    await expect(client().getSignupGateState()).resolves.toBeNull();
  });

  it("rejects with a sanitised ApiError on 503", async () => {
    server.use(http.get(`${API}/signup/gate-state`, () => problemResponse(503)));

    const error = await rejectionOf(client().getSignupGateState());

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).problem.status).toBe(503);
    expect((error as ApiError).problem.detail).toBeUndefined();
  });
});

describe("submitSignupGates", () => {
  it("sends exactly the four-field body and the Idempotency-Key header", async () => {
    let captured: { body: unknown; key: string | null } | undefined;
    server.use(
      http.post(`${API}/signup/gates`, async ({ request }) => {
        captured = {
          body: (await request.json()) as unknown,
          key: request.headers.get("idempotency-key"),
        };
        return new HttpResponse(null, { status: 200 });
      }),
    );

    await expect(client().submitSignupGates(payload, "key-1")).resolves.toBeUndefined();

    expect(captured?.key).toBe("key-1");
    expect(captured?.body).toEqual({
      countryCode: "VN",
      terms: true,
      risk: true,
      age: true,
    });
    expect(Object.keys(captured?.body as object).sort()).toEqual([
      "age",
      "countryCode",
      "risk",
      "terms",
    ]);
  });

  it("maps a 403 to the restricted variant and drops the detail", async () => {
    server.use(http.post(`${API}/signup/gates`, () => problemResponse(403)));

    const error = await rejectionOf(client().submitSignupGates(payload, "key-1"));

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).problem.status).toBe(403);
    expect((error as ApiError).problem.detail).toBeUndefined();
    expect(gateFailureVariant(error)).toBe("restricted");
  });

  it("maps a 422 to the unasserted variant without parsing fields from the body", async () => {
    server.use(
      http.post(`${API}/signup/gates`, () =>
        HttpResponse.json(
          { ...problemBody(422), errors: { terms: ["must be accepted"] } },
          { status: 422 },
        ),
      ),
    );

    const error = await rejectionOf(client().submitSignupGates(payload, "key-1"));

    expect(gateFailureVariant(error)).toBe("unasserted");
    expect((error as ApiError).problem.errors).toBeUndefined();
  });

  it("maps a 503 to the nothing-saved variant", async () => {
    server.use(http.post(`${API}/signup/gates`, () => problemResponse(503)));

    const error = await rejectionOf(client().submitSignupGates(payload, "key-1"));

    expect(gateFailureVariant(error)).toBe("unavailable");
  });

  it("treats a network failure as incomplete", async () => {
    expect(gateFailureVariant(new NetworkError("connection refused"))).toBe("incomplete");
  });

  it("treats a 500 as incomplete", async () => {
    server.use(http.post(`${API}/signup/gates`, () => problemResponse(500)));

    const error = await rejectionOf(client().submitSignupGates(payload, "key-1"));

    expect(gateFailureVariant(error)).toBe("incomplete");
  });
});

describe("callWithDeadline", () => {
  it("aborts a hung read at the deadline and issues exactly one request", async () => {
    let requests = 0;
    server.use(
      http.get(`${API}/me/principal`, async () => {
        requests += 1;
        await delay("infinite");
        return HttpResponse.json(principalFixture);
      }),
    );

    const error = await rejectionOf(client(25).getPrincipalReadOnly());

    expect(error).toBeInstanceOf(DeadlineAbort);
    expect(requests).toBe(1);
    expect(gateFailureVariant(error)).toBe("incomplete");
  });

  it("aborts a hung submit at the deadline and issues exactly one request", async () => {
    let requests = 0;
    server.use(
      http.post(`${API}/signup/gates`, async () => {
        requests += 1;
        await delay("infinite");
        return new HttpResponse(null, { status: 200 });
      }),
    );

    const error = await rejectionOf(client(25).submitSignupGates(payload, "key-1"));

    expect(error).toBeInstanceOf(DeadlineAbort);
    expect(requests).toBe(1);
  });

  it("keeps the documented 15 s default and exposes the hung-handler factory", () => {
    expect(GATE_DEADLINE_MS).toBe(15_000);
    expect(hungGet).toBeTypeOf("function");
  });
});
