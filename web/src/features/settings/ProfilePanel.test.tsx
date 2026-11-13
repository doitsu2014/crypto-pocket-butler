import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { expectNoAxeViolations } from "../../test/axe";
import { problem, stubGateway } from "../../test/gateway";
import { renderWithProviders } from "../../test/render";
import type { ProfileView } from "../../api/types";
import ProfilePanel from "./ProfilePanel";

const profile: ProfileView = {
  userId: "user-1",
  displayName: "Mai",
  displayNameOverridden: false,
  providerDisplayName: "Mai",
  email: "mai@example.com",
  emailVerified: true,
  status: "active",
};

function renderPanel() {
  return renderWithProviders(<ProfilePanel />, { route: "/settings/profile" });
}

describe("ProfilePanel", () => {
  afterEach(() => {
    stubGateway({});
  });

  it("saves the trimmed name with the exact PATCH body and re-renders from the response", async () => {
    const requests: unknown[] = [];
    stubGateway({
      "GET /v1/me/profile": () => ({ body: profile }),
      "PATCH /v1/me/profile": ({ body }) => {
        requests.push(body);
        return {
          body: {
            ...profile,
            displayName: "Mai Anh",
            displayNameOverridden: true,
            providerDisplayName: "Mai",
          },
        };
      },
    });
    const user = userEvent.setup();
    renderPanel();

    const name = await screen.findByTestId("settings-display-name");
    await user.clear(name);
    await user.type(name, "  Mai Anh  ");
    await user.click(screen.getByTestId("settings-profile-save"));

    expect(requests).toEqual([{ displayName: "Mai Anh" }]);
    await waitFor(() => expect(screen.getByTestId("settings-display-name")).toHaveValue("Mai Anh"));
  });

  it("keeps the typed value and names the field when the server refuses with 422", async () => {
    stubGateway({
      "GET /v1/me/profile": () => ({ body: profile }),
      "PATCH /v1/me/profile": () => ({
        status: 422,
        body: {
          type: "https://errors.cpb/identity/validation",
          title: "Validation failed",
          status: 422,
          detail: "server detail that must never be rendered",
          correlationId: "corr-422",
          errors: [{ field: "displayName", reason: "is longer than 200 characters" }],
        },
      }),
    });
    const user = userEvent.setup();
    renderPanel();

    const name = await screen.findByTestId("settings-display-name");
    await user.clear(name);
    await user.type(name, "A very long name");
    await user.click(screen.getByTestId("settings-profile-save"));

    expect(await screen.findByTestId("settings-name-error")).toHaveTextContent(/200 characters/);
    expect(screen.getByTestId("settings-display-name")).toHaveValue("A very long name");
    expect(screen.queryByText(/server detail/)).not.toBeInTheDocument();
  });

  it("hides the provider-name reset until the name is overridden", async () => {
    stubGateway({ "GET /v1/me/profile": () => ({ body: profile }) });
    renderPanel();

    await screen.findByTestId("settings-display-name");
    expect(screen.queryByTestId("settings-reset-name")).not.toBeInTheDocument();
  });

  it("disables the reset with a hint when the provider supplied no name", async () => {
    stubGateway({
      "GET /v1/me/profile": () => ({
        body: { ...profile, displayNameOverridden: true, providerDisplayName: null },
      }),
    });
    renderPanel();

    expect(await screen.findByTestId("settings-reset-name")).toBeDisabled();
    expect(screen.getByTestId("settings-reset-hint")).toHaveTextContent(/no name/i);
  });

  it("renders the 409 hint when the provider name is no longer available", async () => {
    stubGateway({
      "GET /v1/me/profile": () => ({
        body: { ...profile, displayNameOverridden: true },
      }),
      "PATCH /v1/me/profile": () => ({
        status: 409,
        body: problem(409, "server detail that must never be rendered"),
      }),
    });
    const user = userEvent.setup();
    renderPanel();

    await user.click(await screen.findByTestId("settings-reset-name"));

    expect(await screen.findByTestId("settings-reset-conflict")).toHaveTextContent(
      /nothing was changed/i,
    );
    expect(screen.getByTestId("settings-display-name")).toHaveValue("Mai");
  });

  it("renders an em dash for a missing email and pairs the status with its label", async () => {
    stubGateway({
      "GET /v1/me/profile": () => ({ body: { ...profile, email: null, emailVerified: false } }),
    });
    renderPanel();

    expect(await screen.findByTestId("settings-email")).toHaveTextContent("—");
    expect(screen.getByTestId("status-pill")).toHaveTextContent(/active/i);
  });

  it("has no automated accessibility violations", async () => {
    stubGateway({ "GET /v1/me/profile": () => ({ body: profile }) });
    const { container } = renderPanel();

    await screen.findByTestId("settings-display-name");
    await expectNoAxeViolations(container);
  });
});
