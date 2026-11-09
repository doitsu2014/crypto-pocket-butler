import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "jest-axe";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import { adminSession, customerSession, supportSession } from "../test/fixtures";
import { SupportSignInGate } from "./SupportSignInGate";

function renderGate(
  session = null as typeof supportSession | null,
  overrides: Partial<ComponentProps<typeof SupportSignInGate>> = {},
) {
  const onSignIn = vi.fn().mockResolvedValue(undefined);
  const onSignOut = vi.fn();
  const onReady = vi.fn();
  const result = render(
    <SupportSignInGate
      session={session}
      onSignIn={onSignIn}
      onSignOut={onSignOut}
      onReady={onReady}
      {...overrides}
    >
      {(activeSession, role) => (
        <p data-testid="shell">
          {role}:{activeSession.user.id}
        </p>
      )}
    </SupportSignInGate>,
  );
  return { ...result, onSignIn, onSignOut, onReady };
}

describe("SupportSignInGate", () => {
  it("shows a labelled sign-in form when there is no session", () => {
    renderGate(null);

    expect(screen.getByLabelText(/email/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument();
    expect(screen.getByTestId("sign-in-submit")).toHaveTextContent(/sign in/i);
    expect(screen.queryByTestId("shell")).not.toBeInTheDocument();
  });

  it("signs in and passes the credentials to the auth client", async () => {
    const user = userEvent.setup();
    const { onSignIn } = renderGate(null);

    await user.type(screen.getByLabelText(/email/i), "agent@example.test");
    await user.type(screen.getByLabelText(/password/i), "correct horse");
    await user.click(screen.getByTestId("sign-in-submit"));

    expect(onSignIn).toHaveBeenCalledWith("agent@example.test", "correct horse");
  });

  it("shows a sign-in failure without exposing the token", async () => {
    const user = userEvent.setup();
    renderGate(null, { onSignIn: vi.fn().mockRejectedValue(new Error("Invalid login credentials")) });

    await user.type(screen.getByLabelText(/email/i), "agent@example.test");
    await user.type(screen.getByLabelText(/password/i), "wrong");
    await user.click(screen.getByTestId("sign-in-submit"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/invalid login credentials/i);
  });

  it("admits a support user and reports the role", () => {
    const { onReady } = renderGate(supportSession);

    expect(screen.getByTestId("shell")).toHaveTextContent("support:agent-1");
    expect(onReady).toHaveBeenCalledWith("support");
  });

  it("admits an admin user and reports the role", () => {
    const { onReady } = renderGate(adminSession);

    expect(screen.getByTestId("shell")).toHaveTextContent("admin:admin-1");
    expect(onReady).toHaveBeenCalledWith("admin");
  });

  it("refuses a customer role and loads no data", () => {
    const { onReady } = renderGate(customerSession);

    expect(screen.getByRole("heading", { name: /not authorised/i })).toBeInTheDocument();
    expect(screen.queryByTestId("shell")).not.toBeInTheDocument();
    expect(onReady).not.toHaveBeenCalled();
  });

  it("refuses a session whose app_metadata has no support role", () => {
    renderGate({
      accessToken: "token",
      user: { id: "user-1", appMetadata: {} },
    });

    expect(screen.getByRole("heading", { name: /not authorised/i })).toBeInTheDocument();
  });

  it("puts the sign-in controls in keyboard order", async () => {
    const user = userEvent.setup();
    renderGate(null);

    await user.tab();
    expect(screen.getByLabelText(/email/i)).toHaveFocus();
    await user.tab();
    expect(screen.getByLabelText(/password/i)).toHaveFocus();
    await user.tab();
    expect(screen.getByTestId("sign-in-submit")).toHaveFocus();
  });

  it("has no accessibility violations on the sign-in screen", async () => {
    const { container } = renderGate(null);
    expect((await axe(container)).violations).toEqual([]);
  });
});
