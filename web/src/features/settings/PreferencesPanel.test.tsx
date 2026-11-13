import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoAxeViolations } from "../../test/axe";
import { problem, stubGateway } from "../../test/gateway";
import { renderWithProviders } from "../../test/render";
import type { EffectivePreferences } from "../../api/types";
import PreferencesPanel from "./PreferencesPanel";
import { SettingsNav } from "./SettingsNav";

const preferences: EffectivePreferences = {
  userId: "user-1",
  locale: { value: "en", source: "user" },
  baseCurrency: { value: "USD", source: "default" },
  timezone: { value: "Europe/Paris", source: "user" },
  notifications: {
    email: { value: true, source: "user" },
    push: { value: false, source: "default" },
  },
};

function renderPanel(ui = <PreferencesPanel />) {
  return renderWithProviders(ui, { route: "/settings/preferences" });
}

describe("PreferencesPanel", () => {
  it("sends only the changed fields on save", async () => {
    const patches: unknown[] = [];
    stubGateway({
      "GET /v1/me/preferences": () => ({ body: preferences }),
      "PUT /v1/me/preferences": ({ body }) => {
        patches.push(body);
        return {
          body: {
            ...preferences,
            locale: { value: "vi", source: "user" },
            notifications: {
              email: { value: true, source: "user" },
              push: { value: true, source: "user" },
            },
          },
        };
      },
    });
    const user = userEvent.setup();
    renderPanel();

    await user.selectOptions(await screen.findByTestId("preferences-locale"), "vi");
    await user.click(screen.getByTestId("preferences-push"));
    await user.click(screen.getByTestId("preferences-save"));

    expect(patches).toEqual([{ locale: "vi", notifications: { push: true } }]);
    expect(await screen.findByTestId("preferences-saved")).toHaveTextContent(/saved/i);
  });

  it("names each invalid field from the server's 422 answer", async () => {
    stubGateway({
      "GET /v1/me/preferences": () => ({ body: preferences }),
      "PUT /v1/me/preferences": () => ({
        status: 422,
        body: {
          type: "https://errors.cpb/identity/validation",
          title: "Validation failed",
          status: 422,
          detail: "server detail that must never be rendered",
          correlationId: "corr-422",
          errors: [
            { field: "locale", reason: "is not a supported language" },
            { field: "baseCurrency", reason: "is not a supported currency" },
            { field: "timezone", reason: "is not a supported timezone" },
            { field: "notifications.email", reason: "must be a boolean" },
          ],
        },
      }),
    });
    const user = userEvent.setup();
    renderPanel();

    await user.selectOptions(await screen.findByTestId("preferences-locale"), "vi");
    await user.click(screen.getByTestId("preferences-save"));

    expect(await screen.findByTestId("preferences-error-locale")).toHaveTextContent(
      /supported language/,
    );
    expect(screen.getByTestId("preferences-error-baseCurrency")).toHaveTextContent(
      /supported currency/,
    );
    expect(screen.getByTestId("preferences-error-timezone")).toHaveTextContent(
      /supported timezone/,
    );
    expect(screen.getByTestId("preferences-error-notifications.email")).toHaveTextContent(
      /must be a boolean/,
    );
    expect(screen.getByTestId("preferences-locale")).toHaveAccessibleDescription(
      /supported language/,
    );
    expect(screen.getByTestId("preferences-base-currency")).toHaveAccessibleDescription(
      /supported currency/,
    );
    expect(screen.getByTestId("preferences-timezone")).toHaveAccessibleDescription(
      /supported timezone/,
    );
    expect(screen.getByTestId("preferences-email")).toHaveAccessibleDescription(/must be a boolean/);
    expect(screen.queryByText(/server detail/)).not.toBeInTheDocument();
  });

  it("labels each value with its server source", async () => {
    stubGateway({ "GET /v1/me/preferences": () => ({ body: preferences }) });
    const { container } = renderPanel();

    expect(await screen.findByTestId("preferences-locale-source")).toHaveTextContent(
      /your choice/i,
    );
    expect(screen.getByTestId("preferences-base-currency-source")).toHaveTextContent(
      /service default/i,
    );
    await expectNoAxeViolations(container);
  });

  it("reports a read failure without blanking the navigation", async () => {
    stubGateway({
      "GET /v1/me/preferences": () => ({
        status: 503,
        body: problem(503, "server detail that must never be rendered"),
      }),
    });
    renderPanel(
      <>
        <SettingsNav />
        <PreferencesPanel />
      </>,
    );

    expect(await screen.findByTestId("preferences-error")).toBeInTheDocument();
    expect(screen.getByTestId("settings-nav-profile")).toHaveAttribute("href", "/settings/profile");
    await waitFor(() => expect(screen.queryByText(/server detail/)).not.toBeInTheDocument());
  });

  it("operates the timezone list from the keyboard", async () => {
    stubGateway({ "GET /v1/me/preferences": () => ({ body: preferences }) });
    const user = userEvent.setup();
    renderPanel();

    const input = await screen.findByTestId("preferences-timezone");
    await user.click(input);
    await user.clear(input);
    await user.type(input, "New");

    expect(await screen.findByTestId("preferences-timezone-list")).toBeInTheDocument();

    await user.keyboard("{ArrowDown}");
    const active = input.getAttribute("aria-activedescendant");
    expect(active).toMatch(/^preferences-timezone-option-/);

    await user.keyboard("{Enter}");
    expect(input).toHaveValue("America/New_York");
    expect(screen.queryByTestId("preferences-timezone-list")).not.toBeInTheDocument();
    expect(input).toHaveAttribute("aria-expanded", "false");

    await user.click(input);
    await user.keyboard("{Escape}");
    expect(screen.queryByTestId("preferences-timezone-list")).not.toBeInTheDocument();
  });

  it("switches state is written in text beside the control, never colour alone", async () => {
    stubGateway({ "GET /v1/me/preferences": () => ({ body: preferences }) });
    renderPanel();

    const email = await screen.findByTestId("preferences-email");
    expect(email).toHaveAttribute("role", "switch");
    expect(email).toHaveAttribute("aria-checked", "true");
    expect(screen.getByTestId("preferences-email-state")).toHaveTextContent(/on/i);
    expect(screen.getByTestId("preferences-push-state")).toHaveTextContent(/off/i);
  });
});
