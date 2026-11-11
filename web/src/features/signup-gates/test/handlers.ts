import { delay, http, HttpResponse } from "msw";
import { NO_RECORD_PROBLEM_TYPE } from "../../../api/client";
import type { Principal, SignupGateState, SignupGatesPayload } from "../../../api/types";

/**
 * MSW v2 handlers for the three signup-gate endpoints (functional-spec §6 G1/G2/G3).
 *
 * The harness is feature-local by design: `web/src/test/setup.ts` and the existing
 * fetch stub stay untouched, so pre-existing suites keep their harness
 * (unit-test-instructions.md "Mocking guidance").
 *
 * Handlers are absolute URLs: Node's fetch (the runner's global) refuses a
 * relative request URL, and MSW matches the absolute URL the client sends.
 */

const ORIGIN = globalThis.location?.origin ?? "http://localhost:3000";
export const API = `${ORIGIN}/v1`;

export const principalFixture: Principal = {
  userId: "user-1",
  role: "owner",
  status: "pending",
  nextStep: "none",
};

export const passedGateState: SignupGateState = {
  missing: [],
  jurisdictionBlocked: false,
};

export const incompleteGateState: SignupGateState = {
  missing: ["terms", "risk", "age"],
  jurisdictionBlocked: false,
};

export function principal(overrides: Partial<Principal> = {}): Principal {
  return { ...principalFixture, ...overrides };
}

export function gateState(overrides: Partial<SignupGateState> = {}): SignupGateState {
  return { ...passedGateState, ...overrides };
}

/** A ProblemDetails body the tests can assert against (never rendered verbatim). */
export function problemBody(
  status: number,
  options: { type?: string; title?: string; correlationId?: string; detail?: string } = {},
) {
  return {
    type: options.type ?? `https://crypto-pocket-butler.example/problems/${status}`,
    title: options.title ?? `Gateway ${status}`,
    status,
    detail: options.detail ?? "server-only detail that must never render",
    correlationId: options.correlationId ?? `corr-${status}`,
  };
}

export function problemResponse(
  status: number,
  options: { type?: string; title?: string; correlationId?: string; detail?: string } = {},
) {
  return HttpResponse.json(problemBody(status, options), {
    status,
    headers: { "content-type": "application/problem+json" },
  });
}

/** The defined "no record" answer: a 404 problem, mapped to the `null` sentinel. */
export function noRecordResponse() {
  return problemResponse(404, {
    type: NO_RECORD_PROBLEM_TYPE,
    title: "No record",
    correlationId: "corr-no-record",
  });
}

export const defaultHandlers = [
  http.get(`${API}/me/principal`, () => HttpResponse.json(principalFixture)),
  http.get(`${API}/signup/gate-state`, () => HttpResponse.json(passedGateState)),
  http.post(`${API}/signup/gates`, () => new HttpResponse(null, { status: 200 })),
];

/** A handler that never answers, for deadline tests. */
export function hungGet(path: string) {
  return http.get(`${API}${path}`, async () => {
    await delay("infinite");
    return HttpResponse.json({});
  });
}

export function hungPost(path: string) {
  return http.post(`${API}${path}`, async () => {
    await delay("infinite");
    return HttpResponse.json({});
  });
}

export type { SignupGatesPayload };
