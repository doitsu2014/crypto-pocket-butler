import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import type { AdminUserRecord } from "../../api/types";
import { problemDetails, renderWithQuery } from "../../test/fixtures";
import { server } from "../../test/server";
import { RoleChangeForm } from "./RoleChangeForm";

function record(overrides: Partial<AdminUserRecord> = {}): AdminUserRecord {
  return {
    id: "user-1",
    displayName: "Test User",
    email: "user@example.com",
    emailVerified: true,
    status: "active",
    role: "admin",
    createdAt: "2026-11-01T10:00:00.000Z",
    updatedAt: "2026-11-02T10:00:00.000Z",
    ...overrides,
  };
}

function renderForm(
  overrides: Partial<AdminUserRecord> = {},
  signedInEmail = "admin@example.test",
) {
  return renderWithQuery(
    <RoleChangeForm
      record={record(overrides)}
      signedInEmail={signedInEmail}
      token="admin-token"
      onChanged={vi.fn()}
    />,
  );
}

function problem(status: number, title: string, correlationId = "corr-role", errors?: unknown[]) {
  return HttpResponse.json(
    { ...problemDetails(status, title, correlationId), ...(errors ? { errors } : {}) },
    { status, headers: { "Content-Type": "application/problem+json" } },
  );
}

describe("RoleChangeForm", () => {
  it("changes the role, states the transition and that the change is audited", async () => {
    const user = userEvent.setup();
    let seenBody: unknown;
    const onChanged = vi.fn();
    server.use(
      http.put("*/v1/admin/users/user-1/role", async ({ request }) => {
        seenBody = await request.json();
        return HttpResponse.json({ userId: "user-1", previousRole: "admin", newRole: "support" });
      }),
    );
    renderWithQuery(
      <RoleChangeForm
        record={record()}
        signedInEmail="admin@example.test"
        token="admin-token"
        onChanged={onChanged}
      />,
    );

    await user.selectOptions(screen.getByTestId("admin-role-select"), "support");
    await user.type(screen.getByTestId("admin-role-reason"), "Moved to the support team");
    await user.click(screen.getByTestId("admin-role-submit"));

    await waitFor(() =>
      expect(screen.getByTestId("admin-role-result")).toHaveTextContent("admin → support"),
    );
    expect(screen.getByTestId("admin-role-result")).toHaveTextContent("This change is audited.");
    expect(seenBody).toEqual({ newRole: "support", reason: "Moved to the support team" });
    await waitFor(() =>
      expect(onChanged).toHaveBeenCalledWith({
        userId: "user-1",
        previousRole: "admin",
        newRole: "support",
      }),
    );
  });

  it("maps a 422 to the newRole and reason fields", async () => {
    const user = userEvent.setup();
    server.use(
      http.put("*/v1/admin/users/user-1/role", () =>
        problem(422, "Validation failed", "corr-422", [
          { field: "newRole", reason: "is not an assignable role" },
          { field: "reason", reason: "must be 1-500 characters" },
        ]),
      ),
    );
    renderForm();

    await user.selectOptions(screen.getByTestId("admin-role-select"), "support");
    await user.type(screen.getByTestId("admin-role-reason"), "x");
    await user.click(screen.getByTestId("admin-role-submit"));

    await waitFor(() =>
      expect(screen.getByTestId("admin-role-error")).toHaveTextContent("is not an assignable role"),
    );
    expect(screen.getByTestId("admin-role-reason-error")).toHaveTextContent(
      "must be 1-500 characters",
    );
  });

  it("renders the session line with the correlation id for a 403", async () => {
    const user = userEvent.setup();
    server.use(
      http.put("*/v1/admin/users/user-1/role", () => problem(403, "Forbidden", "corr-403")),
    );
    renderForm();

    await user.selectOptions(screen.getByTestId("admin-role-select"), "support");
    await user.type(screen.getByTestId("admin-role-reason"), "Moved to support");
    await user.click(screen.getByTestId("admin-role-submit"));

    const alert = await screen.findByTestId("admin-role-error");
    expect(alert).toHaveTextContent("Your session is not an administrator.");
    expect(alert).toHaveTextContent("corr-403");
  });

  it("does not claim success on a 503 and shows the correlation id", async () => {
    const user = userEvent.setup();
    server.use(
      http.put("*/v1/admin/users/user-1/role", () =>
        problem(503, "Service unavailable", "corr-503"),
      ),
    );
    renderForm();

    await user.selectOptions(screen.getByTestId("admin-role-select"), "support");
    await user.type(screen.getByTestId("admin-role-reason"), "Moved to support");
    await user.click(screen.getByTestId("admin-role-submit"));

    const alert = await screen.findByTestId("admin-role-error");
    expect(alert).toHaveTextContent("corr-503");
    expect(screen.queryByTestId("admin-role-result")).not.toBeInTheDocument();
    expect(screen.queryByText("This change is audited.")).not.toBeInTheDocument();
  });

  it("disables submit until the role differs and the reason is present", async () => {
    const user = userEvent.setup();
    renderForm();

    const submit = screen.getByTestId("admin-role-submit");
    expect(submit).toBeDisabled();

    await user.type(screen.getByTestId("admin-role-reason"), "A reason");
    expect(submit).toBeDisabled();

    await user.selectOptions(screen.getByTestId("admin-role-select"), "support");
    expect(submit).toBeEnabled();
  });

  it("warns only when the record is the signed-in account", async () => {
    const user = userEvent.setup();
    const { unmount } = renderForm({ email: "admin@example.test" }, "admin@example.test");
    expect(screen.getByTestId("admin-role-self-warning")).toHaveTextContent(
      "This is the account you are signed in with.",
    );
    unmount();

    renderForm({ email: "user@example.com" }, "admin@example.test");
    expect(screen.queryByTestId("admin-role-self-warning")).not.toBeInTheDocument();
    await user.tab();
  });

  it("has no accessibility violations", async () => {
    const { container } = renderForm();
    expect((await axe(container)).violations).toEqual([]);
  });
});
