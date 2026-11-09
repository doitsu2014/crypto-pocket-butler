import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import VerifyEmailPage from "./VerifyEmailPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";

describe("email verification", () => {
  it("resends to a valid address and confirms", async () => {
    const resend = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderWithProviders(
      <VerifyEmailPage initialEmail="mai@example.com" resend={resend} />,
    );

    await user.click(screen.getByTestId("verify-resend"));

    expect(resend).toHaveBeenCalledWith("mai@example.com");
    expect(await screen.findByTestId("verify-sent")).toBeInTheDocument();
  });

  it("rejects an invalid address before calling the gateway", async () => {
    const resend = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(<VerifyEmailPage initialEmail="bad" resend={resend} />);

    await user.click(screen.getByTestId("verify-resend"));

    expect(screen.getByTestId("verify-error")).toHaveTextContent(
      /valid email address/i,
    );
    expect(resend).not.toHaveBeenCalled();
  });

  it("shows the retry-after label when the gateway rate-limits the resend", async () => {
    const resend = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Too many requests",
        status: 429,
        detail: "Retry after 60 seconds",
        correlationId: "corr-429",
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <VerifyEmailPage initialEmail="mai@example.com" resend={resend} />,
    );

    await user.click(screen.getByTestId("verify-resend"));

    expect(await screen.findByTestId("verify-retry-after")).toHaveTextContent(
      "Retry after 60 seconds",
    );
  });
});
