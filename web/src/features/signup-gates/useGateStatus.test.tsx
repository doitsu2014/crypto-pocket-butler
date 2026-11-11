import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import type { ReactNode } from "react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { GatewayClient } from "../../api/client";
import type { Principal, SignupGateState } from "../../api/types";
import { API, gateState, noRecordResponse, principal, problemResponse } from "./test/handlers";
import { server } from "./test/server";
import type { GateStatus } from "./gateStatus";
import { useGateStatus } from "./useGateStatus";

/**
 * Entry resolution (plan step 5.3/5.4; functional-spec §2.1 decision table,
 * first-match-wins; performance-design §2/§5/§8).
 */

const sessionUserId = "user-1";

function wrapperFor(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

function newQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
}

function renderGateStatus(options?: { deadlineMs?: number; queryClient?: QueryClient }) {
  const queryClient = options?.queryClient ?? newQueryClient();
  const client = options?.deadlineMs
    ? new GatewayClient({ getToken: () => null, deadlineMs: options.deadlineMs })
    : undefined;
  const rendered = renderHook(() => useGateStatus({ sessionUserId, client }), {
    wrapper: wrapperFor(queryClient),
  });
  return { ...rendered, queryClient };
}

const decisionCases: Array<{
  name: string;
  principal: Principal;
  state: SignupGateState;
  expected: GateStatus;
}> = [
  {
    name: "a pending record with nothing missing still needs the gates",
    principal: principal({ status: "pending", nextStep: "none" }),
    state: gateState({ missing: [] }),
    expected: { kind: "gates-required", serverMissing: [] },
  },
  {
    name: "verify_email routes to verify-email before any gate row",
    principal: principal({ status: "pending", nextStep: "verify_email" }),
    state: gateState({ missing: ["terms"] }),
    expected: { kind: "owned-elsewhere", destination: "verify-email" },
  },
  {
    name: "enrol_mfa routes to enrol-mfa",
    principal: principal({ status: "active", nextStep: "enrol_mfa" }),
    state: gateState({ missing: [] }),
    expected: { kind: "owned-elsewhere", destination: "enrol-mfa" },
  },
  {
    name: "restore_account routes to restore-account",
    principal: principal({ status: "active", nextStep: "restore_account" }),
    state: gateState({ missing: [] }),
    expected: { kind: "owned-elsewhere", destination: "restore-account" },
  },
  {
    name: "a disabled account routes to the blocked screen",
    principal: principal({ status: "disabled", nextStep: "none" }),
    state: gateState({ missing: [] }),
    expected: { kind: "owned-elsewhere", destination: "account-blocked" },
  },
  {
    name: "a pending-deletion account routes to the blocked screen",
    principal: principal({ status: "pending_deletion", nextStep: "none" }),
    state: gateState({ missing: [] }),
    expected: { kind: "owned-elsewhere", destination: "account-blocked" },
  },
  {
    name: "a deleted account routes to the blocked screen",
    principal: principal({ status: "deleted", nextStep: "none" }),
    state: gateState({ missing: [] }),
    expected: { kind: "owned-elsewhere", destination: "account-blocked" },
  },
  {
    name: "an active record with server-named missing items needs the gates",
    principal: principal({ status: "active", nextStep: "none" }),
    state: gateState({ missing: ["terms", "age"] }),
    expected: { kind: "gates-required", serverMissing: ["terms", "age"] },
  },
  {
    name: "a pending record with missing items names them",
    principal: principal({ status: "pending", nextStep: "none" }),
    state: gateState({ missing: ["risk"] }),
    expected: { kind: "gates-required", serverMissing: ["risk"] },
  },
  {
    name: "continued use blocked by jurisdiction wins over passed",
    principal: principal({ status: "active", nextStep: "none" }),
    state: gateState({ missing: [], jurisdictionBlocked: true }),
    expected: { kind: "jurisdiction-blocked", continuedUse: true },
  },
  {
    name: "a fully gated active record passes",
    principal: principal({ status: "active", nextStep: "none" }),
    state: gateState({ missing: [], jurisdictionBlocked: false }),
    expected: { kind: "passed" },
  },
];

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

describe("useGateStatus decision table", () => {
  beforeEach(() => {
    server.use(
      http.get(`${API}/me/principal`, () => HttpResponse.json(principal())),
      http.get(`${API}/signup/gate-state`, () => HttpResponse.json(gateState())),
    );
  });

  it.each(decisionCases)("$name", async ({ principal: principalRow, state, expected }) => {
    server.use(
      http.get(`${API}/me/principal`, () => HttpResponse.json(principalRow)),
      http.get(`${API}/signup/gate-state`, () => HttpResponse.json(state)),
    );

    const { result } = renderGateStatus();

    await waitFor(() => expect(result.current).toEqual(expected));
  });

  it("treats two no-record sentinels as the Gate Screen with no server-named items", async () => {
    server.use(
      http.get(`${API}/me/principal`, () => noRecordResponse()),
      http.get(`${API}/signup/gate-state`, () => noRecordResponse()),
    );

    const { result } = renderGateStatus();

    await waitFor(() =>
      expect(result.current).toEqual({
        kind: "gates-required",
        serverMissing: [],
        noRecord: true,
      }),
    );
  });

  it("starts in the loading state before either read settles", () => {
    const { result } = renderGateStatus();

    expect(result.current).toEqual({ kind: "loading" });
  });
});

describe("useGateStatus transport policy", () => {
  beforeEach(() => {
    server.use(
      http.get(`${API}/me/principal`, () => HttpResponse.json(principal({ status: "active" }))),
      http.get(`${API}/signup/gate-state`, () => HttpResponse.json(gateState())),
    );
  });

  it("issues G1 and G2 in parallel before either settles", async () => {
    const started: string[] = [];
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    server.use(
      http.get(`${API}/me/principal`, async () => {
        started.push("principal");
        await gate;
        return HttpResponse.json(principal({ status: "active" }));
      }),
      http.get(`${API}/signup/gate-state`, async () => {
        started.push("gate-state");
        await gate;
        return HttpResponse.json(gateState());
      }),
    );

    const { result } = renderGateStatus();

    await waitFor(() =>
      expect(started).toEqual(expect.arrayContaining(["principal", "gate-state"])),
    );
    release();
    await waitFor(() => expect(result.current.kind).toBe("passed"));
  });

  it("re-resolves on re-entry instead of unlocking from cached state", async () => {
    const queryClient = newQueryClient();
    const first = renderGateStatus({ queryClient });

    await waitFor(() => expect(first.result.current.kind).toBe("passed"));
    first.unmount();

    server.use(
      http.get(`${API}/signup/gate-state`, () =>
        HttpResponse.json(gateState({ missing: ["terms"] })),
      ),
    );
    const second = renderGateStatus({ queryClient });

    await waitFor(() =>
      expect(second.result.current).toEqual({
        kind: "gates-required",
        serverMissing: ["terms"],
      }),
    );
  });

  it("surfaces a deadline as an error after exactly one attempt", async () => {
    let requests = 0;
    server.use(
      http.get(`${API}/me/principal`, async () => {
        requests += 1;
        await new Promise(() => undefined);
        return HttpResponse.json(principal());
      }),
    );

    const { result } = renderGateStatus({ deadlineMs: 25 });

    await waitFor(() => expect(result.current.kind).toBe("error"), { timeout: 2000 });
    expect(requests).toBe(1);
  });

  it("retries a fast 5xx at most three times", async () => {
    let requests = 0;
    server.use(
      http.get(`${API}/me/principal`, () => {
        requests += 1;
        return problemResponse(500);
      }),
    );

    const { result } = renderGateStatus();

    await waitFor(() => expect(result.current.kind).toBe("error"), { timeout: 8000 });
    expect(requests).toBe(3);
  }, 10_000);

  it("never retries a 404 route-missing problem", async () => {
    let requests = 0;
    server.use(
      http.get(`${API}/me/principal`, () => {
        requests += 1;
        return problemResponse(404, {
          type: "https://crypto-pocket-butler.example/problems/not-found",
        });
      }),
    );

    const { result } = renderGateStatus();

    await waitFor(() => expect(result.current.kind).toBe("error"));
    expect(requests).toBe(1);
  });
});
