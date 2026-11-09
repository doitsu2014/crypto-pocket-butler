import type { IncidentSeverity, IncidentStatus, NotificationState } from "../api/types";
import type { StatusPresentation } from "./presentation";

export const INCIDENT_SEVERITIES = ["sev1", "sev2", "sev3"] as const;
export const INCIDENT_STATUSES = ["declared", "investigating", "resolved"] as const;
export const MAX_SUMMARY_LENGTH = 200;
export const SEV1_NOTICE_WINDOW_MINUTES = 15;

export function isIncidentSeverity(value: string): value is IncidentSeverity {
  return (INCIDENT_SEVERITIES as readonly string[]).includes(value);
}

export function isIncidentStatus(value: string): value is IncidentStatus {
  return (INCIDENT_STATUSES as readonly string[]).includes(value);
}

export interface IncidentValues {
  severity: string;
  summary: string;
  components: string[];
}

export interface IncidentErrors {
  severity?: string;
  summary?: string;
  components?: string;
}

export function validateIncident(values: IncidentValues): IncidentErrors {
  const errors: IncidentErrors = {};

  if (!isIncidentSeverity(values.severity)) {
    errors.severity = "Choose a severity of sev1, sev2, or sev3.";
  }

  const summary = values.summary?.trim() ?? "";
  if (!summary) {
    errors.summary = "Enter an incident summary.";
  } else if (summary.length > MAX_SUMMARY_LENGTH) {
    errors.summary = `The summary must be ${MAX_SUMMARY_LENGTH} characters or fewer.`;
  }

  const components = values.components.map((component) => component.trim()).filter(Boolean);
  if (components.length === 0) {
    errors.components = "List at least one affected component.";
  }

  return errors;
}

export function hasIncidentErrors(errors: IncidentErrors): boolean {
  return Object.values(errors).some((message) => message !== undefined);
}

/** Splits the comma/newline separated component field, trimming and deduplicating. */
export function parseComponents(text: string): string[] {
  const seen = new Set<string>();
  const components: string[] = [];
  for (const part of text.split(/[\n,]/)) {
    const component = part.trim();
    const key = component.toLowerCase();
    if (component && !seen.has(key)) {
      seen.add(key);
      components.push(component);
    }
  }
  return components;
}

export function incidentSeverityPresentation(severity: IncidentSeverity): StatusPresentation {
  switch (severity) {
    case "sev1":
      return { label: "SEV-1", tone: "danger", shape: "square" };
    case "sev2":
      return { label: "SEV-2", tone: "warn", shape: "diamond" };
    case "sev3":
      return { label: "SEV-3", tone: "neutral", shape: "dot" };
  }
}

export function incidentStatusPresentation(status: IncidentStatus): StatusPresentation {
  switch (status) {
    case "declared":
      return { label: "Declared", tone: "warn", shape: "diamond" };
    case "investigating":
      return { label: "Investigating", tone: "warn", shape: "ring" };
    case "resolved":
      return { label: "Resolved", tone: "success", shape: "dot" };
  }
}

export function notificationPresentation(state: NotificationState): StatusPresentation {
  switch (state) {
    case "sent":
      return { label: "Notification sent", tone: "success", shape: "dot" };
    case "pending":
      return { label: "Notification pending", tone: "warn", shape: "diamond" };
    case "failed":
      return { label: "Notification failed", tone: "danger", shape: "square" };
  }
}

/** BR3.2: a SEV-1 declaration or escalation needs a C9 notice within 15 minutes. */
export function requiresImmediateNotice(severity: string): boolean {
  return severity === "sev1";
}
