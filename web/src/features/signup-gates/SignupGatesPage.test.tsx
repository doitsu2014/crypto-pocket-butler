import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { delay, http, HttpResponse } from "msw";
import { Route, Routes } from "react-router-dom";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { createTranslator } from "../../i18n";
import { expectNoAxeViolations } from "../../test/axe";
import { renderWithProviders } from "../../test/render";
import { GatewayClient } from "../../api/client";
import type { Principal, SignupGateState } from "../../api/types";
import SignupGatesPage from "./SignupGatesPage";
import {
  API,
  gateState,
  hungGet,
  incompleteGateState,
  noRecordResponse,
  passedGateState,
  principal,
  principalFixture,
  problemResponse,
} from "./test/handlers";
import { server } from "./test/server";

/**
 * The gate screen as a route module (plan step 6.7/6.8; functional-spec
 * §2.2–§2.6; NFR25).
 *
 * The shell session holder is replaced so the hooks see a signed-in visitor;
 * the gateway answers come from MSW handlers over the real client, never from
 * mocked feature modules.
 */

const session = vi.hoisted(() => ({ signOut: vi.fn(async () => {}) }));

vi.mock("../../auth/session", () => ({
  useSession: () => ({
    status: "signed-in",
    session: { access_token: "test-token", user: { id: "user-1" } },
  }),
  signOut: session.signOut,
  getAccessToken: () => "test-token",
}));

const t = createTranslator("en");

let principalState: Principal;
let gate: SignupGateState;

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));

beforeEach(() => {
  session.signOut.mockClear();
  principalState = { ...principalFixture };
  gate = { ...incompleteGateState };
  server.use(
    http.get(`${API}/me/principal`, () => HttpResponse.json(principalState)),
    http.get(`${API}/signup/gate-state`, () => HttpResponse.json(gate)),
  );
});

afterEach(() => server.resetHandlers());
afterAll(() => server.close());

function renderPage(client?: GatewayClient) {
  return renderWithProviders(
    <Routes>
      <Route path="/gates" element={<SignupGatesPage client={client} />} />
      <Route path="/dashboard" element={<p data-testid="app-content">app</p>} />
    </Routes>,
    { route: "/gates" },
  );
}

async function fillAll(user: ReturnType<typeof userEvent.setup>) {
  await user.type(screen.getByTestId("gates-country-input"), "viet");
  await user.click(screen.getByTestId("gates-country-option-VN"));
  await user.click(screen.getByTestId("gates-terms-checkbox"));
  await user.click(screen.getByTestId("gates-risk-checkbox"));
  await user.click(screen.getByTestId("gates-age-checkbox"));
}

function markRegistered() {
  principalState = principal({ status: "active", nextStep: "none" });
  gate = { ...passedGateState };
}

describe("SignupGatesPage", () => {
  it("renders a dedicated loading state before the gate state resolves", async () => {
    server.use(hungGet("/me/principal"));
    const { container } = renderPage();

    expect(screen.getByTestId("gates-loading")).toBeInTheDocument();
    await expectNoAxeViolations(container);
    expect(screen.queryByTestId("gates-form")).not.toBeInTheDocument();
  });

  it("shows every acceptance unchecked and enabled on a first visit, and submits them as true", async () => {
    const user = userEvent.setup();
    let body: unknown;
    let registered = false;
    server.use(
      http.get(`${API}/me/principal`, () =>
        registered
          ? HttpResponse.json(principal({ status: "active", nextStep: "none" }))
          : noRecordResponse(),
      ),
      http.get(`${API}/signup/gate-state`, () =>
        registered ? HttpResponse.json(passedGateState) : noRecordResponse(),
      ),
      http.post(`${API}/signup/gates`, async ({ request }) => {
        body = (await request.json()) as unknown;
        registered = true;
        return new HttpResponse(null, { status: 200 });
      }),
    );
    renderPage();

    await screen.findByTestId("gates-form");
    for (const kind of ["terms", "risk", "age"] as const) {
      const box = screen.getByTestId(`gates-${kind}-checkbox`);
      expect(box).not.toBeChecked();
      expect(box).toBeEnabled();
    }

    await fillAll(user);
    await user.click(screen.getByTestId("gates-continue"));

    await screen.findByTestId("app-content", undefined, { timeout: 5000 });
    expect(body).toEqual({ countryCode: "VN", terms: true, risk: true, age: true });
  });

  it("submits, announces progress, keeps Continue disabled, then reaches the app content", async () => {
    const user = userEvent.setup();
    const gate = { release: null as (() => void) | null };
    server.use(
      http.post(`${API}/signup/gates`, async () => {
        await new Promise<void>((resolve) => {
          gate.release = resolve;
        });
        markRegistered();
        return new HttpResponse(null, { status: 200 });
      }),
    );
    const { container } = renderPage();

    await screen.findByTestId("gates-form");
    await fillAll(user);
    await user.click(screen.getByTestId("gates-continue"));

    await waitFor(() => expect(screen.getByTestId("gates-continue")).toBeDisabled());
    expect(screen.getByRole("status")).toHaveTextContent(t("gates.submitting"));
    await expectNoAxeViolations(container);

    gate.release?.();
    await screen.findByTestId("app-content", undefined, { timeout: 5000 });
  });

  it("replaces the screen with the blocked notice on 403 and offers Sign out only", async () => {
    const user = userEvent.setup();
    server.use(http.post(`${API}/signup/gates`, () => problemResponse(403)));
    renderPage();

    await screen.findByTestId("gates-form");
    await fillAll(user);
    await user.click(screen.getByTestId("gates-continue"));

    const notice = await screen.findByTestId("gates-blocked-notice");
    expect(within(notice).getByText(t("gates.blocked.notStored"))).toBeInTheDocument();
    expect(screen.queryByTestId("gates-form")).not.toBeInTheDocument();
    expect(within(notice).getAllByRole("button")).toHaveLength(1);

    await user.click(screen.getByTestId("gates-signout"));
    expect(session.signOut).toHaveBeenCalledTimes(1);
  });

  it("re-renders the screen from G2 after a 422, naming only what the server reports", async () => {
    const user = userEvent.setup();
    let gateReads = 0;
    server.use(
      http.get(`${API}/signup/gate-state`, () => {
        gateReads += 1;
        return HttpResponse.json(gate);
      }),
      http.post(`${API}/signup/gates`, () => {
        gate = gateState({ missing: ["risk"] });
        return problemResponse(422);
      }),
    );
    renderPage();

    await screen.findByTestId("gates-form");
    await fillAll(user);
    await user.click(screen.getByTestId("gates-continue"));

    await waitFor(() => expect(gateReads).toBeGreaterThanOrEqual(2));
    const summary = await screen.findByTestId("gates-validation-summary");
    expect(within(summary).getByText(t("gates.risk.required"))).toBeInTheDocument();
    expect(screen.getByTestId("gates-terms-checkbox")).toBeChecked();
    expect(screen.getByTestId("gates-terms-checkbox")).toBeDisabled();
    expect(screen.getByTestId("gates-age-checkbox")).toBeDisabled();
  });

  it("shows the nothing-saved error on 503 and keeps the field values", async () => {
    const user = userEvent.setup();
    server.use(http.post(`${API}/signup/gates`, () => problemResponse(503)));
    renderPage();

    await screen.findByTestId("gates-form");
    await fillAll(user);
    await user.click(screen.getByTestId("gates-continue"));

    const notice = await screen.findByTestId("gates-error-notice");
    expect(within(notice).getByText(t("gates.error.nothingSaved"))).toBeInTheDocument();
    expect(screen.getByTestId("gates-country-input")).toHaveValue("Vietnam");
    expect(screen.getByTestId("gates-terms-checkbox")).toBeChecked();
    expect(screen.getByTestId("gates-risk-checkbox")).toBeChecked();
    expect(screen.getByTestId("gates-age-checkbox")).toBeChecked();
  });

  it("surfaces the incomplete error on a mid-sequence 5xx and retries with the same key", async () => {
    const user = userEvent.setup();
    const keys: string[] = [];
    let attempts = 0;
    server.use(
      http.post(`${API}/signup/gates`, ({ request }) => {
        keys.push(request.headers.get("Idempotency-Key") ?? "");
        attempts += 1;
        if (attempts === 1) {
          gate = gateState({ missing: ["risk", "age"] });
          return problemResponse(500);
        }
        markRegistered();
        return new HttpResponse(null, { status: 200 });
      }),
    );
    renderPage();

    await screen.findByTestId("gates-form");
    await fillAll(user);
    await user.click(screen.getByTestId("gates-continue"));

    const notice = await screen.findByTestId("gates-error-notice");
    expect(within(notice).getByText(t("gates.error.incomplete"))).toBeInTheDocument();
    expect(
      within(notice).getByText(t("gates.error.correlation", { correlationId: "corr-500" })),
    ).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId("gates-terms-checkbox")).toBeDisabled());

    await user.click(screen.getByTestId("gates-error-retry"));
    await screen.findByTestId("app-content", undefined, { timeout: 5000 });

    expect(keys).toHaveLength(2);
    expect(keys[0]).not.toBe("");
    expect(keys[1]).toBe(keys[0]);
  });

  it("treats a hung submission as incomplete once the deadline passes, never auto-retrying", async () => {
    const user = userEvent.setup();
    let posts = 0;
    server.use(
      http.post(`${API}/signup/gates`, async () => {
        posts += 1;
        await delay("infinite");
        return HttpResponse.json({});
      }),
    );
    const client = new GatewayClient({ getToken: () => "test-token", deadlineMs: 40 });
    renderPage(client);

    await screen.findByTestId("gates-form");
    await fillAll(user);
    await user.click(screen.getByTestId("gates-continue"));

    const notice = await screen.findByTestId("gates-error-notice", undefined, { timeout: 5000 });
    expect(within(notice).getByText(t("gates.error.incomplete"))).toBeInTheDocument();
    expect(screen.getByTestId("gates-country-input")).toHaveValue("Vietnam");
    expect(posts).toBe(1);
  });

  it("shows the full-page retry on an entry read failure and re-reads on retry", async () => {
    const user = userEvent.setup();
    let reads = 0;
    server.use(
      http.get(`${API}/me/principal`, () => {
        reads += 1;
        return problemResponse(500);
      }),
    );
    renderPage();

    await screen.findByTestId("gates-fullpage-retry", undefined, { timeout: 5000 });
    expect(screen.queryByTestId("error-panel")).not.toBeInTheDocument();
    expect(screen.queryByTestId("gates-form")).not.toBeInTheDocument();
    const before = reads;

    await user.click(screen.getByTestId("gates-fullpage-retry"));
    await waitFor(() => expect(reads).toBeGreaterThan(before), { timeout: 5000 });
  });

  it("renders the continued-use blocked notice from entry routing", async () => {
    principalState = principal({ status: "active", nextStep: "none" });
    gate = gateState({ missing: [], jurisdictionBlocked: true });
    const { container } = renderPage();

    const notice = await screen.findByTestId("gates-blocked-notice");
    expect(within(notice).getByText(t("gates.blocked.continuedUseBody"))).toBeInTheDocument();
    expect(within(notice).queryByText(t("gates.blocked.notStored"))).not.toBeInTheDocument();
    await expectNoAxeViolations(container);
  });

  it("is axe clean with the validation summary, the nothing-saved error, and the retry page", async () => {
    const user = userEvent.setup();
    const ready = renderPage();
    await screen.findByTestId("gates-form");

    await user.click(screen.getByTestId("gates-continue"));
    await screen.findByTestId("gates-validation-summary");
    await expectNoAxeViolations(ready.container);

    server.use(http.post(`${API}/signup/gates`, () => problemResponse(503)));
    await fillAll(user);
    await user.click(screen.getByTestId("gates-continue"));
    await screen.findByTestId("gates-error-notice");
    await expectNoAxeViolations(ready.container);
    ready.unmount();

    server.use(http.get(`${API}/me/principal`, () => problemResponse(500)));
    const retry = renderPage();
    await screen.findByTestId("gates-fullpage-retry", undefined, { timeout: 5000 });
    await expectNoAxeViolations(retry.container);
  });
});
