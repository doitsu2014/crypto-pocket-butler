import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { activeGrant, problemDetails } from "../test/fixtures";
import { renderWithQuery } from "../test/fixtures";
import { server } from "../test/server";
import { GrantRequestForm } from "./GrantRequestForm";

const REASON = "Investigating a reported sync failure";

async function fillValidForm(overrides: { reason?: string; duration?: string; approver?: string } = {}) {
  const user = userEvent.setup();
  await user.type(screen.getByTestId("grant-request-target"), "user-1");
  await user.clear(screen.getByTestId("grant-request-reason"));
  await user.type(screen.getByTestId("grant-request-reason"), overrides.reason ?? REASON);
  await user.clear(screen.getByTestId("grant-request-duration"));
  await user.type(screen.getByTestId("grant-request-duration"), overrides.duration ?? "60");
  await user.type(screen.getByTestId("grant-request-approver"), overrides.approver ?? "agent-2");
  return user;
}

describe("GrantRequestForm", () => {
  it("posts a valid request and reports the created grant", async () => {
    const grant = activeGrant({ status: "pending_approval" });
    let body: unknown;
    server.use(
      http.post("*/v1/support-grants", async ({ request }) => {
        body = await request.json();
        return HttpResponse.json(grant, { status: 201 });
      }),
    );
    const onCreated = vi.fn();
    renderWithQuery(
      <GrantRequestForm requesterId="agent-1" token="token-1" onCreated={onCreated} />,
    );

    const user = await fillValidForm();
    await user.click(screen.getByTestId("grant-request-submit"));

    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(grant));
    expect(body).toEqual({
      userId: "user-1",
      reason: REASON,
      durationMinutes: 60,
      approverId: "agent-2",
    });
  });

  it("refuses a reason shorter than 20 characters without calling the API", async () => {
    const onCreated = vi.fn();
    renderWithQuery(
      <GrantRequestForm requesterId="agent-1" token="token-1" onCreated={onCreated} />,
    );

    const user = await fillValidForm({ reason: "too short" });
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByText(/at least 20 characters/i)).toBeInTheDocument();
    expect(onCreated).not.toHaveBeenCalled();
  });

  it("refuses a duration of zero", async () => {
    const onCreated = vi.fn();
    renderWithQuery(
      <GrantRequestForm requesterId="agent-1" token="token-1" onCreated={onCreated} />,
    );

    const user = await fillValidForm({ duration: "0" });
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByText(/between 1 and 480/i)).toBeInTheDocument();
    expect(onCreated).not.toHaveBeenCalled();
  });

  it("refuses a duration above 480 minutes", async () => {
    renderWithQuery(<GrantRequestForm requesterId="agent-1" token="token-1" onCreated={vi.fn()} />);

    const user = await fillValidForm({ duration: "481" });
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByText(/between 1 and 480/i)).toBeInTheDocument();
  });

  it("refuses the requester as their own second approver", async () => {
    renderWithQuery(<GrantRequestForm requesterId="agent-1" token="token-1" onCreated={vi.fn()} />);

    const user = await fillValidForm({ approver: "agent-1" });
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByText(/must differ from the requester/i)).toBeInTheDocument();
  });

  it("shows the server problem and correlation id on a refused request", async () => {
    server.use(
      http.post("*/v1/support-grants", () =>
        HttpResponse.json(problemDetails(422, "Grant request refused", "corr-422"), {
          status: 422,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    renderWithQuery(<GrantRequestForm requesterId="agent-1" token="token-1" onCreated={vi.fn()} />);

    const user = await fillValidForm();
    await user.click(screen.getByTestId("grant-request-submit"));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(/grant request refused/i);
    expect(alert).toHaveTextContent("corr-422");
  });

  it("shows a network failure without losing the entered values", async () => {
    server.use(http.post("*/v1/support-grants", () => HttpResponse.error()));
    renderWithQuery(<GrantRequestForm requesterId="agent-1" token="token-1" onCreated={vi.fn()} />);

    const user = await fillValidForm();
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/gateway/i);
    expect(screen.getByTestId("grant-request-target")).toHaveValue("user-1");
  });

  it("has no accessibility violations", async () => {
    const { container } = renderWithQuery(
      <GrantRequestForm requesterId="agent-1" token="token-1" onCreated={vi.fn()} />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
