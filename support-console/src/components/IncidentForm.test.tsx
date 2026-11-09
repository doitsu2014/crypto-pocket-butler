import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { activeIncident, problemDetails, renderWithQuery } from "../test/fixtures";
import { server } from "../test/server";
import { IncidentForm } from "./IncidentForm";

async function fillForm(options: { severity?: string; summary?: string; components?: string } = {}) {
  const user = userEvent.setup();
  if (options.severity) {
    await user.selectOptions(screen.getByTestId("incident-form-severity"), options.severity);
  }
  await user.type(
    screen.getByTestId("incident-form-summary"),
    options.summary ?? "Account sync delays for one venue",
  );
  await user.type(
    screen.getByTestId("incident-form-components"),
    options.components ?? "account-sync, market-data",
  );
  return user;
}

const declaredIncident = activeIncident({
  id: "incident-9",
  severity: "sev1",
  status: "declared",
  notification: { state: "sent" },
});

describe("IncidentForm", () => {
  it("declares an incident and shows the C9 notification outcome", async () => {
    let body: unknown;
    server.use(
      http.post("*/v1/support/incidents", async ({ request }) => {
        body = await request.json();
        return HttpResponse.json(declaredIncident, { status: 201 });
      }),
    );
    const onDeclared = vi.fn();
    renderWithQuery(<IncidentForm token="token-1" onDeclared={onDeclared} />);

    const user = await fillForm({ severity: "sev1" });
    await user.click(screen.getByTestId("incident-form-submit"));

    await waitFor(() => expect(onDeclared).toHaveBeenCalledWith(declaredIncident));
    expect(body).toEqual({
      severity: "sev1",
      summary: "Account sync delays for one venue",
      components: ["account-sync", "market-data"],
    });
    expect(await screen.findByText("Notification sent")).toBeInTheDocument();
  });

  it("shows the 15-minute SEV-1 notice requirement", async () => {
    renderWithQuery(<IncidentForm token="token-1" onDeclared={vi.fn()} />);

    await userEvent.setup().selectOptions(screen.getByTestId("incident-form-severity"), "sev1");

    expect(screen.getByTestId("incident-form-sev1-notice")).toHaveTextContent(
      /within 15 minutes/i,
    );
  });

  it("refuses a summary longer than 200 characters", async () => {
    server.use(http.post("*/v1/support/incidents", () => HttpResponse.json(declaredIncident)));
    renderWithQuery(<IncidentForm token="token-1" onDeclared={vi.fn()} />);

    const user = await fillForm({ summary: "a".repeat(201) });
    await user.click(screen.getByTestId("incident-form-submit"));

    expect(await screen.findByText(/200 characters or fewer/i)).toBeInTheDocument();
  });

  it("requires at least one affected component", async () => {
    renderWithQuery(<IncidentForm token="token-1" onDeclared={vi.fn()} />);

    const user = await fillForm({ components: "   " });
    await user.click(screen.getByTestId("incident-form-submit"));

    expect(await screen.findByText(/at least one affected component/i)).toBeInTheDocument();
  });

  it("surfaces a failed C9 notification without pretending it was sent", async () => {
    server.use(
      http.post("*/v1/support/incidents", () =>
        HttpResponse.json(
          activeIncident({
            id: "incident-10",
            notification: { state: "failed", detail: "Notification service unavailable" },
          }),
          { status: 201 },
        ),
      ),
    );
    renderWithQuery(<IncidentForm token="token-1" onDeclared={vi.fn()} />);

    const user = await fillForm();
    await user.click(screen.getByTestId("incident-form-submit"));

    expect(await screen.findByText("Notification failed")).toBeInTheDocument();
    expect(screen.getByText(/notification service unavailable/i)).toBeInTheDocument();
  });

  it("shows a pending notification while C9 is in flight", async () => {
    server.use(
      http.post("*/v1/support/incidents", () =>
        HttpResponse.json(
          activeIncident({ id: "incident-11", notification: { state: "pending" } }),
          { status: 201 },
        ),
      ),
    );
    renderWithQuery(<IncidentForm token="token-1" onDeclared={vi.fn()} />);

    const user = await fillForm();
    await user.click(screen.getByTestId("incident-form-submit"));

    expect(await screen.findByText("Notification pending")).toBeInTheDocument();
  });

  it("shows a server problem on a refused declaration", async () => {
    server.use(
      http.post("*/v1/support/incidents", () =>
        HttpResponse.json(problemDetails(403, "Admin only", "corr-403"), {
          status: 403,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    renderWithQuery(<IncidentForm token="token-1" onDeclared={vi.fn()} />);

    const user = await fillForm();
    await user.click(screen.getByTestId("incident-form-submit"));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(/admin only/i);
    expect(alert).toHaveTextContent("corr-403");
  });

  it("has no accessibility violations", async () => {
    const { container } = renderWithQuery(
      <IncidentForm token="token-1" onDeclared={vi.fn()} />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
