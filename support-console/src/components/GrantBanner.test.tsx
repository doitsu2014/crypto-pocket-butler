import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { activeGrant, expiredGrant, pendingGrant } from "../test/fixtures";
import { GrantBanner } from "./GrantBanner";

const NOW = Date.parse("2026-11-08T10:00:00.000Z");

describe("GrantBanner", () => {
  it("shows an active grant with its remaining minutes and the metadata-only reminder", () => {
    const grant = activeGrant({
      requestedAt: "2026-11-08T09:00:00.000Z",
      expiresAt: "2026-11-08T10:42:00.000Z",
    });
    render(<GrantBanner grant={grant} now={NOW} onCheckStatus={vi.fn()} onRequestNew={vi.fn()} />);

    const banner = screen.getByTestId("grant-banner");
    expect(banner).toHaveTextContent("Active");
    expect(banner).toHaveTextContent("42 minutes left");
    expect(banner).toHaveTextContent(/every metadata read is recorded/i);
    expect(banner).toHaveTextContent(/metadata only/i);
    expect(banner).toHaveTextContent("user-1");
    expect(banner).toHaveTextContent("agent-2");
  });

  it("shows pending approval without claiming access", () => {
    render(
      <GrantBanner
        grant={pendingGrant()}
        now={NOW}
        onCheckStatus={vi.fn()}
        onRequestNew={vi.fn()}
      />,
    );

    const banner = screen.getByTestId("grant-banner");
    expect(banner).toHaveTextContent("Pending approval");
    expect(banner).toHaveTextContent(/second approver/i);
    expect(banner).not.toHaveTextContent(/minutes left/i);
    expect(banner).not.toHaveTextContent(/metadata read is recorded/i);
  });

  it("ends access at expiresAt even when the stored status still says active", () => {
    const grant = activeGrant({
      requestedAt: "2026-11-08T09:00:00.000Z",
      expiresAt: "2026-11-08T09:59:00.000Z",
    });
    render(<GrantBanner grant={grant} now={NOW} onCheckStatus={vi.fn()} onRequestNew={vi.fn()} />);

    const banner = screen.getByTestId("grant-banner");
    expect(banner).toHaveTextContent("Expired");
    expect(banner).toHaveTextContent(/access ended at/i);
    expect(banner).not.toHaveTextContent(/minutes left/i);
  });

  it("shows a revoked grant as ended", () => {
    render(
      <GrantBanner
        grant={activeGrant({ status: "revoked" })}
        now={NOW}
        onCheckStatus={vi.fn()}
        onRequestNew={vi.fn()}
      />,
    );

    const banner = screen.getByTestId("grant-banner");
    expect(banner).toHaveTextContent("Revoked");
    expect(banner).toHaveTextContent(/revoked/i);
  });

  it("shows an already-expired grant with the ended copy", () => {
    render(
      <GrantBanner
        grant={expiredGrant()}
        now={Date.now()}
        onCheckStatus={vi.fn()}
        onRequestNew={vi.fn()}
      />,
    );

    expect(screen.getByTestId("grant-banner")).toHaveTextContent(/access ended at/i);
  });

  it("wires the check-status and request-new callbacks", async () => {
    const user = userEvent.setup();
    const onCheckStatus = vi.fn();
    const onRequestNew = vi.fn();
    render(
      <GrantBanner
        grant={activeGrant()}
        now={NOW}
        onCheckStatus={onCheckStatus}
        onRequestNew={onRequestNew}
      />,
    );

    await user.click(screen.getByTestId("grant-banner-check-status"));
    await user.click(screen.getByTestId("grant-banner-request-new"));

    expect(onCheckStatus).toHaveBeenCalledTimes(1);
    expect(onRequestNew).toHaveBeenCalledTimes(1);
  });

  it("has no accessibility violations", async () => {
    const { container } = render(
      <GrantBanner
        grant={activeGrant()}
        now={NOW}
        onCheckStatus={vi.fn()}
        onRequestNew={vi.fn()}
      />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
