import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { Route, Routes } from "react-router-dom";
import { QueryClient } from "@tanstack/react-query";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../test/axe";
import { renderWithProviders } from "../test/render";
import { RequireGatesPassed } from "./RequireGatesPassed";
import { GATE_DESTINATIONS, type GateStatus } from "../features/signup-gates/gateStatus";
import {
  API,
  incompleteGateState,
  passedGateState,
  principal,
} from "../features/signup-gates/test/handlers";
import { server } from "../features/signup-gates/test/server";

/**
 * The post-sign-in gate guard (plan step 6.9/6.10; functional-spec §2.1;
 * AC6.2.2, AC6.5.2, NFR24.1).
 *
 * No branch enters the app on an unresolved status, and a remount resolves
 * again instead of trusting a cached `passed`.
 */

const session = vi.hoisted(() => ({ signOut: vi.fn(async () => {}) }));

vi.mock("../auth/session", () => ({
  useSession: () => ({
    status: "signed-in",
    session: { access_token: "test-token", user: { id: "user-1" } },
  }),
  signOut: session.signOut,
  getAccessToken: () => "test-token",
}));

const destinations = Object.values(GATE_DESTINATIONS);

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
beforeEach(() => {
  session.signOut.mockClear();
  server.use(
    http.get(`${API}/me/principal`, () =>
      HttpResponse.json(principal({ status: "active", nextStep: "none" })),
    ),
    http.get(`${API}/signup/gate-state`, () => HttpResponse.json(passedGateState)),
  );
});
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

function renderGuard(status?: GateStatus, options: { queryClient?: QueryClient } = {}) {
  return renderWithProviders(
    <Routes>
      {destinations.map((path) => (
        <Route key={path} path={path} element={<p data-testid={`destination:${path}`} />} />
      ))}
      <Route
        path="/x"
        element={
          <RequireGatesPassed status={status}>
            <p data-testid="product">product</p>
          </RequireGatesPassed>
        }
      />
    </Routes>,
    { route: "/x", queryClient: options.queryClient },
  );
}

describe("RequireGatesPassed", () => {
  it("renders children only when the gate state has passed", () => {
    renderGuard({ kind: "passed" });

    expect(screen.getByTestId("product")).toBeInTheDocument();
    expect(screen.queryByTestId("gates-form")).not.toBeInTheDocument();
  });

  it("renders the skeleton while the gate state is unresolved", () => {
    renderGuard({ kind: "loading" });

    expect(screen.getByTestId("gates-loading")).toBeInTheDocument();
    expect(screen.queryByTestId("product")).not.toBeInTheDocument();
  });

  it("renders the gate screen for any deep-linked product route", async () => {
    server.use(http.get(`${API}/signup/gate-state`, () => HttpResponse.json(incompleteGateState)));
    renderGuard();

    expect(await screen.findByTestId("gates-form")).toBeInTheDocument();
    expect(screen.queryByTestId("product")).not.toBeInTheDocument();
  });

  it("renders the blocked notice for both jurisdiction variants, never the app", async () => {
    for (const continuedUse of [true, false]) {
      const rendered = renderGuard({ kind: "jurisdiction-blocked", continuedUse });
      expect(await screen.findByTestId("gates-blocked-notice")).toBeInTheDocument();
      expect(screen.queryByTestId("product")).not.toBeInTheDocument();
      rendered.unmount();
    }
  });

  it("routes the registered owned-elsewhere destinations to their paths", async () => {
    for (const destination of ["verify-email", "enrol-mfa"] as const) {
      const rendered = renderGuard({ kind: "owned-elsewhere", destination });
      const path = GATE_DESTINATIONS[destination];
      expect(await screen.findByTestId(`destination:${path}`)).toBeInTheDocument();
      expect(screen.queryByTestId("product")).not.toBeInTheDocument();
      rendered.unmount();
    }
  });

  it("renders a terminal unavailable surface for account destinations instead of navigating", async () => {
    for (const destination of ["restore-account", "account-blocked"] as const) {
      const rendered = renderGuard({ kind: "owned-elsewhere", destination });

      expect(await screen.findByTestId("gates-destination-unavailable")).toBeInTheDocument();
      expect(
        screen.queryByTestId(`destination:${GATE_DESTINATIONS[destination]}`),
      ).not.toBeInTheDocument();
      expect(screen.queryByTestId("product")).not.toBeInTheDocument();

      await userEvent.click(screen.getByTestId("gates-signout"));
      expect(session.signOut).toHaveBeenCalledTimes(1);
      session.signOut.mockClear();
      rendered.unmount();
    }
  });

  it("renders the dedicated full-page retry on an entry error, never ErrorPanel", () => {
    renderGuard({ kind: "error", correlationId: "corr-entry" });

    expect(screen.getByTestId("gates-fullpage-retry")).toBeInTheDocument();
    expect(screen.queryByTestId("error-panel")).not.toBeInTheDocument();
    expect(screen.queryByTestId("product")).not.toBeInTheDocument();
  });

  it("re-resolves on remount instead of unlocking from a cached passed", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    let reads = 0;
    server.use(
      http.get(`${API}/signup/gate-state`, () => {
        reads += 1;
        return HttpResponse.json(reads === 1 ? passedGateState : incompleteGateState);
      }),
    );

    const first = renderGuard(undefined, { queryClient });
    expect(await screen.findByTestId("product", undefined, { timeout: 5000 })).toBeInTheDocument();
    first.unmount();

    renderGuard(undefined, { queryClient });
    expect(
      await screen.findByTestId("gates-form", undefined, { timeout: 5000 }),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("product")).not.toBeInTheDocument();
    await waitFor(() => expect(reads).toBeGreaterThanOrEqual(2));
  });

  it("is axe clean for every unresolved status", async () => {
    server.use(http.get(`${API}/signup/gate-state`, () => HttpResponse.json(incompleteGateState)));
    const statuses: GateStatus[] = [
      { kind: "loading" },
      { kind: "gates-required", serverMissing: ["terms"] },
      { kind: "jurisdiction-blocked", continuedUse: true },
      { kind: "owned-elsewhere", destination: "restore-account" },
      { kind: "error", correlationId: "corr-entry" },
    ];
    for (const status of statuses) {
      const rendered = renderGuard(status);
      if (status.kind === "gates-required") {
        await screen.findByTestId("gates-form");
      }
      await expectNoAxeViolations(rendered.container);
      rendered.unmount();
    }
  });
});
