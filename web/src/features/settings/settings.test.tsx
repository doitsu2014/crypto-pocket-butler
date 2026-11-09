import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import MfaPage from "./MfaPage";
import SettingsPage from "./SettingsPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";
import type { Profile, SessionInfo } from "../../api/types";

const profile: Profile = {
  userId: "user-1",
  email: "mai@example.com",
  displayName: "Mai",
  mfaRequired: true,
  mfaEnrolled: false,
};

const sessions: SessionInfo[] = [
  {
    id: "s1",
    device: "MacBook · Chrome",
    location: "Ho Chi Minh City",
    lastActiveAt: "2026-11-08T10:00:00Z",
    current: true,
  },
  {
    id: "s2",
    device: "iPhone · Safari",
    location: "Ha Noi",
    lastActiveAt: "2026-11-07T18:00:00Z",
    current: false,
  },
];

describe("settings", () => {
  it("shows the profile, the MFA requirement and the current session", () => {
    renderWithProviders(<SettingsPage profileOverride={profile} sessionsOverride={sessions} />, {
      route: "/settings",
    });

    expect(screen.getByTestId("settings-profile-email")).toHaveTextContent("mai@example.com");
    expect(screen.getByTestId("settings-mfa-status")).toHaveTextContent(/enrol/i);
    expect(screen.getByTestId("settings-mfa-link")).toHaveAttribute("href", "/settings/mfa");
    expect(screen.getByTestId("settings-current")).toHaveTextContent(/this device/i);
    expect(screen.getAllByTestId("settings-session-row")).toHaveLength(2);
  });

  it("saves the display name and reports success", async () => {
    const updateProfile = vi.fn().mockResolvedValue({
      ...profile,
      displayName: "Mai Anh",
    });
    const user = userEvent.setup();
    renderWithProviders(
      <SettingsPage
        profileOverride={profile}
        sessionsOverride={sessions}
        updateProfile={updateProfile}
      />,
      { route: "/settings" },
    );

    const name = screen.getByTestId("settings-display-name");
    await user.clear(name);
    await user.type(name, "Mai Anh");
    await user.click(screen.getByTestId("settings-profile-save"));

    expect(updateProfile).toHaveBeenCalledWith({ displayName: "Mai Anh" });
    expect(await screen.findByTestId("settings-saved")).toBeInTheDocument();
  });

  it("revokes another session and removes its row", async () => {
    const revokeSession = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderWithProviders(
      <SettingsPage
        profileOverride={profile}
        sessionsOverride={sessions}
        revokeSession={revokeSession}
      />,
      { route: "/settings" },
    );

    await user.click(screen.getByTestId("settings-revoke-s2"));

    expect(revokeSession).toHaveBeenCalledWith("s2");
    expect(screen.getAllByTestId("settings-session-row")).toHaveLength(1);
  });

  it("keeps the session and surfaces the refusal when revocation fails", async () => {
    const revokeSession = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Forbidden",
        status: 403,
        correlationId: "corr-403",
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <SettingsPage
        profileOverride={profile}
        sessionsOverride={sessions}
        revokeSession={revokeSession}
      />,
      { route: "/settings" },
    );

    await user.click(screen.getByTestId("settings-revoke-s2"));

    expect(await screen.findByTestId("error-panel")).toBeInTheDocument();
    expect(screen.getAllByTestId("settings-session-row")).toHaveLength(2);
  });

  it("names the field when a profile save is refused", async () => {
    const updateProfile = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Validation failed",
        status: 422,
        correlationId: "corr-422",
        errors: [{ field: "displayName", reason: "is too long" }],
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <SettingsPage
        profileOverride={profile}
        sessionsOverride={sessions}
        updateProfile={updateProfile}
      />,
      { route: "/settings" },
    );

    await user.click(screen.getByTestId("settings-profile-save"));

    expect(await screen.findByTestId("settings-errors")).toHaveTextContent(
      "displayName: is too long",
    );
  });
});

describe("MFA enrolment", () => {
  it("shows the enrolment secret, then the recovery codes once", async () => {
    const enrol = vi.fn().mockResolvedValue({
      secret: "JBSWY3DPEHPK3PXP",
      otpauthUrl: "otpauth://totp/crypto-pocket-butler?secret=JBSWY3DPEHPK3PXP",
    });
    const recovery = vi.fn().mockResolvedValue({
      codes: ["ABCD-EFGH", "IJKL-MNOP"],
    });
    const user = userEvent.setup();
    renderWithProviders(<MfaPage profileOverride={profile} enrol={enrol} recovery={recovery} />, {
      route: "/settings/mfa",
    });

    await user.click(screen.getByTestId("mfa-enrol"));

    expect(await screen.findByTestId("mfa-secret")).toHaveTextContent("JBSWY3DPEHPK3PXP");

    await user.click(screen.getByTestId("mfa-recovery"));

    expect(await screen.findByTestId("mfa-recovery-codes")).toHaveTextContent("ABCD-EFGH");
    expect(screen.getByTestId("mfa-recovery-once")).toHaveTextContent(/once/i);
  });

  it("declines into a limited session", async () => {
    const decline = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(<MfaPage profileOverride={profile} decline={decline} />, {
      route: "/settings/mfa",
    });

    await user.click(screen.getByTestId("mfa-decline"));

    expect(decline).toHaveBeenCalledOnce();
    expect(await screen.findByTestId("mfa-declined")).toBeInTheDocument();
  });

  it("shows the gateway refusal when enrolment fails", async () => {
    const enrol = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Configuration failed",
        status: 500,
        correlationId: "corr-500",
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<MfaPage profileOverride={profile} enrol={enrol} />, {
      route: "/settings/mfa",
    });

    await user.click(screen.getByTestId("mfa-enrol"));

    expect(await screen.findByTestId("error-panel")).toBeInTheDocument();
  });
});
