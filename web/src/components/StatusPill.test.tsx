import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { formatDateTime } from "../i18n";
import { expectNoAxeViolations } from "../test/axe";
import { renderWithProviders } from "../test/render";
import type { UserStatus } from "../api/types";
import { StatusPill } from "./index";

const cases: Array<[UserStatus, RegExp, string]> = [
  ["pending", /pending/i, "ring"],
  ["active", /active/i, "dot"],
  ["disabled", /disabled/i, "square"],
  ["pending_deletion", /pending deletion/i, "square-outline"],
  ["deleted", /deleted/i, "cross"],
];

describe("StatusPill", () => {
  it.each(cases)("renders %s as a label plus its own shape", (status, label, shape) => {
    renderWithProviders(<StatusPill status={status} />);

    expect(screen.getByTestId("status-pill")).toHaveTextContent(label);
    expect(screen.getByTestId("status-pill-shape")).toHaveAttribute("data-shape", shape);
  });

  it("carries the restore date as text beside the pending-deletion pill", () => {
    const restoreDate = "2026-12-01T09:00:00Z";
    renderWithProviders(<StatusPill status="pending_deletion" restoreDate={restoreDate} />);

    expect(screen.getByTestId("status-pill-restore")).toHaveTextContent(
      formatDateTime(restoreDate, "en", "UTC"),
    );
    expect(screen.getByTestId("status-pill")).toHaveTextContent(/pending deletion/i);
    expect(screen.getByTestId("status-pill")).not.toHaveTextContent(
      formatDateTime(restoreDate, "en", "UTC"),
    );
  });

  it("omits the restore line for every other status", () => {
    renderWithProviders(<StatusPill status="active" restoreDate="2026-12-01T09:00:00Z" />);

    expect(screen.queryByTestId("status-pill-restore")).not.toBeInTheDocument();
  });

  it("has no automated accessibility violations", async () => {
    const { container } = renderWithProviders(
      <div>
        {cases.map(([status]) => (
          <StatusPill key={status} status={status} />
        ))}
      </div>,
    );

    await expectNoAxeViolations(container);
  });
});
