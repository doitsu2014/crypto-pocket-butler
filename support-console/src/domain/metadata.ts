import type {
  AccountMetadata,
  AccountSearchResult,
  GrantStatus,
  Incident,
  IncidentNotification,
  IncidentSeverity,
  IncidentStatus,
  NotificationState,
  SupportAccessGrant,
  TicketMetadata,
} from "../api/types";

/**
 * BR1.4: the console whitelists the fields it shows. Even if the support API
 * over-sends, a credential, holding, valuation, or portfolio field can never
 * reach the component tree.
 */
export const ACCOUNT_METADATA_FIELDS = [
  "id",
  "kind",
  "label",
  "venue",
  "status",
  "lastSyncedAt",
] as const;

export const TICKET_METADATA_FIELDS = [
  "id",
  "subject",
  "status",
  "updatedAt",
  "requesterRef",
] as const;

function asRecord(raw: unknown): Record<string, unknown> {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new Error("The support API returned an invalid metadata payload.");
  }
  return raw as Record<string, unknown>;
}

function readString(record: Record<string, unknown>, key: string): string | undefined {
  const value = record[key];
  return typeof value === "string" ? value : undefined;
}

function readNullableString(
  record: Record<string, unknown>,
  key: string,
): string | null | undefined {
  const value = record[key];
  if (value === null) {
    return null;
  }
  return typeof value === "string" ? value : undefined;
}

function requireOneOf(
  record: Record<string, unknown>,
  key: string,
  allowed: readonly string[],
  label: string,
): string {
  const value = requireString(record, key);
  if (!allowed.includes(value)) {
    throw new Error(`The support API returned an invalid ${label}.`);
  }
  return value;
}

function requireString(record: Record<string, unknown>, key: string): string {
  const value = readString(record, key);
  if (!value) {
    throw new Error(`The support API returned incomplete metadata (missing ${key}).`);
  }
  return value;
}

export function pickAccountMetadata(raw: unknown): AccountMetadata {
  const record = asRecord(raw);
  const metadata: AccountMetadata = {
    id: requireString(record, "id"),
    kind: requireString(record, "kind"),
    label: requireString(record, "label"),
    status: requireString(record, "status"),
  };

  const venue = readString(record, "venue");
  if (venue !== undefined) {
    metadata.venue = venue;
  }
  const lastSyncedAt = readString(record, "lastSyncedAt");
  if (lastSyncedAt !== undefined) {
    metadata.lastSyncedAt = lastSyncedAt;
  }
  return metadata;
}

export function pickTicketMetadata(raw: unknown): TicketMetadata {
  const record = asRecord(raw);
  const ticket: TicketMetadata = {
    id: requireString(record, "id"),
    subject: requireString(record, "subject"),
    status: requireString(record, "status"),
    updatedAt: requireString(record, "updatedAt"),
  };

  const requesterRef = readString(record, "requesterRef");
  if (requesterRef !== undefined) {
    ticket.requesterRef = requesterRef;
  }
  return ticket;
}

export function pickAccountSearchResult(raw: unknown): AccountSearchResult {
  const record = asRecord(raw);
  if (!Array.isArray(record.items)) {
    throw new Error("The support API returned an invalid metadata search payload.");
  }

  const nextCursor = readString(record, "nextCursor");
  return {
    items: record.items.map(pickAccountMetadata),
    nextCursor: nextCursor ?? null,
  };
}

export function pickTicketList(raw: unknown): { items: TicketMetadata[] } {
  const record = asRecord(raw);
  if (!Array.isArray(record.items)) {
    throw new Error("The support API returned an invalid ticket metadata payload.");
  }
  return { items: record.items.map(pickTicketMetadata) };
}

const GRANT_STATUSES = ["pending_approval", "active", "expired", "revoked"] as const;
const INCIDENT_SEVERITIES = ["sev1", "sev2", "sev3"] as const;
const INCIDENT_STATUSES = ["declared", "investigating", "resolved"] as const;
const NOTIFICATION_STATES = ["sent", "pending", "failed"] as const;

/**
 * Narrows a grant response to the C2 fields. An over-sent credential or
 * portfolio field cannot become a component prop.
 */
export function pickSupportGrant(raw: unknown): SupportAccessGrant {
  const record = asRecord(raw);
  const grant: SupportAccessGrant = {
    id: requireString(record, "id"),
    userId: requireString(record, "userId"),
    reason: requireString(record, "reason"),
    requestedAt: requireString(record, "requestedAt"),
    expiresAt: readNullableString(record, "expiresAt") ?? null,
    status: requireOneOf(record, "status", GRANT_STATUSES, "grant status") as GrantStatus,
    approvedBy: readNullableString(record, "approvedBy") ?? null,
    approvedAt: readNullableString(record, "approvedAt") ?? null,
  };

  const requestedBy = readNullableString(record, "requestedBy");
  if (requestedBy !== undefined) {
    grant.requestedBy = requestedBy;
  }
  return grant;
}

function pickNotification(raw: unknown): IncidentNotification | null | undefined {
  if (raw === undefined) {
    return undefined;
  }
  if (raw === null) {
    return null;
  }
  const record = asRecord(raw);
  const notification: IncidentNotification = {
    state: requireOneOf(
      record,
      "state",
      NOTIFICATION_STATES,
      "notification state",
    ) as NotificationState,
  };
  const detail = readNullableString(record, "detail");
  if (detail !== undefined) {
    notification.detail = detail;
  }
  return notification;
}

/** Narrows an incident response to the C2 fields, including its C9 outcome. */
export function pickIncident(raw: unknown): Incident {
  const record = asRecord(raw);
  const components = record.components;
  if (!Array.isArray(components) || components.some((item) => typeof item !== "string")) {
    throw new Error("The support API returned an invalid incident component list.");
  }

  const incident: Incident = {
    id: requireString(record, "id"),
    severity: requireOneOf(
      record,
      "severity",
      INCIDENT_SEVERITIES,
      "incident severity",
    ) as IncidentSeverity,
    status: requireOneOf(record, "status", INCIDENT_STATUSES, "incident status") as IncidentStatus,
    summary: requireString(record, "summary"),
    components: components as string[],
    declaredAt: requireString(record, "declaredAt"),
    updatedAt: requireString(record, "updatedAt"),
  };

  const notification = pickNotification(record.notification);
  if (notification !== undefined) {
    incident.notification = notification;
  }
  return incident;
}

/** A missing value is an em dash, never 0 or an empty cell. */
export function formatMetadataValue(value: string | null | undefined): string {
  const trimmed = value?.trim();
  return trimmed ? trimmed : "—";
}
