import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import type { ReactNode } from "react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { GatewayClient } from "../../api/client";
import type { GateFormValues } from "./gateStatus";
import { API, gateState, problemBody, problemResponse } from "./test/handlers";
import { server } from "./test/server";
import { peekTicket, resetSubmissionTicket } from "./submissionTicket";
import { gateStateKey } from "./useGateStatus";
import { useSubmitGates } from "./useSubmitGates";

/**
 * G3 submit lifecycle (plan step 5.5/5.6; spec §2.4/§2.6; performance-design §6;
 * security-design §4). The hook maps allowed/restricted/unasserted/
 * nothing-saved/incomplete and keeps the ticket across retries.
 */

const sessionUserId = "user-1";

const values: GateFormValues = {
  countryCode: "VN",
  terms: true,
  risk: true,
  age: true,
};

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

function renderSubmit(options?: { deadlineMs?: number; queryClient?: QueryClient }) {
  const queryClient = options?.queryClient ?? newQueryClient();
  const client = new GatewayClient({
    getToken: () => null,
    deadlineMs: options?.deadlineMs,
  });
  const rendered = renderHook(() => useSubmitGates({ sessionUserId, client }), {
    wrapper: wrapperFor(queryClient),
  });
  return { ...rendered, queryClient };
}

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => {
  server.resetHandlers();
  resetSubmissionTicket();
});
afterAll(() => server.close());

beforeEach(() => {
  resetSubmissionTicket();
  server.use(
    http.get(`${API}/signup/gate-state`, () => HttpResponse.json(gateState())),
    http.post(`${API}/signup/gates`, () => new HttpResponse(null, { status: 200 })),
  );
});

describe("useSubmitGates outcomes", () => {
  it("resolves passed and invalidates the gate-state key on 200", async () => {
    const queryClient = newQueryClient();
    queryClient.setQueryData(gateStateKey(sessionUserId), gateState({ missing: ["terms"] }));
    const { result } = renderSubmit({ queryClient });

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toEqual({ kind: "passed" });
    expect(queryClient.getQueryState(gateStateKey(sessionUserId))?.isInvalidated).toBe(true);
  });

  it("resolves blocked on 403 without fetching G2 or retrying", async () => {
    let gateStateReads = 0;
    let submits = 0;
    server.use(
      http.get(`${API}/signup/gate-state`, () => {
        gateStateReads += 1;
        return HttpResponse.json(gateState());
      }),
      http.post(`${API}/signup/gates`, () => {
        submits += 1;
        return problemResponse(403);
      }),
    );
    const { result } = renderSubmit();

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toEqual({ kind: "blocked" });
    expect(gateStateReads).toBe(0);
    expect(submits).toBe(1);
  });

  it("resolves unasserted with the server's missing list on 422", async () => {
    server.use(
      http.get(`${API}/signup/gate-state`, () =>
        HttpResponse.json(gateState({ missing: ["terms", "age"] })),
      ),
      http.post(`${API}/signup/gates`, () =>
        HttpResponse.json(
          { ...problemBody(422), errors: { terms: ["must be accepted"] } },
          { status: 422 },
        ),
      ),
    );
    const { result } = renderSubmit();

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toEqual({ kind: "unasserted", missing: ["terms", "age"] });
  });

  it("resolves nothing-saved with the correlation id on 503", async () => {
    server.use(http.post(`${API}/signup/gates`, () => problemResponse(503)));
    const { result } = renderSubmit();

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toEqual({ kind: "nothing-saved", correlationId: "corr-503" });
  });

  it("resolves incomplete and re-reads G2 on a 500", async () => {
    let gateStateReads = 0;
    server.use(
      http.get(`${API}/signup/gate-state`, () => {
        gateStateReads += 1;
        return HttpResponse.json(gateState({ missing: ["risk"] }));
      }),
      http.post(`${API}/signup/gates`, () => problemResponse(500)),
    );
    const { result } = renderSubmit();

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toMatchObject({ kind: "incomplete", correlationId: "corr-500" });
    expect(gateStateReads).toBe(1);
  });

  it("resolves incomplete when the G2 re-read after a 422 fails", async () => {
    server.use(
      http.get(`${API}/signup/gate-state`, () => problemResponse(400)),
      http.post(`${API}/signup/gates`, () => problemResponse(422)),
    );
    const { result } = renderSubmit();

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toEqual({ kind: "incomplete", correlationId: "corr-422" });
  });

  it("does not turn a failed G2 re-read after a possibly-partial failure into an empty missing list", async () => {
    server.use(
      http.get(`${API}/signup/gate-state`, () => problemResponse(400)),
      http.post(`${API}/signup/gates`, () => problemResponse(500)),
    );
    const { result } = renderSubmit();

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toEqual({ kind: "incomplete", correlationId: "corr-500" });
  });

  it("resolves incomplete on a network failure", async () => {
    server.use(http.post(`${API}/signup/gates`, () => HttpResponse.error()));
    const { result } = renderSubmit();

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toMatchObject({ kind: "incomplete" });
  });

  it("resolves incomplete on a deadline with exactly one request", async () => {
    let submits = 0;
    server.use(
      http.post(`${API}/signup/gates`, async () => {
        submits += 1;
        await new Promise(() => undefined);
        return new HttpResponse(null, { status: 200 });
      }),
    );
    const { result } = renderSubmit({ deadlineMs: 25 });

    let outcome!: Awaited<ReturnType<typeof result.current.submit>>;
    await act(async () => {
      outcome = await result.current.submit(values);
    });

    expect(outcome).toMatchObject({ kind: "incomplete" });
    expect(submits).toBe(1);
  });

  it("never auto-retries a failure (exactly one request per attempt)", async () => {
    let submits = 0;
    server.use(
      http.post(`${API}/signup/gates`, () => {
        submits += 1;
        return problemResponse(500);
      }),
    );
    const { result } = renderSubmit();

    await act(async () => {
      await result.current.submit(values);
    });

    expect(submits).toBe(1);
  });
});

describe("useSubmitGates ticket integration", () => {
  it("reuses the ticket for an identical retry and mints a new one after an edit", async () => {
    const keys: string[] = [];
    let attempt = 0;
    server.use(
      http.post(`${API}/signup/gates`, async ({ request }) => {
        attempt += 1;
        keys.push(request.headers.get("idempotency-key") ?? "");
        return attempt === 1 ? problemResponse(500) : new HttpResponse(null, { status: 200 });
      }),
    );
    const { result } = renderSubmit();

    await act(async () => {
      await result.current.submit(values);
    });
    await act(async () => {
      await result.current.submit({ ...values });
    });

    expect(keys).toHaveLength(2);
    expect(keys[0]).toBeTruthy();
    expect(keys[1]).toBe(keys[0]);

    await act(async () => {
      await result.current.submit({ ...values, countryCode: "SG" });
    });

    expect(keys).toHaveLength(3);
    expect(keys[2]).not.toBe(keys[0]);
  });

  it("flips isPending while the submit is in flight and keeps the key", async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    server.use(
      http.post(`${API}/signup/gates`, async () => {
        await gate;
        return new HttpResponse(null, { status: 200 });
      }),
    );
    const { result } = renderSubmit();

    act(() => {
      void result.current.submit(values);
    });

    await waitFor(() => expect(result.current.isPending).toBe(true));
    expect(peekTicket(sessionUserId)).not.toBeNull();

    release();
    await waitFor(() => expect(result.current.isPending).toBe(false));
  });
});
