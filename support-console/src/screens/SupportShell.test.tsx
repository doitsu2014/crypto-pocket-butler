import { act, screen, waitFor } from "@testing-library/react";
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
  supportMetadata,
  supportSession,
  ticketMetadata,
} from "../test/fixtures";
import { server } from "../test/server";
import { SupportShell } from "./SupportShell";

const REASON = "Investigating a reported sync failure";

afterEach(() => {
  vi.useRealTimers();
  // `location.hash = ""` on a URL without a fragment rewrites it to `/#` and
  // queues a jsdom hashchange that lands in the next test, resetting the shell
  // to the grants section. Replacing the fragment fires no event.
  window.history.replaceState(null, "", "/");
});

describe("SupportShell", () => {
  it("adopts the newest grant from mine without comparing requestedBy", async () => {
    const newest = activeGrant({
      id: "grant-newest",
      requestedBy: "identity-user-999",
      reason: REASON,
    });
    const older = activeGrant({ id: "grant-older", requestedBy: "identity-user-999" });
    server.use(
      http.get("*/v1/support-grants/mine", () => HttpResponse.json([newest, older])),
      http.get("*/v1/support-grants/*/metadata", () => HttpResponse.json(supportMetadata())),
    );

    renderWithQuery(<SupportShell session={supportSession} role="support" onSignOut={vi.fn()} />);

    const banner = await screen.findByTestId("grant-banner");
    expect(banner).toHaveTextContent(REASON);
    expect(banner).not.toHaveTextContent(older.reason === REASON ? "older" : "grant-older");
  });

  it("adopts a just-created grant from the 201 and keeps it over a stale mine answer", async () => {
    const created = pendingGrant({ id: "grant-created", reason: "Just created reason" });
    server.use(
      http.get("*/v1/support-grants/mine", () => HttpResponse.json([])),
      http.post("*/v1/support-grants", () => HttpResponse.json(created, { status: 201 })),
    );

    const user = userEvent.setup();
    renderWithQuery(<SupportShell session={supportSession} role="support" onSignOut={vi.fn()} />);

    await user.type(await screen.findByTestId("grant-request-target"), "user-1");
    await user.type(screen.getByTestId("grant-request-reason"), REASON);
    await user.click(screen.getByTestId("grant-request-submit"));

    const banner = await screen.findByTestId("grant-banner");
    expect(banner).toHaveTextContent("Just created reason");
    expect(banner).toHaveTextContent("Pending approval");
  });

  it("never lets an admin's approval of another agent's grant become the admin's banner", async () => {
    const otherGrant = pendingGrant({ id: "grant-other", requestedBy: "agent-2" });
    const approved = {
      ...otherGrant,
      status: "active" as const,
      expiresAt: new Date(Date.now() + 3_600_000).toISOString(),
    };
    let approved_ = false;
    server.use(
      http.get("*/v1/support-grants", () => HttpResponse.json([otherGrant])),
      http.post("*/v1/support-grants/*/approve", () => {
        approved_ = true;
        return HttpResponse.json(approved);
      }),
    );

    const user = userEvent.setup();
    renderWithQuery(
      <SupportShell
        session={adminSession}
        role="admin"
        onSignOut={vi.fn()}
        initialSection="admin-users"
      />,
    );

    await user.click(await screen.findByTestId("grant-approval-approve-grant-other"));
    await waitFor(() => expect(approved_).toBe(true));

    expect(screen.queryByTestId("grant-banner")).not.toBeInTheDocument();
    expect(screen.queryByTestId("support-shell-no-grant")).toBeInTheDocument();
  });

  it("renames the search section to metadata and still lands a #search hash on it", async () => {
    window.location.hash = "#search";
    server.use(
      http.get("*/v1/support-grants/mine", () => HttpResponse.json([])),
      http.get("*/v1/support-grants/*/metadata", () => HttpResponse.json(supportMetadata())),
    );

    renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialGrant={activeGrant()}
      />,
    );

    expect(screen.queryByTestId("nav-search")).not.toBeInTheDocument();
    expect(screen.getByTestId("nav-metadata")).toHaveAttribute("aria-current", "page");
    expect(await screen.findByTestId("support-metadata-panel")).toBeInTheDocument();
    expect(await screen.findByText("Customer One")).toBeInTheDocument();
  });

  it("shows a support session neither the approvals queue nor the admin link", async () => {
    server.use(http.get("*/v1/support-grants/mine", () => HttpResponse.json([])));

    renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialGrant={activeGrant()}
      />,
    );

    expect(screen.getByTestId("grant-request-form")).toBeInTheDocument();
    expect(screen.queryByTestId("grant-approval-queue")).not.toBeInTheDocument();
    expect(screen.queryByTestId("nav-admin-users")).not.toBeInTheDocument();
    expect(screen.queryByTestId("nav-incidents")).not.toBeInTheDocument();
  });

  it("gives an admin the admin link instead of the request form", async () => {
    renderWithQuery(
      <SupportShell
        session={adminSession}
        role="admin"
        onSignOut={vi.fn()}
        initialSection="grants"
      />,
    );

    expect(screen.queryByTestId("grant-request-form")).not.toBeInTheDocument();
    expect(screen.getByTestId("nav-admin-users")).toBeInTheDocument();
    expect(screen.getByTestId("admin-link")).toBeInTheDocument();
  });

  it("shows the incident console to an admin", async () => {
    renderWithQuery(
      <SupportShell
        session={adminSession}
        role="admin"
        onSignOut={vi.fn()}
        initialSection="incidents"
      />,
    );

    expect(screen.getByTestId("nav-incidents")).toHaveAttribute("aria-current", "page");
    expect(
      await screen.findByTestId("incident-console", {}, { timeout: 3_000 }),
    ).toBeInTheDocument();
  });

  it("loads the ticket queue on its section when a grant is active", async () => {
    server.use(
      http.get("*/v1/support-grants/mine", () => HttpResponse.json([])),
      http.get("*/v1/support/tickets", () => HttpResponse.json({ items: [ticketMetadata] })),
    );
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

  it("requires a grant for the ticket queue", async () => {
    server.use(http.get("*/v1/support-grants/mine", () => HttpResponse.json([])));
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
    server.use(http.get("*/v1/support-grants/mine", () => HttpResponse.json([])));
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
    server.use(
      http.get("*/v1/support-grants/mine", () => HttpResponse.json([])),
      http.get("*/v1/support-grants/*/metadata", () => HttpResponse.json(supportMetadata())),
    );
    const grant = activeGrant({ expiresAt: "2026-11-08T10:00:10.000Z" });
    const { client } = renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialGrant={grant}
        initialSection="metadata"
      />,
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(screen.queryByText(/available while a grant is active/i)).not.toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });

    expect(screen.getByText(/available while a grant is active/i)).toBeInTheDocument();
    expect(client.getQueryCache().getAll()).toHaveLength(0);
  });

  it("puts the sign-out control and section links in keyboard order", async () => {
    server.use(http.get("*/v1/support-grants/mine", () => HttpResponse.json([])));
    const user = userEvent.setup();
    renderWithQuery(<SupportShell session={supportSession} role="support" onSignOut={vi.fn()} />);

    await user.tab();
    expect(screen.getByTestId("support-shell-sign-out")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("nav-grants")).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("nav-metadata")).toHaveFocus();
  });

  it("falls back to the grant workspace when a support user types the admin hash", async () => {
    server.use(http.get("*/v1/support-grants/mine", () => HttpResponse.json([])));
    renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialSection="admin-users"
      />,
    );

    expect(screen.queryByTestId("nav-admin-users")).not.toBeInTheDocument();
    expect(screen.queryByTestId("admin-users-section")).not.toBeInTheDocument();
    expect(screen.getByTestId("grant-request-form")).toBeInTheDocument();
  });

  it("has no accessibility violations", async () => {
    server.use(
      http.get("*/v1/support-grants/mine", () => HttpResponse.json([])),
      http.get("*/v1/support-grants/*/metadata", () => HttpResponse.json(supportMetadata())),
    );
    const { container } = renderWithQuery(
      <SupportShell
        session={supportSession}
        role="support"
        onSignOut={vi.fn()}
        initialGrant={activeGrant()}
        initialSection="metadata"
      />,
    );
    await screen.findByTestId("support-metadata-panel");
    expect((await axe(container)).violations).toEqual([]);
  });
});
