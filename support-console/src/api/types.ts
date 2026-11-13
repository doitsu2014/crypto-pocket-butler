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

/**
 * The closed support metadata projection (S3). Exactly these five fields:
 * anything else the API sends is dropped by the picker, never rendered.
 */
export interface SupportMetadataAccount {
  id: string;
  label: string;
  sourceType: string;
  syncStatus: string;
  lastSyncedAt: string | null;
}

export interface SupportMetadataPortfolio {
  id: string;
  label: string;
}

export interface SupportMetadata {
  userId: string;
  displayName: string | null;
  emailVerified: boolean;
  accounts: SupportMetadataAccount[];
  portfolios: SupportMetadataPortfolio[];
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

export interface ProblemFieldError {
  field: string;
  reason: string;
}

export interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail?: string;
  correlationId: string;
  errors?: ProblemFieldError[];
}

/** `POST /v1/support-grants` body (S1). */
export interface GrantRequestInput {
  userId: string;
  reason: string;
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

/** The role an administrator can assign (U10). Distinct from `SupportRole`. */
export type ConsoleRole = "owner" | "admin" | "support";

/** The five user statuses the admin answer reports (U10). */
export type AdminUserStatus = "pending" | "active" | "disabled" | "pending_deletion" | "deleted";

/** The admin lookup answer: the record plus the provider's current role. */
export interface AdminUserRecord {
  id: string;
  displayName?: string;
  email?: string;
  emailVerified: boolean;
  status: AdminUserStatus;
  role: ConsoleRole;
  createdAt: string;
  updatedAt: string;
}

/** `PUT /admin/users/{userId}/role` body. */
export interface RoleChangeBody {
  newRole: ConsoleRole;
  reason: string;
}

/** The role change answer. */
export interface RoleChangeResult {
  userId: string;
  previousRole: ConsoleRole;
  newRole: ConsoleRole;
}

/** `POST /admin/users/{userId}/disable` and `/restore` body. */
export interface AdminStatusBody {
  reason: string;
}

/** The disable or restore answer. */
export interface AdminStatusResult {
  userId: string;
  status: AdminUserStatus;
  transitionId: string;
  occurredAt: string;
}
