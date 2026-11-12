import { describe, expect, it } from "vitest";
import {
  formatMetadataValue,
  pickIncident,
  pickSupportGrant,
  pickSupportMetadata,
  pickTicketMetadata,
  SUPPORT_METADATA_FIELDS,
  TICKET_METADATA_FIELDS,
} from "./metadata";
import { activeGrant, activeIncident, supportMetadata } from "../test/fixtures";

describe("support metadata minimisation", () => {
  it("keeps only the whitelisted projection and drops every over-sent field", () => {
    const raw = {
      ...supportMetadata(),
      email: "customer@example.test",
      credentials: { apiKey: "never-render" },
      holdings: [{ asset: "BTC", quantity: "1" }],
      valuation: "1000.00",
      accounts: [
        {
          id: "acct-1",
          label: "Main Binance",
          sourceType: "exchange",
          syncStatus: "active",
          lastSyncedAt: "2026-11-08T08:55:00.000Z",
          balance: "1000.00",
          address: "0xdeadbeef",
        },
      ],
    };

    const metadata = pickSupportMetadata(raw);

    expect(Object.keys(metadata).sort()).toEqual([...SUPPORT_METADATA_FIELDS].sort());
    expect(metadata.accounts[0]).toEqual({
      id: "acct-1",
      label: "Main Binance",
      sourceType: "exchange",
      syncStatus: "active",
      lastSyncedAt: "2026-11-08T08:55:00.000Z",
    });
    expect(JSON.stringify(metadata)).not.toMatch(
      /credential|holding|valuation|balance|address|never-render|customer@example/i,
    );
  });

  it("maps a missing display name to null rather than dropping the key", () => {
    const withoutName = supportMetadata();
    delete (withoutName as Partial<typeof withoutName>).displayName;

    expect(pickSupportMetadata(withoutName).displayName).toBeNull();
    expect(pickSupportMetadata({ ...supportMetadata(), displayName: null }).displayName).toBeNull();
  });

  it("keeps an empty projection as a legitimate answer", () => {
    const metadata = pickSupportMetadata(
      supportMetadata({ accounts: [], portfolios: [], displayName: null }),
    );
    expect(metadata.accounts).toEqual([]);
    expect(metadata.portfolios).toEqual([]);
    expect(metadata.displayName).toBeNull();
  });

  it("throws on a malformed projection instead of rendering a partial one", () => {
    expect(() => pickSupportMetadata(null)).toThrow(/metadata/i);
    expect(() => pickSupportMetadata({ ...supportMetadata(), accounts: "nope" })).toThrow(
      /metadata/i,
    );
    expect(() => pickSupportMetadata({ ...supportMetadata(), emailVerified: "yes" })).toThrow(
      /metadata/i,
    );
    expect(() => pickSupportMetadata({ ...supportMetadata(), portfolios: [{}] })).toThrow(
      /metadata/i,
    );
  });

  it("renders a missing value as an em dash, never as zero", () => {
    expect(formatMetadataValue(null)).toBe("—");
    expect(formatMetadataValue(undefined)).toBe("—");
    expect(formatMetadataValue("   ")).toBe("—");
    expect(formatMetadataValue("active")).toBe("active");
  });
});

describe("ticket metadata minimisation", () => {
  it("keeps ticket metadata only and never conversation or requester labels", () => {
    const raw = {
      id: "ticket-1",
      subject: "Sync status question",
      status: "open",
      updatedAt: "2026-11-08T08:00:00.000Z",
      requesterRef: "userref-1",
      requesterLabel: "Customer A",
      conversation: [{ body: "the customer's private message" }],
      messages: ["private"],
    };

    const ticket = pickTicketMetadata(raw);

    expect(ticket).toEqual({
      id: "ticket-1",
      subject: "Sync status question",
      status: "open",
      updatedAt: "2026-11-08T08:00:00.000Z",
      requesterRef: "userref-1",
    });
    expect(Object.keys(ticket).sort()).toEqual([...TICKET_METADATA_FIELDS].sort());
    expect(JSON.stringify(ticket)).not.toMatch(
      /private|conversation|messages|requesterLabel|Customer A/i,
    );
  });

  it("rejects a non-object ticket payload", () => {
    expect(() => pickTicketMetadata(undefined)).toThrow(/metadata/i);
  });
});

describe("grant and incident pickers", () => {
  it("keeps only the grant fields and strips any over-sent data", () => {
    const source = activeGrant();
    const raw = {
      ...source,
      credentials: { apiKey: "never-render" },
      portfolio: { id: "p-1" },
    };

    const grant = pickSupportGrant(raw);

    expect(grant).toEqual(source);
    expect(JSON.stringify(grant)).not.toMatch(/credential|portfolio|never-render/i);
  });

  it("accepts a grant answer keyed grantId as well as id", () => {
    const { id, ...withoutId } = activeGrant({ id: "grant-1" });
    const grant = pickSupportGrant({ ...withoutId, grantId: id });

    expect(grant.id).toBe("grant-1");
  });

  it("rejects a grant with an unknown status or no identifier", () => {
    expect(() => pickSupportGrant({ ...activeGrant(), status: "escalated" })).toThrow(
      /grant status/i,
    );
    const { id: droppedId, ...withoutId } = activeGrant();
    expect(droppedId).toBe("grant-1");
    expect(() => pickSupportGrant(withoutId)).toThrow(/missing id/i);
  });

  it("keeps only the incident fields, including the C9 outcome", () => {
    const source = activeIncident();
    const raw = {
      ...source,
      credentials: { apiKey: "never-render" },
      internalNotes: "not for the console",
    };

    const incident = pickIncident(raw);

    expect(incident).toEqual(source);
    expect(incident.notification).toEqual({ state: "sent" });
    expect(JSON.stringify(incident)).not.toMatch(/credential|internalNotes|never-render/i);
  });

  it("rejects an incident with an unknown severity or status", () => {
    expect(() => pickIncident({ ...activeIncident(), severity: "sev9" })).toThrow(
      /incident severity/i,
    );
    expect(() => pickIncident({ ...activeIncident(), status: "closed" })).toThrow(
      /incident status/i,
    );
  });
});
