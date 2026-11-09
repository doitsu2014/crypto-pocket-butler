import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { activeGrant, expiredGrant, pendingGrant } from "../test/fixtures";
import { GrantBanner } from "./GrantBanner";

describe("GrantBanner", () => {
  it("shows the reason, status, expiry, and approver for an active grant", () => {
    const grant = activeGrant({ expiresAt: "2026-11-08T12:00:00.000Z" });
    render(
      <GrantBanner grant={grant} now={Date.parse("2026-11-08T10:00:00.000Z")} onRequestNew={vi.fn()} />,
    );

    expect(screen.getByTestId("grant-banner")).toHaveTextContent(
      "Investigating a reported sync failure",
    );
    expect(screen.getByTestId("grant-banner")).toHaveTextContent("Active");
    expect(screen.getByTestId("grant-banner")).toHaveTextContent(/access ends at/i);
    expect(screen.getByTestId("grant-banner")).toHaveTextContent("agent-2");
  });

  it("shows pending approval without claiming access", () => {
    render(
      <GrantBanner grant={pendingGrant()} now={Date.parse("2026-11-08T10:00:00.000Z")} onRequestNew={vi.fn()} />,
    );

    expect(screen.getByTestId("grant-banner")).toHaveTextContent("Pending approval");
    expect(screen.getByTestId("grant-banner")).toHaveTextContent(/second approver/i);
    expect(screen.getByTestId("grant-banner")).not.toHaveTextContent(/access ends at/i);
  });

  it("shows access ended for an expired grant and offers a new grant", async () => {
    const user = userEvent.setup();
    const onRequestNew = vi.fn();
    render(
      <GrantBanner grant={expiredGrant()} now={Date.now()} onRequestNew={onRequestNew} />,
    );

    expect(screen.getByTestId("grant-banner")).toHaveTextContent(/access ended at/i);
    await user.click(screen.getByTestId("grant-banner-request-new"));
    expect(onRequestNew).toHaveBeenCalled();
  });

  it("shows a revoked grant as ended", () => {
    render(
      <GrantBanner
        grant={activeGrant({ status: "revoked" })}
        now={Date.now()}
        onRequestNew={vi.fn()}
      />,
    );

    expect(screen.getByTestId("grant-banner")).toHaveTextContent("Revoked");
    expect(screen.getByTestId("grant-banner")).toHaveTextContent(/revoked/i);
  });

  it("has no accessibility violations", async () => {
    const { container } = render(
      <GrantBanner grant={activeGrant()} now={Date.now()} onRequestNew={vi.fn()} />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
