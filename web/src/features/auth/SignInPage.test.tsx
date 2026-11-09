import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import SignInPage from "./SignInPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";
import type { SessionState } from "../../auth/session";

const signedOut: SessionState = { status: "signed-out", session: null };

describe("sign-in and sign-up", () => {
  it("keeps the typed values and rejects an invalid email locally", async () => {
    const signIn = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(
      <SignInPage sessionOverride={signedOut} signIn={signIn} />,
      { route: "/" },
    );

    await user.type(screen.getByTestId("signin-email"), "not-an-email");
    await user.type(screen.getByTestId("signin-password"), "short");
    await user.click(screen.getByTestId("signin-submit"));

    expect(screen.getByTestId("signin-field-errors")).toHaveTextContent(
      /valid email address/i,
    );
    expect(signIn).not.toHaveBeenCalled();
    expect((screen.getByTestId("signin-email") as HTMLInputElement).value).toBe(
      "not-an-email",
    );
  });

  it("submits a valid sign-in and shows a gateway refusal inline", async () => {
    const signIn = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Unauthorized",
        status: 401,
        detail: "Invalid login credentials",
        correlationId: "corr-401",
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <SignInPage sessionOverride={signedOut} signIn={signIn} />,
      { route: "/" },
    );

    await user.type(screen.getByTestId("signin-email"), "mai@example.com");
    await user.type(screen.getByTestId("signin-password"), "correct-horse");
    await user.click(screen.getByTestId("signin-submit"));

    expect(await screen.findByTestId("signin-error")).toHaveTextContent(
      "Invalid login credentials",
    );
    expect((screen.getByTestId("signin-email") as HTMLInputElement).value).toBe(
      "mai@example.com",
    );
  });

  it("requires a gate token for sign-up and passes it to Supabase", async () => {
    const signUp = vi.fn().mockResolvedValue(undefined);
    const createGate = vi.fn().mockResolvedValue({ gateToken: "gate-1" });
    const user = userEvent.setup();
    renderWithProviders(
      <SignInPage
        sessionOverride={signedOut}
        signUp={signUp}
        createGate={createGate}
      />,
      { route: "/" },
    );

    await user.click(screen.getByTestId("signup-toggle"));
    await user.type(screen.getByTestId("signin-email"), "mai@example.com");
    await user.type(screen.getByTestId("signin-password"), "correct-horse");
    await user.click(screen.getByTestId("signin-submit"));

    expect(screen.getByTestId("signin-field-errors")).toHaveTextContent(
      /required/i,
    );
    expect(createGate).not.toHaveBeenCalled();

    await user.type(screen.getByTestId("signup-gate-token"), "gate-1");
    await user.click(screen.getByTestId("signin-submit"));

    expect(createGate).toHaveBeenCalledOnce();
    expect(signUp).toHaveBeenCalledWith("mai@example.com", "correct-horse", "gate-1");
  });

  it("redirects an already signed-in visitor to the dashboard", () => {
    renderWithProviders(
      <SignInPage
        sessionOverride={{
          status: "signed-in",
          session: { access_token: "token-1" },
        }}
      />,
      { route: "/" },
    );
    expect(screen.queryByTestId("signin-form")).toBeNull();
  });
});
