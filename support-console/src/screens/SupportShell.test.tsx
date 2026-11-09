import { act, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { afterEach, describe, expect, it, vi } from "vitest";
import { getSupportSession, setSupportSession } from "../auth/session";
import {
  activeGrant,
  adminSession,
  pendingGrant,
  renderWithQuery,
  supportSession,
  ticketMetadata,
} from "../test/fixtures";
import { server } from "../test/server";
import { SupportShell } from "./SupportShell";

afterEach(() => {
  vi.useRealTimers();
});

describe("SupportShell", () => {
  it("shows the grant workspace to a support user and hides incidents", () => {
    renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialGrant={activeGrant()}
        initialPendingGrants={[pendingGrant()]}
      />,
    );

    expect(screen.getByTestId("grant-banner")).toBeInTheDocument();
    expect(screen.getByTestId("grant-request-form")).toBeInTheDocument();
    expect(screen.getByTestId("grant-approval-queue")).toBeInTheDocument();
    expect(screen.queryByTestId("nav-incidents")).not.toBeInTheDocument();
    expect(screen.getByTestId("nav-grants")).toHaveAttribute("aria-current", "page");
  });

  it("shows the incident console to an admin", async () => {
    renderWithQuery(
      <SupportShell session={adminSession} role="admin" onSignOut={vi.fn()} initialSection="incidents" />,
    );

    expect(screen.getByTestId("nav-incidents")).toBeInTheDocument();
    expect(await screen.findByTestId("incident-console")).toBeInTheDocument();
  });

  it("navigates to grant-scoped search by keyboard-accessible links", async () => {
    const user = userEvent.setup();
    renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialGrant={activeGrant()}
      />,
    );

    await user.click(screen.getByTestId("nav-search"));

    expect(screen.getByTestId("account-search")).toBeInTheDocument();
    expect(screen.queryByTestId("account-search-grant-required")).not.toBeInTheDocument();
  });

  it("loads the ticket queue on its section when a grant is active", async () => {
    server.use(http.get("*/v1/support/tickets", () => HttpResponse.json({ items: [ticketMetadata] })));
    renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialGrant={activeGrant()}
        initialSection="tickets"
      />,
    );

    expect(await screen.findByText("Sync status question")).toBeInTheDocument();
  });

  it("requires a grant for the ticket queue", () => {
    renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialSection="tickets"
      />,
    );

    expect(screen.getByTestId("ticket-queue-grant-required")).toBeInTheDocument();
  });

  it("clears the cache and the in-memory session on sign-out", async () => {
    const user = userEvent.setup();
    setSupportSession(supportSession);
    const onSignOut = vi.fn();
    const { client } = renderWithQuery(
      <SupportShell session={supportSession} role="support" onSignOut={onSignOut} />,
    );
    client.setQueryData(["support", "tickets", ""], { items: [] });

    await user.click(screen.getByTestId("support-shell-sign-out"));

    expect(client.getQueryCache().getAll()).toHaveLength(0);
    expect(getSupportSession()).toBeNull();
    expect(onSignOut).toHaveBeenCalled();
  });

  it("clears the cache and disables reads when the grant expires", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-11-08T10:00:00.000Z"));
    const grant = activeGrant({ expiresAt: "2026-11-08T10:00:10.000Z" });
    const { client } = renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialGrant={grant}
        initialSection="search"
      />,
    );
    client.setQueryData(["support", "accounts"], { items: [] });
    expect(screen.queryByTestId("account-search-grant-required")).not.toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });

    expect(screen.getByTestId("account-search-grant-required")).toBeInTheDocument();
    expect(client.getQueryCache().getAll()).toHaveLength(0);
  });

  it("puts the sign-out control and section links in keyboard order", async () => {
    const user = userEvent.setup();
    renderWithQuery(
      <SupportShell session={supportSession} role="support" onSignOut={vi.fn()} />,
    );

    await user.tab();
    expect(screen.getByTestId("support-shell-sign-out")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("nav-grants")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("nav-search")).toHaveFocus();
  });

  it("has no accessibility violations", async () => {
    const { container } = renderWithQuery(
      <SupportShell session={supportSession} role="support" onSignOut={vi.fn()} />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
