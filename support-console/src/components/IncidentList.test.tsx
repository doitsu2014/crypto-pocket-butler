import { render, screen } from "@testing-library/react";
import { axe } from "jest-axe";
import { describe, expect, it } from "vitest";
import { activeIncident } from "../test/fixtures";
import { IncidentList } from "./IncidentList";

describe("IncidentList", () => {
  it("lists severity, status, summary, components, and the notification outcome", () => {
    render(
      <IncidentList
        incidents={[
          activeIncident({ id: "incident-1", notification: { state: "sent" } }),
          activeIncident({ id: "incident-2", severity: "sev3", status: "resolved" }),
        ]}
      />,
    );

    const first = screen.getByTestId("incident-row-incident-1");
    expect(first).toHaveTextContent("SEV-2");
    expect(first).toHaveTextContent("Investigating");
    expect(first).toHaveTextContent("Account sync delays for one venue");
    expect(first).toHaveTextContent("account-sync");
    expect(first).toHaveTextContent("Notification sent");

    const second = screen.getByTestId("incident-row-incident-2");
    expect(second).toHaveTextContent("SEV-3");
    expect(second).toHaveTextContent("Resolved");
  });

  it("shows an empty state", () => {
    render(<IncidentList incidents={[]} />);
    expect(screen.getByTestId("incident-list-empty")).toHaveTextContent(/no incidents/i);
  });

  it("does not present a resolved incident as active", () => {
    render(<IncidentList incidents={[activeIncident({ status: "resolved" })]} />);
    const row = screen.getByTestId("incident-row-incident-1");
    expect(row).toHaveTextContent("Resolved");
    expect(row).not.toHaveTextContent(/^Active$/);
  });

  it("has no accessibility violations", async () => {
    const { container } = render(<IncidentList incidents={[activeIncident()]} />);
    expect((await axe(container)).violations).toEqual([]);
  });
});
