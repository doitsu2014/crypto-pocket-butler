/**
 * C2 DTO shapes. These are deliberately narrow: account metadata and ticket
 * metadata only. There is no credential, holding, valuation, portfolio, or
 * ticket-conversation field anywhere in this module (BR1.4).
 */

export type GrantStatus = "pending_approval" | "active" | "expired" | "revoked";

export interface SupportAccessGrant {
  id: string;
  userId: string;
  requestedBy?: string | null;
  reason: string;
  requestedAt: string;
  expiresAt: string | null;
  status: GrantStatus;
  approvedBy: string | null;
  approvedAt: string | null;
}

export interface AccountMetadata {
  id: string;
  kind: string;
  label: string;
  venue?: string | null;
  status: string;
  lastSyncedAt?: string | null;
}

export interface AccountSearchResult {
  items: AccountMetadata[];
  nextCursor: string | null;
}

export interface TicketMetadata {
  id: string;
  subject: string;
  status: string;
  updatedAt: string;
  /** A pseudonymous reference; the ticket system's requester label never leaves it. */
  requesterRef?: string | null;
}

export interface TicketList {
  items: TicketMetadata[];
}

export type IncidentSeverity = "sev1" | "sev2" | "sev3";

export type IncidentStatus = "declared" | "investigating" | "resolved";

export type NotificationState = "sent" | "pending" | "failed";

export interface IncidentNotification {
  state: NotificationState;
  detail?: string | null;
}

export interface Incident {
  id: string;
  severity: IncidentSeverity;
  status: IncidentStatus;
  summary: string;
  components: string[];
  declaredAt: string;
  updatedAt: string;
  notification?: IncidentNotification | null;
}

export interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail?: string;
  correlationId: string;
}

export interface GrantRequestInput {
  userId: string;
  reason: string;
  durationMinutes: number;
  approverId: string;
}

export interface GrantRequestValues {
  requesterId: string;
  targetUserId: string;
  reason: string;
  durationMinutes: number;
  approverId: string;
}

export interface MetadataReadInput {
  grantId: string;
  reason: string;
}

export interface AccountSearchInput {
  grantId: string;
  query: string;
}

export interface IncidentInput {
  severity: IncidentSeverity;
  summary: string;
  components: string[];
}

export interface IncidentPatch {
  severity?: IncidentSeverity;
  status?: IncidentStatus;
  summary?: string;
  components?: string[];
}
