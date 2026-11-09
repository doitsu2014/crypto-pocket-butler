import { describe, expect, it } from "vitest";
import {
  INCIDENT_SEVERITIES,
  INCIDENT_STATUSES,
  incidentSeverityPresentation,
  incidentStatusPresentation,
  MAX_SUMMARY_LENGTH,
  notificationPresentation,
  parseComponents,
  requiresImmediateNotice,
  SEV1_NOTICE_WINDOW_MINUTES,
  validateIncident,
} from "./incidents";

describe("incident validation", () => {
  const valid = {
    severity: "sev2" as const,
    summary: "Account sync delays for one venue",
    components: ["account-sync"],
  };

  it("accepts a complete incident", () => {
    expect(validateIncident(valid)).toEqual({});
  });

  it("rejects an unknown severity", () => {
    const errors = validateIncident({ ...valid, severity: "sev9" });
    expect(errors.severity).toMatch(/sev1/i);
    expect(INCIDENT_SEVERITIES).toEqual(["sev1", "sev2", "sev3"]);
  });

  it("bounds the summary to 200 characters", () => {
    expect(validateIncident({ ...valid, summary: "   " }).summary).toMatch(/summary/i);
    expect(
      validateIncident({ ...valid, summary: "a".repeat(MAX_SUMMARY_LENGTH) }).summary,
    ).toBeUndefined();
    expect(
      validateIncident({ ...valid, summary: "a".repeat(MAX_SUMMARY_LENGTH + 1) }).summary,
    ).toMatch(/200/);
  });

  it("requires at least one affected component", () => {
    expect(validateIncident({ ...valid, components: [] }).components).toMatch(/component/i);
    expect(validateIncident({ ...valid, components: ["  "] }).components).toMatch(/component/i);
    expect(validateIncident({ ...valid, components: [" ok "] }).components).toBeUndefined();
  });
});

describe("component parsing", () => {
  it("splits commas and newlines, trims, and drops empties", () => {
    expect(parseComponents(" account-sync, market-data\n , portfolio ")).toEqual([
      "account-sync",
      "market-data",
      "portfolio",
    ]);
  });

  it("deduplicates components case-insensitively", () => {
    expect(parseComponents("Account-Sync, account-sync, API")).toEqual(["Account-Sync", "API"]);
  });

  it("returns an empty list for blank text", () => {
    expect(parseComponents("   ")).toEqual([]);
  });
});

describe("incident presentation", () => {
  it("labels all severities with a shape and tone", () => {
    expect(incidentSeverityPresentation("sev1").label).toBe("SEV-1");
    expect(incidentSeverityPresentation("sev1").tone).toBe("danger");
    expect(incidentSeverityPresentation("sev1").shape).toBe("square");
    expect(incidentSeverityPresentation("sev2").label).toBe("SEV-2");
    expect(incidentSeverityPresentation("sev3").label).toBe("SEV-3");
  });

  it("labels all statuses with a shape and tone", () => {
    expect(INCIDENT_STATUSES).toEqual(["declared", "investigating", "resolved"]);
    expect(incidentStatusPresentation("declared").label).toBe("Declared");
    expect(incidentStatusPresentation("investigating").label).toBe("Investigating");
    expect(incidentStatusPresentation("resolved").label).toBe("Resolved");
    for (const status of INCIDENT_STATUSES) {
      expect(["dot", "diamond", "square", "ring"]).toContain(
        incidentStatusPresentation(status).shape,
      );
    }
  });

  it("presents the C9 notification outcome with a label and shape", () => {
    expect(notificationPresentation("sent").label).toBe("Notification sent");
    expect(notificationPresentation("pending").label).toBe("Notification pending");
    expect(notificationPresentation("failed").label).toBe("Notification failed");
    expect(notificationPresentation("failed").shape).toBe("square");
  });

  it("requires an immediate notice only for SEV-1", () => {
    expect(SEV1_NOTICE_WINDOW_MINUTES).toBe(15);
    expect(requiresImmediateNotice("sev1")).toBe(true);
    expect(requiresImmediateNotice("sev2")).toBe(false);
    expect(requiresImmediateNotice("sev3")).toBe(false);
  });
});
