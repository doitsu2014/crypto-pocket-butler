import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { formatDateTime } from "../../i18n";
import { problem, stubGateway } from "../../test/gateway";
import { renderWithProviders } from "../../test/render";
import { expectNoAxeViolations } from "../../test/axe";
import type { ConsentHistoryView, ConsentStateView } from "../../api/types";
import ConsentPanel, { ANALYTICS_POLICY_VERSION } from "./ConsentPanel";

const grantedOn = "2026-08-01T09:00:00Z";
const withdrawnOn = "2026-10-01T09:00:00Z";

function state(analytics: string): ConsentStateView {
  return {
    userId: "user-1",
    kinds: {
      terms: "granted",
      risk: "granted",
      age: "granted",
      analytics,
      marketing: "none",
    },
    missing: [],
  };
}

function history(analyticsWithdrawnAt: string | null): ConsentHistoryView {
  return {
    events: [
      ...(analyticsWithdrawnAt
        ? [
            {
              id: "event-4",
              seq: 4,
              kind: "analytics",
              action: "withdrawn",
              occurredAt: analyticsWithdrawnAt,
              policyVersion: ANALYTICS_POLICY_VERSION,
            },
          ]
        : []),
      {
        id: "event-3",
        seq: 3,
        kind: "analytics",
        action: "granted",
        occurredAt: grantedOn,
        policyVersion: ANALYTICS_POLICY_VERSION,
      },
      {
        id: "event-2",
        seq: 2,
        kind: "terms",
        action: "granted",
        occurredAt: grantedOn,
        policyVersion: ANALYTICS_POLICY_VERSION,
      },
      {
        id: "event-1",
        seq: 1,
        kind: "risk",
        action: "granted",
        occurredAt: grantedOn,
        policyVersion: ANALYTICS_POLICY_VERSION,
      },
      {
        id: "event-0",
        seq: 0,
        kind: "age",
        action: "granted",
        occurredAt: grantedOn,
        policyVersion: ANALYTICS_POLICY_VERSION,
      },
    ],
  };
}

function renderPanel() {
  return renderWithProviders(<ConsentPanel />, { route: "/settings/consent" });
}

describe("ConsentPanel", () => {
  it("grants analytics with the policy version and re-reads the server state", async () => {
    const grants: unknown[] = [];
    let analytics = "withdrawn";
    stubGateway({
      "GET /v1/me/consent/state": () => ({ body: state(analytics) }),
      "GET /v1/me/consent": () => ({ body: history(null) }),
      "POST /v1/me/consent": ({ body }) => {
        grants.push(body);
        analytics = "granted";
        return { status: 201, body: { kind: "analytics", state: "granted" } };
      },
    });
    const user = userEvent.setup();
    renderPanel();

    const toggle = await screen.findByTestId("consent-analytics");
    expect(toggle).toHaveAttribute("aria-checked", "false");
    await user.click(toggle);

    expect(grants).toEqual([{ kind: "analytics", policyVersion: ANALYTICS_POLICY_VERSION }]);
    await waitFor(() =>
      expect(screen.getByTestId("consent-analytics")).toHaveAttribute("aria-checked", "true"),
    );
    expect(screen.queryByTestId("consent-analytics-withdrawn")).not.toBeInTheDocument();
  });

  it("withdraws analytics and shows the withdrawal date only after the withdrawal", async () => {
    let analytics = "granted";
    let withdrew = false;
    stubGateway({
      "GET /v1/me/consent/state": () => ({ body: state(analytics) }),
      "GET /v1/me/consent": () => ({
        body: history(withdrew ? withdrawnOn : null),
      }),
      "POST /v1/me/consent/analytics/withdraw": () => {
        analytics = "withdrawn";
        withdrew = true;
        return { body: { kind: "analytics", state: "withdrawn", changed: true } };
      },
    });
    const user = userEvent.setup();
    renderPanel();

    const toggle = await screen.findByTestId("consent-analytics");
    expect(toggle).toHaveAttribute("aria-checked", "true");
    expect(screen.queryByTestId("consent-analytics-withdrawn")).not.toBeInTheDocument();

    await user.click(toggle);

    await waitFor(() =>
      expect(screen.getByTestId("consent-analytics")).toHaveAttribute("aria-checked", "false"),
    );
    expect(screen.getByTestId("consent-analytics-withdrawn")).toHaveTextContent(
      formatDateTime(withdrawnOn, "en", "UTC"),
    );
  });

  it("re-reads the server state on 409 instead of showing a failure", async () => {
    let analytics = "none";
    stubGateway({
      "GET /v1/me/consent/state": () => ({ body: state(analytics) }),
      "GET /v1/me/consent": () => ({ body: history(null) }),
      "POST /v1/me/consent": () => {
        // The server already recorded a grant this tab had not seen.
        analytics = "granted";
        return { status: 409, body: problem(409, "server detail that must never be rendered") };
      },
    });
    const user = userEvent.setup();
    renderPanel();

    await user.click(await screen.findByTestId("consent-analytics"));

    await waitFor(() =>
      expect(screen.getByTestId("consent-analytics")).toHaveAttribute("aria-checked", "true"),
    );
    expect(screen.queryByText(/server detail/)).not.toBeInTheDocument();
  });

  it("renders the required acceptances read-only with their latest granted date", async () => {
    stubGateway({
      "GET /v1/me/consent/state": () => ({ body: state("withdrawn") }),
      "GET /v1/me/consent": () => ({ body: history(null) }),
    });
    renderPanel();

    for (const kind of ["terms", "risk", "age"]) {
      const row = await screen.findByTestId(`consent-kind-${kind}`);
      expect(row).toHaveTextContent(formatDateTime(grantedOn, "en", "UTC"));
      expect(row.querySelector("input, button, select")).toBeNull();
    }
  });

  it("has no automated accessibility violations", async () => {
    stubGateway({
      "GET /v1/me/consent/state": () => ({ body: state("granted") }),
      "GET /v1/me/consent": () => ({ body: history(withdrawnOn) }),
    });
    const { container } = renderPanel();

    await screen.findByTestId("consent-analytics");
    await expectNoAxeViolations(container);
  });
});
