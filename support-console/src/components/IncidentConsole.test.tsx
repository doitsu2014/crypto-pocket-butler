import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it } from "vitest";
import { activeIncident, renderWithQuery } from "../test/fixtures";
import { server } from "../test/server";
import { IncidentConsole } from "./IncidentConsole";

describe("IncidentConsole", () => {
  it("lets an admin declare and see an incident", async () => {
    const incident = activeIncident({ id: "incident-3", severity: "sev2", status: "declared" });
    server.use(
      http.post("*/v1/support/incidents", () => HttpResponse.json(incident, { status: 201 })),
    );
    renderWithQuery(<IncidentConsole role="admin" token="token-1" />);

    const user = userEvent.setup();
    await user.type(screen.getByTestId("incident-form-summary"), "Account sync delays");
    await user.type(screen.getByTestId("incident-form-components"), "account-sync");
    await user.click(screen.getByTestId("incident-form-submit"));

    expect(await screen.findByTestId("incident-row-incident-3")).toHaveTextContent(
      "Account sync delays",
    );
  });

  it("hides the console from a support user", () => {
    renderWithQuery(<IncidentConsole role="support" token="token-1" />);

    expect(screen.getByTestId("incident-console-admin-only")).toHaveTextContent(/admins only/i);
    expect(screen.queryByTestId("incident-form")).not.toBeInTheDocument();
  });

  it("shows existing incidents to an admin", () => {
    renderWithQuery(
      <IncidentConsole
        role="admin"
        token="token-1"
        initialIncidents={[activeIncident({ id: "incident-5" })]}
      />,
    );

    expect(screen.getByTestId("incident-row-incident-5")).toBeInTheDocument();
  });

  it("has no accessibility violations", async () => {
    const { container } = renderWithQuery(<IncidentConsole role="admin" token="token-1" />);
    expect((await axe(container)).violations).toEqual([]);
  });
});
