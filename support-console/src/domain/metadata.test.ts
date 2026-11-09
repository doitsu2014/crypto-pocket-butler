import { describe, expect, it } from "vitest";
import {
  ACCOUNT_METADATA_FIELDS,
  formatMetadataValue,
  pickAccountMetadata,
  pickAccountSearchResult,
  pickIncident,
  pickSupportGrant,
  pickTicketMetadata,
  TICKET_METADATA_FIELDS,
} from "./metadata";
import { activeGrant, activeIncident } from "../test/fixtures";

describe("account metadata minimisation", () => {
  it("keeps only the metadata fields the console may show", () => {
    const raw = {
      id: "acct-1",
      kind: "exchange",
      label: "Main Binance",
      venue: "binance",
      status: "active",
      lastSyncedAt: "2026-11-08T08:55:00.000Z",
      credentials: { apiKey: "should-not-appear" },
      holdings: [{ asset: "BTC", quantity: "1" }],
      valuation: "1000.00",
      portfolio: { id: "p-1" },
    };

    const metadata = pickAccountMetadata(raw);

    expect(metadata).toEqual({
      id: "acct-1",
      kind: "exchange",
      label: "Main Binance",
      venue: "binance",
      status: "active",
      lastSyncedAt: "2026-11-08T08:55:00.000Z",
    });
    expect(Object.keys(metadata).sort()).toEqual([...ACCOUNT_METADATA_FIELDS].sort());
    expect(JSON.stringify(metadata)).not.toMatch(/credential|holding|valuation|portfolio/i);
  });

  it("leaves an absent optional field undefined", () => {
    const metadata = pickAccountMetadata({ id: "acct-2", kind: "wallet", label: "Cold", status: "active" });
    expect(metadata.lastSyncedAt).toBeUndefined();
    expect(metadata.venue).toBeUndefined();
  });

  it("rejects a non-object payload instead of guessing", () => {
    expect(() => pickAccountMetadata(null)).toThrow(/metadata/i);
    expect(() => pickAccountMetadata("not-json")).toThrow(/metadata/i);
  });

  it("normalises a search response and strips every non-metadata field", () => {
    const result = pickAccountSearchResult({
      items: [
        {
          id: "acct-1",
          kind: "exchange",
          label: "Main Binance",
          status: "active",
          credentials: { apiKey: "secret" },
        },
      ],
      nextCursor: "cursor-2",
      totalCount: 42,
    });

    expect(result.nextCursor).toBe("cursor-2");
    expect(result.items).toHaveLength(1);
    expect(Object.keys(result.items[0]).sort()).toEqual(
      ["id", "kind", "label", "status"].sort(),
    );
  });

  it("rejects a search payload without an items array", () => {
    expect(() => pickAccountSearchResult({ accounts: [] })).toThrow(/metadata/i);
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
    expect(JSON.stringify(ticket)).not.toMatch(/private|conversation|messages|requesterLabel|Customer A/i);
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

  it("rejects a grant with an unknown status", () => {
    expect(() => pickSupportGrant({ ...activeGrant(), status: "escalated" })).toThrow(
      /grant status/i,
    );
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
