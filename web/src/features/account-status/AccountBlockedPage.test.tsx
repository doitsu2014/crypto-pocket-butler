import { Route, Routes } from "react-router-dom";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoAxeViolations } from "../../test/axe";
import { problem, stubGateway } from "../../test/gateway";
import { renderWithProviders } from "../../test/render";
import AccountBlockedPage from "./AccountBlockedPage";

function renderBlocked(ui = <AccountBlockedPage variant="disabled" onSignOut={() => {}} />) {
  return renderWithProviders(ui, { route: "/account/blocked" });
}

describe("AccountBlockedPage", () => {
  it("names support on the disabled variant and offers no self-service control", async () => {
    renderBlocked();

    expect(await screen.findByTestId("account-blocked-disabled")).toHaveTextContent(/disabled/i);
    expect(screen.getByTestId("account-blocked-support")).toBeInTheDocument();
    expect(screen.getByTestId("account-blocked-sign-out")).toBeInTheDocument();
    expect(screen.queryByTestId("restore-account")).not.toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });

  it("keeps the deleted variant terminal", async () => {
    renderBlocked(<AccountBlockedPage variant="deleted" onSignOut={() => {}} />);

    expect(await screen.findByTestId("account-blocked-deleted")).toHaveTextContent(
      /has been deleted/i,
    );
    expect(screen.queryByTestId("account-blocked-support")).not.toBeInTheDocument();
    expect(screen.getByTestId("account-blocked-sign-out")).toBeInTheDocument();
  });

  it("derives the variant from the served account status when none is given", async () => {
    stubGateway({ "GET /v1/me/account-status": () => ({ body: { status: "deleted" } }) });
    renderBlocked(<AccountBlockedPage onSignOut={() => {}} />);

    expect(await screen.findByTestId("account-blocked-deleted")).toBeInTheDocument();
  });

  it("routes a pending-deletion account away from the blocked screen", async () => {
    stubGateway({
      "GET /v1/me/account-status": () => ({ body: { status: "pending_deletion" } }),
    });
    renderWithProviders(
      <Routes>
        <Route path="/account/blocked" element={<AccountBlockedPage onSignOut={() => {}} />} />
        <Route path="/account/restore" element={<p>the restore page</p>} />
      </Routes>,
      { route: "/account/blocked" },
    );

    expect(await screen.findByText("the restore page")).toBeInTheDocument();
    expect(screen.queryByTestId("account-blocked-disabled")).not.toBeInTheDocument();
  });

  it("routes an active account away from the blocked screen", async () => {
    stubGateway({ "GET /v1/me/account-status": () => ({ body: { status: "active" } }) });
    renderWithProviders(
      <Routes>
        <Route path="/account/blocked" element={<AccountBlockedPage onSignOut={() => {}} />} />
        <Route path="/settings/profile" element={<p>the profile page</p>} />
      </Routes>,
      { route: "/account/blocked" },
    );

    expect(await screen.findByText("the profile page")).toBeInTheDocument();
    expect(screen.queryByTestId("account-blocked-disabled")).not.toBeInTheDocument();
  });

  it("never shows the disabled screen when the status read fails", async () => {
    stubGateway({
      "GET /v1/me/account-status": () => ({
        status: 503,
        body: problem(503, "server detail that must never be rendered"),
      }),
    });
    renderBlocked(<AccountBlockedPage onSignOut={() => {}} />);

    expect(await screen.findByTestId("account-blocked-unresolved")).toBeInTheDocument();
    expect(screen.queryByTestId("account-blocked-disabled")).not.toBeInTheDocument();
    expect(screen.queryByText(/server detail/)).not.toBeInTheDocument();
  });

  it("signs out on request", async () => {
    const onSignOut = vi.fn();
    const user = userEvent.setup();
    renderBlocked(<AccountBlockedPage variant="disabled" onSignOut={onSignOut} />);

    await user.click(await screen.findByTestId("account-blocked-sign-out"));

    expect(onSignOut).toHaveBeenCalledTimes(1);
  });

  it("has no automated accessibility violations", async () => {
    const { container } = renderBlocked();

    await screen.findByTestId("account-blocked-disabled");
    await expectNoAxeViolations(container);
  });
});
