import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import OnboardingPage from "./OnboardingPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";

describe("onboarding", () => {
  it("does not pre-tick consent and requires a legal basis when granting", async () => {
    const postConsent = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(<OnboardingPage postConsent={postConsent} />, {
      route: "/onboarding",
    });

    expect(screen.getByTestId("consent-analytics")).not.toBeChecked();
    expect(screen.getByTestId("consent-marketing")).not.toBeChecked();

    await user.click(screen.getByTestId("consent-analytics"));
    await user.click(screen.getByTestId("onboarding-consent-next"));

    expect(screen.getByTestId("onboarding-errors")).toHaveTextContent(
      /legal basis/i,
    );
    expect(postConsent).not.toHaveBeenCalled();
  });

  it("advances through consent and preferences and submits the first wallet", async () => {
    const postConsent = vi.fn().mockResolvedValue(undefined);
    const updatePreferences = vi.fn().mockResolvedValue({
      baseCurrency: "USD",
      locale: "en",
      timezone: "UTC",
    });
    const addWallet = vi.fn().mockResolvedValue({});
    const user = userEvent.setup();
    renderWithProviders(
      <OnboardingPage
        postConsent={postConsent}
        updatePreferences={updatePreferences}
        addWallet={addWallet}
      />,
      { route: "/onboarding" },
    );

    await user.click(screen.getByTestId("onboarding-consent-next"));
    expect(await screen.findByTestId("onboarding-preferences")).toBeInTheDocument();
    await user.click(screen.getByTestId("onboarding-preferences-next"));

    expect(await screen.findByTestId("onboarding-account")).toBeInTheDocument();
    await user.type(screen.getByTestId("onboarding-address"), "0xabc123");
    await user.click(screen.getByTestId("onboarding-account-submit"));

    expect(addWallet).toHaveBeenCalledWith({
      venue: "ethereum",
      label: "My wallet",
      address: "0xabc123",
    });
  });

  it("keeps the preferences step open and shows field errors on a 422", async () => {
    const updatePreferences = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Validation failed",
        status: 422,
        correlationId: "corr-422",
        errors: [{ field: "timezone", reason: "Unknown timezone" }],
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <OnboardingPage
        postConsent={vi.fn().mockResolvedValue(undefined)}
        updatePreferences={updatePreferences}
      />,
      { route: "/onboarding" },
    );

    await user.click(screen.getByTestId("onboarding-consent-next"));
    await screen.findByTestId("onboarding-preferences");
    await user.click(screen.getByTestId("onboarding-preferences-next"));

    expect(await screen.findByTestId("onboarding-errors")).toHaveTextContent(
      "timezone: Unknown timezone",
    );
    expect(screen.getByTestId("onboarding-preferences")).toBeInTheDocument();
  });
});
