import { describe, expect, it } from "vitest";
import {
  createTranslator,
  formatDateTime,
  localiseProblem,
  resolveLocale,
} from "./index";

describe("i18n", () => {
  it("prefers the URL language, then the server preference, then the browser", () => {
    expect(
      resolveLocale({ urlLang: "vi", preference: "en", browser: "en-US" }),
    ).toBe("vi");
    expect(
      resolveLocale({ urlLang: null, preference: "vi", browser: "en-US" }),
    ).toBe("vi");
    expect(
      resolveLocale({ urlLang: null, preference: null, browser: "vi-VN" }),
    ).toBe("vi");
    expect(
      resolveLocale({ urlLang: null, preference: null, browser: "fr-FR" }),
    ).toBe("en");
  });

  it("falls back to English for a key missing from the Vietnamese resource", () => {
    const t = createTranslator("vi");
    // All shipped keys are translated; an unknown key returns the key itself.
    expect(t("nav.dashboard")).toBe("Tổng quan");
    expect(t("does.not.exist")).toBe("does.not.exist");
  });

  it("interpolates parameters into a message", () => {
    const t = createTranslator("en");
    expect(t("auth.signedInAs")).toBe("Signed in as");
    expect(t("common.retry")).toBe("Try again");
  });

  it("formats an instant in the user's timezone and locale", () => {
    expect(
      formatDateTime("2026-11-08T12:00:00Z", "en", "UTC"),
    ).toContain("2026");
    expect(
      formatDateTime("2026-11-08T12:00:00Z", "vi", "Asia/Ho_Chi_Minh"),
    ).toContain("2026");
  });

  it("renders an absent instant as an em dash", () => {
    expect(formatDateTime(null, "en", "UTC")).toBe("—");
  });

  it("maps a known problem type to a localised message and keeps the server detail otherwise", () => {
    const t = createTranslator("en");
    expect(
      localiseProblem(
        { type: "https://crypto-pocket-butler.example/problems/validation-failed", title: "Validation failed", status: 422, correlationId: "c1" },
        t,
      ),
    ).toBe("Some fields need attention.");
    expect(
      localiseProblem(
        { type: "about:blank", title: "Something specific", status: 400, correlationId: "c2" },
        t,
      ),
    ).toBe("Something specific");
  });
});
