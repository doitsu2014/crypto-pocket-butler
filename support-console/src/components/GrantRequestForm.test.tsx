import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { activeGrant, problemDetails, renderWithQuery } from "../test/fixtures";
import { server } from "../test/server";
import { GrantRequestForm } from "./GrantRequestForm";

const REASON = "Investigating a reported sync failure";

function renderForm(onCheckStatus = vi.fn()) {
  const onCreated = vi.fn();
  const result = renderWithQuery(
    <GrantRequestForm token="token-1" onCreated={onCreated} onCheckStatus={onCheckStatus} />,
  );
  return { ...result, onCreated, onCheckStatus };
}

async function fillValidForm(user: ReturnType<typeof userEvent.setup>) {
  await user.type(screen.getByTestId("grant-request-target"), "user-1");
  await user.type(screen.getByTestId("grant-request-reason"), REASON);
}

describe("GrantRequestForm", () => {
  it("posts one request, reports the 201 grant, shows a success line, and clears the fields", async () => {
    const grant = activeGrant({ status: "pending_approval" });
    let body: unknown;
    let requests = 0;
    server.use(
      http.post("*/v1/support-grants", async ({ request }) => {
        requests += 1;
        body = await request.json();
        return HttpResponse.json(grant, { status: 201 });
      }),
    );
    const { onCreated } = renderForm();

    const user = userEvent.setup();
    await fillValidForm(user);
    await user.click(screen.getByTestId("grant-request-submit"));

    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(grant));
    expect(requests).toBe(1);
    expect(body).toEqual({ userId: "user-1", reason: REASON });
    expect(screen.getByTestId("grant-request-success")).toHaveTextContent(
      /pending until a second approver accepts it/i,
    );
    expect(screen.getByTestId("grant-request-target")).toHaveValue("");
    expect(screen.getByTestId("grant-request-reason")).toHaveValue("");
  });

  it("disables Submit while the request is in flight", async () => {
    server.use(
      http.post("*/v1/support-grants", async () => {
        await new Promise((resolve) => setTimeout(resolve, 100));
        return HttpResponse.json(activeGrant({ status: "pending_approval" }), { status: 201 });
      }),
    );
    renderForm();

    const user = userEvent.setup();
    await fillValidForm(user);
    const submit = screen.getByTestId("grant-request-submit");
    await user.click(submit);

    expect(submit).toBeDisabled();
    expect(submit).toHaveTextContent(/requesting/i);
  });

  it("requires a target user id without calling the API", async () => {
    const requests: string[] = [];
    server.use(
      http.post("*/v1/support-grants", () => {
        requests.push("posted");
        return HttpResponse.json(activeGrant(), { status: 201 });
      }),
    );
    const { onCreated } = renderForm();

    const user = userEvent.setup();
    await user.type(screen.getByTestId("grant-request-reason"), REASON);
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByText(/enter the target user's id/i)).toBeInTheDocument();
    expect(requests).toHaveLength(0);
    expect(onCreated).not.toHaveBeenCalled();
  });

  it("requires a reason without calling the API", async () => {
    const requests: string[] = [];
    server.use(
      http.post("*/v1/support-grants", () => {
        requests.push("posted");
        return HttpResponse.json(activeGrant(), { status: 201 });
      }),
    );
    renderForm();

    const user = userEvent.setup();
    await user.type(screen.getByTestId("grant-request-target"), "user-1");
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByText(/enter a reason/i)).toBeInTheDocument();
    expect(requests).toHaveLength(0);
  });

  it("refuses a 501-character reason without calling the API", async () => {
    const requests: string[] = [];
    server.use(
      http.post("*/v1/support-grants", () => {
        requests.push("posted");
        return HttpResponse.json(activeGrant(), { status: 201 });
      }),
    );
    renderForm();

    const user = userEvent.setup();
    await user.type(screen.getByTestId("grant-request-target"), "user-1");
    fireEvent.change(screen.getByTestId("grant-request-reason"), {
      target: { value: "a".repeat(501) },
    });
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByText(/500 characters or fewer/i)).toBeInTheDocument();
    expect(requests).toHaveLength(0);
  });

  it("offers Check your current grant on a 409", async () => {
    server.use(
      http.post("*/v1/support-grants", () =>
        HttpResponse.json(problemDetails(409, "An open grant already exists", "corr-409"), {
          status: 409,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    const { onCheckStatus } = renderForm();

    const user = userEvent.setup();
    await fillValidForm(user);
    await user.click(screen.getByTestId("grant-request-submit"));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(/already have an open grant/i);
    const check = screen.getByTestId("grant-request-check-status");
    await user.click(check);
    expect(onCheckStatus).toHaveBeenCalledTimes(1);
  });

  it("names an unknown target user on a 404", async () => {
    server.use(
      http.post("*/v1/support-grants", () =>
        HttpResponse.json(problemDetails(404, "User not found", "corr-404"), {
          status: 404,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    renderForm();

    const user = userEvent.setup();
    await fillValidForm(user);
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/no user matches that id/i);
  });

  it("uses a neutral not-allowed line on a 403 and never the problem detail", async () => {
    server.use(
      http.post("*/v1/support-grants", () =>
        HttpResponse.json(
          {
            ...problemDetails(403, "Forbidden", "corr-403"),
            detail: "You lack the support role",
          },
          {
            status: 403,
            headers: { "Content-Type": "application/problem+json" },
          },
        ),
      ),
    );
    renderForm();

    const user = userEvent.setup();
    await fillValidForm(user);
    await user.click(screen.getByTestId("grant-request-submit"));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(/not allowed to request support access/i);
    expect(alert).not.toHaveTextContent(/lack the support role/i);
    expect(alert).not.toHaveTextContent("corr-403");
  });

  it("maps a 422 to per-field messages", async () => {
    server.use(
      http.post("*/v1/support-grants", () =>
        HttpResponse.json(
          {
            ...problemDetails(422, "Validation failed", "corr-422"),
            errors: [
              { field: "userId", reason: "The user id is not valid." },
              { field: "reason", reason: "The reason is not acceptable." },
            ],
          },
          {
            status: 422,
            headers: { "Content-Type": "application/problem+json" },
          },
        ),
      ),
    );
    renderForm();

    const user = userEvent.setup();
    await fillValidForm(user);
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByText("The user id is not valid.")).toBeInTheDocument();
    expect(screen.getByText("The reason is not acceptable.")).toBeInTheDocument();
    expect(screen.getByTestId("grant-request-target")).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByTestId("grant-request-reason")).toHaveAttribute("aria-invalid", "true");
  });

  it("shows the Retry-After seconds on a 429 and never retries by itself", async () => {
    let requests = 0;
    server.use(
      http.post("*/v1/support-grants", () => {
        requests += 1;
        return HttpResponse.json(problemDetails(429, "Too many requests", "corr-429"), {
          status: 429,
          headers: {
            "Content-Type": "application/problem+json",
            "Retry-After": "42",
          },
        });
      }),
    );
    renderForm();

    const user = userEvent.setup();
    await fillValidForm(user);
    await user.click(screen.getByTestId("grant-request-submit"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/try again in 42 seconds/i);
    expect(requests).toBe(1);
  });

  it("has no accessibility violations in the idle, error, and success states", async () => {
    const user = userEvent.setup();

    // Idle.
    const idle = renderForm();
    expect((await axe(idle.container)).violations).toEqual([]);
    idle.unmount();

    // Validation error.
    const invalid = renderForm();
    await user.click(screen.getByTestId("grant-request-submit"));
    await screen.findByText(/enter the target user's id/i);
    expect((await axe(invalid.container)).violations).toEqual([]);
    invalid.unmount();

    // Success.
    server.use(
      http.post("*/v1/support-grants", () =>
        HttpResponse.json(activeGrant({ status: "pending_approval" }), { status: 201 }),
      ),
    );
    const success = renderForm();
    await fillValidForm(user);
    await user.click(screen.getByTestId("grant-request-submit"));
    await screen.findByTestId("grant-request-success");
    expect((await axe(success.container)).violations).toEqual([]);
  });
});
