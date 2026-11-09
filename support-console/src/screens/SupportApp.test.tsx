import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import type { SupportAuthClient } from "../auth/supabase";
import {
  adminSession,
  customerSession,
  renderWithQuery,
  supportSession,
} from "../test/fixtures";
import SupportApp from "./SupportApp";

const fakeAuthClient: SupportAuthClient = {
  signIn: vi.fn().mockResolvedValue(supportSession),
  signOut: vi.fn().mockResolvedValue(undefined),
};

describe("SupportApp", () => {
  it("tells the operator when sign-in is not configured", () => {
    renderWithQuery(<SupportApp />);

    expect(screen.getByRole("alert")).toHaveTextContent(/sign-in is not configured/i);
  });

  it("renders the shell for an existing support session", () => {
    renderWithQuery(<SupportApp initialSession={supportSession} authClient={fakeAuthClient} />);

    expect(screen.getByRole("heading", { name: /support console/i })).toBeInTheDocument();
    expect(screen.getByTestId("nav-grants")).toBeInTheDocument();
  });

  it("renders the shell for an existing admin session", () => {
    renderWithQuery(<SupportApp initialSession={adminSession} authClient={fakeAuthClient} />);

    expect(screen.getByTestId("nav-incidents")).toBeInTheDocument();
  });

  it("refuses a customer session", () => {
    renderWithQuery(<SupportApp initialSession={customerSession} authClient={fakeAuthClient} />);

    expect(screen.getByRole("heading", { name: /not authorised/i })).toBeInTheDocument();
  });

  it("signs in through the auth client and opens the shell", async () => {
    renderWithQuery(<SupportApp authClient={fakeAuthClient} />);

    const user = userEvent.setup();
    await user.type(screen.getByLabelText(/email/i), "agent@example.test");
    await user.type(screen.getByLabelText(/password/i), "correct horse");
    await user.click(screen.getByTestId("sign-in-submit"));

    expect(fakeAuthClient.signIn).toHaveBeenCalledWith("agent@example.test", "correct horse");
    expect(await screen.findByTestId("nav-grants")).toBeInTheDocument();
  });

  it("has no accessibility violations on the signed-in shell", async () => {
    const { container } = renderWithQuery(
      <SupportApp initialSession={supportSession} authClient={fakeAuthClient} />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
