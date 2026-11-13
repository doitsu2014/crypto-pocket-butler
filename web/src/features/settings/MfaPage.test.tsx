import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import MfaPage from "./MfaPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";
import type { Profile } from "../../api/types";

const profile: Profile = {
  userId: "user-1",
  email: "mai@example.com",
  displayName: "Mai",
  mfaRequired: true,
  mfaEnrolled: false,
};

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
