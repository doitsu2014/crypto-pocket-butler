import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createTranslator } from "../../i18n";
import { expectNoAxeViolations } from "../../test/axe";
import { renderWithProviders } from "../../test/render";
import { BlockedJurisdictionNotice } from "./BlockedJurisdictionNotice";
import { GateErrorNotice } from "./GateErrorNotice";

/**
 * The two notices of the gate screen (plan step 6.5/6.6; functional-spec
 * §2.5/§2.6; frontend-components §2; NFR25.1, NFR25.2).
 *
 * The blocked notice is terminal and offers Sign out only; the error notice is
 * retryable, manual, names the correlation id, and never renders the
 * ProblemDetails `detail` text.
 */

const t = createTranslator("en");

function renderBlocked(continuedUse: boolean) {
  const onSignOut = vi.fn();
  const rendered = renderWithProviders(
    <BlockedJurisdictionNotice continuedUse={continuedUse} onSignOut={onSignOut} />,
  );
  return { ...rendered, onSignOut };
}

describe("BlockedJurisdictionNotice", () => {
  it("states the service is unavailable, that no personal data is stored, and how to contact support", () => {
    renderBlocked(false);

    const notice = screen.getByTestId("gates-blocked-notice");
    expect(notice).toHaveAttribute("role", "status");
    expect(within(notice).getByText(t("gates.blocked.signupBody"))).toBeInTheDocument();
    expect(within(notice).getByText(t("gates.blocked.notStored"))).toBeInTheDocument();

    const support = within(notice).getByRole("link", { name: t("gates.blocked.support") });
    expect(support).toHaveAttribute("href", t("gates.blocked.supportHref"));
  });

  it("offers Sign out only — no retry, resubmit, or country re-selection exists", () => {
    renderBlocked(false);

    const notice = screen.getByTestId("gates-blocked-notice");
    expect(within(notice).getAllByRole("button")).toHaveLength(1);
    expect(screen.getByTestId("gates-signout")).toBeInTheDocument();
    expect(screen.queryByTestId("gates-error-retry")).not.toBeInTheDocument();
    expect(screen.queryByTestId("gates-form")).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
  });

  it("swaps in the temporarily-unavailable line for continued use, never the no-data line", () => {
    renderBlocked(true);

    const notice = screen.getByTestId("gates-blocked-notice");
    expect(within(notice).getByText(t("gates.blocked.continuedUseBody"))).toBeInTheDocument();
    expect(within(notice).queryByText(t("gates.blocked.notStored"))).not.toBeInTheDocument();
    expect(screen.getByTestId("gates-signout")).toBeInTheDocument();
  });

  it("never surfaces the restrictedBy signal, and Sign out calls the shell handler once", async () => {
    const user = userEvent.setup();
    const { container, onSignOut } = renderBlocked(false);
    const continued = renderBlocked(true);

    for (const root of [container, continued.container]) {
      expect(root.innerHTML).not.toMatch(/restrictedBy|restricted_by/i);
    }

    await user.click(screen.getAllByTestId("gates-signout")[0]);
    expect(onSignOut).toHaveBeenCalledTimes(1);
  });

  it("is axe clean in both variants", async () => {
    const signup = renderBlocked(false);
    await expectNoAxeViolations(signup.container);
    signup.unmount();

    const continued = renderBlocked(true);
    await expectNoAxeViolations(continued.container);
  });
});

describe("GateErrorNotice", () => {
  it("renders the nothing-saved wording with a manual retry", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn();
    renderWithProviders(
      <GateErrorNotice variant="nothing-saved" correlationId="corr-503" onRetry={onRetry} />,
    );

    const notice = screen.getByTestId("gates-error-notice");
    expect(within(notice).getByText(t("gates.error.nothingSaved"))).toBeInTheDocument();
    expect(
      within(notice).getByText(t("gates.error.correlation", { correlationId: "corr-503" })),
    ).toBeInTheDocument();

    expect(onRetry).not.toHaveBeenCalled();
    await user.click(screen.getByTestId("gates-error-retry"));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("renders the incomplete wording that never claims nothing was saved", () => {
    renderWithProviders(
      <GateErrorNotice variant="incomplete" correlationId="corr-500" onRetry={vi.fn()} />,
    );

    const notice = screen.getByTestId("gates-error-notice");
    expect(within(notice).getByText(t("gates.error.incomplete"))).toBeInTheDocument();
    expect(within(notice).queryByText(t("gates.error.nothingSaved"))).not.toBeInTheDocument();
    expect(
      within(notice).getByText(t("gates.error.correlation", { correlationId: "corr-500" })),
    ).toBeInTheDocument();
    expect(screen.getByTestId("gates-error-retry")).not.toBeDisabled();
  });

  it("never renders ProblemDetails detail text, and shows no correlation line without an id", () => {
    renderWithProviders(<GateErrorNotice variant="incomplete" onRetry={vi.fn()} />);

    const notice = screen.getByTestId("gates-error-notice");
    expect(notice).not.toHaveTextContent(/server-only detail/i);
    expect(within(notice).queryByText(/correlation/i)).not.toBeInTheDocument();
  });

  it("is axe clean in both variants", async () => {
    const first = renderWithProviders(
      <GateErrorNotice variant="nothing-saved" correlationId="corr-503" onRetry={vi.fn()} />,
    );
    await expectNoAxeViolations(first.container);
    first.unmount();

    const second = renderWithProviders(
      <GateErrorNotice variant="incomplete" correlationId="corr-500" onRetry={vi.fn()} />,
    );
    await expectNoAxeViolations(second.container);
  });
});
