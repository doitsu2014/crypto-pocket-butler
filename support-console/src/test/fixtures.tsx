import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, type RenderResult } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";
import type {
  AccountMetadata,
  Incident,
  SupportAccessGrant,
  TicketMetadata,
} from "../api/types";
import type { SupportSession } from "../auth/session";

export function createTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: Infinity },
      mutations: { retry: false },
    },
  });
}

export interface RenderWithQueryResult extends RenderResult {
  client: QueryClient;
}

export function renderWithQuery(
  ui: ReactElement,
  client: QueryClient = createTestQueryClient(),
): RenderWithQueryResult {
  const result = render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
  return { ...result, client };
}

export function queryWrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  };
}

export const supportSession: SupportSession = {
  accessToken: "support-token",
  user: {
    id: "agent-1",
    email: "agent@example.test",
    appMetadata: { role: "support" },
  },
};

export const adminSession: SupportSession = {
  accessToken: "admin-token",
  user: {
    id: "admin-1",
    email: "admin@example.test",
    appMetadata: { role: "admin" },
  },
};

export const customerSession: SupportSession = {
  accessToken: "customer-token",
  user: {
    id: "customer-1",
    email: "customer@example.test",
    appMetadata: { role: "customer" },
  },
};

export function activeGrant(overrides: Partial<SupportAccessGrant> = {}): SupportAccessGrant {
  const now = Date.now();
  return {
    id: "grant-1",
    userId: "user-1",
    requestedBy: "agent-1",
    reason: "Investigating a reported sync failure",
    requestedAt: new Date(now - 600_000).toISOString(),
    expiresAt: new Date(now + 3_600_000).toISOString(),
    status: "active",
    approvedBy: "agent-2",
    approvedAt: new Date(now - 300_000).toISOString(),
    ...overrides,
  };
}

export function pendingGrant(overrides: Partial<SupportAccessGrant> = {}): SupportAccessGrant {
  const now = Date.now();
  return {
    id: "grant-pending",
    userId: "user-2",
    requestedBy: "agent-1",
    reason: "Customer cannot see a synced account balance",
    requestedAt: new Date(now - 120_000).toISOString(),
    expiresAt: null,
    status: "pending_approval",
    approvedBy: null,
    approvedAt: null,
    ...overrides,
  };
}

export function expiredGrant(overrides: Partial<SupportAccessGrant> = {}): SupportAccessGrant {
  const now = Date.now();
  return {
    id: "grant-expired",
    userId: "user-1",
    requestedBy: "agent-1",
    reason: "Investigating a reported sync failure",
    requestedAt: new Date(now - 7_200_000).toISOString(),
    expiresAt: new Date(now - 60_000).toISOString(),
    status: "active",
    approvedBy: "agent-2",
    approvedAt: new Date(now - 7_000_000).toISOString(),
    ...overrides,
  };
}

export const accountMetadata: AccountMetadata = {
  id: "acct-1",
  kind: "exchange",
  label: "Main Binance",
  venue: "binance",
  status: "active",
  lastSyncedAt: "2026-11-08T08:55:00.000Z",
};

export const ticketMetadata: TicketMetadata = {
  id: "ticket-1",
  subject: "Sync status question",
  status: "open",
  updatedAt: "2026-11-08T08:00:00.000Z",
  requesterRef: "userref-1",
};

export function activeIncident(overrides: Partial<Incident> = {}): Incident {
  return {
    id: "incident-1",
    severity: "sev2",
    status: "investigating",
    summary: "Account sync delays for one venue",
    components: ["account-sync", "market-data"],
    declaredAt: "2026-11-08T08:00:00.000Z",
    updatedAt: "2026-11-08T08:30:00.000Z",
    notification: { state: "sent" },
    ...overrides,
  };
}

export const problemDetails = (status: number, title: string, correlationId = "corr-1") => ({
  type: `https://crypto-pocket-butler.example/problems/${status}`,
  title,
  status,
  detail: title,
  correlationId,
});
