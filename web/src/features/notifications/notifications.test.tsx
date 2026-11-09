import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import NotificationsPage from "./NotificationsPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";
import type { NotificationItem, NotificationPreferences } from "../../api/types";

const notification: NotificationItem = {
  id: "n1",
  type: "brief",
  title: "Daily brief ready",
  body: "Your daily brief for 8 November is ready.",
  createdAt: "2026-11-08T07:00:00Z",
  read: false,
};

const preferences: NotificationPreferences = {
  types: [
    { type: "brief", inApp: true, email: true },
    { type: "security", inApp: true, email: true, mandatory: true },
  ],
};

describe("notification feed and preferences", () => {
  it("renders the feed and saves a toggled preference", async () => {
    const savePreferences = vi.fn().mockResolvedValue(preferences);
    const user = userEvent.setup();
    renderWithProviders(
      <NotificationsPage
        notificationsOverride={[notification]}
        preferencesOverride={preferences}
        savePreferences={savePreferences}
      />,
      { route: "/notifications" },
    );

    expect(screen.getByTestId("notification-row")).toHaveTextContent("Daily brief ready");
    expect(screen.getByTestId("pref-brief-inapp")).toBeChecked();

    await user.click(screen.getByTestId("pref-brief-email"));
    await user.click(screen.getByTestId("notifications-save"));

    expect(savePreferences).toHaveBeenCalledWith({
      types: [
        { type: "brief", inApp: true, email: false },
        { type: "security", inApp: true, email: true, mandatory: true },
      ],
    });
    expect(await screen.findByTestId("notifications-saved")).toBeInTheDocument();
  });

  it("locks a mandatory security email channel", async () => {
    const savePreferences = vi.fn().mockResolvedValue(preferences);
    const user = userEvent.setup();
    renderWithProviders(
      <NotificationsPage
        notificationsOverride={[]}
        preferencesOverride={preferences}
        savePreferences={savePreferences}
      />,
      { route: "/notifications" },
    );

    expect(screen.getByTestId("pref-security-email")).toBeDisabled();
    expect(screen.getByTestId("pref-mandatory-lock")).toHaveTextContent(/cannot be disabled/i);

    await user.click(screen.getByTestId("notifications-save"));
    expect(savePreferences.mock.calls[0][0].types[1]).toMatchObject({
      type: "security",
      email: true,
    });
  });

  it("shows the empty feed state", () => {
    renderWithProviders(
      <NotificationsPage notificationsOverride={[]} preferencesOverride={preferences} />,
      { route: "/notifications" },
    );

    expect(screen.getByTestId("notifications-empty")).toHaveTextContent(/no notifications/i);
  });

  it("names the field when the gateway refuses a preference change", async () => {
    const savePreferences = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Validation failed",
        status: 422,
        correlationId: "corr-422",
        errors: [{ field: "types", reason: "mandatory channel missing" }],
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <NotificationsPage
        notificationsOverride={[]}
        preferencesOverride={preferences}
        savePreferences={savePreferences}
      />,
      { route: "/notifications" },
    );

    await user.click(screen.getByTestId("notifications-save"));

    expect(await screen.findByTestId("notifications-errors")).toHaveTextContent(
      "types: mandatory channel missing",
    );
  });
});
