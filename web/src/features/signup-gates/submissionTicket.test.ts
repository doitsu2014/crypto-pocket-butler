import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { discardTicket, peekTicket, resetSubmissionTicket, ticketKeyFor } from "./submissionTicket";
import type { GateFormValues } from "./gateStatus";

/**
 * The memory-only submission ticket (security-design §4; NFR11.1/NFR11.2).
 * The key must outlive component remounts, bind to exactly one payload and one
 * session, and never touch a storage API.
 */

const values: GateFormValues = {
  countryCode: "VN",
  terms: true,
  risk: true,
  age: true,
};

let minted = 0;

beforeEach(() => {
  resetSubmissionTicket();
  minted = 0;
  vi.spyOn(globalThis.crypto, "randomUUID").mockImplementation(
    () => `00000000-0000-4000-8000-${String(++minted).padStart(12, "0")}`,
  );
});

afterEach(() => {
  vi.restoreAllMocks();
  resetSubmissionTicket();
});

describe("submission ticket", () => {
  it("mints a crypto.randomUUID key stamped with the current session", () => {
    const key = ticketKeyFor("user-1", values);

    expect(globalThis.crypto.randomUUID).toHaveBeenCalledTimes(1);
    expect(key).toBe("00000000-0000-4000-8000-000000000001");
    expect(peekTicket("user-1")).toBe(key);
  });

  it("reuses the key for an identical payload", () => {
    const first = ticketKeyFor("user-1", values);
    const second = ticketKeyFor("user-1", { ...values });

    expect(second).toBe(first);
    expect(globalThis.crypto.randomUUID).toHaveBeenCalledTimes(1);
  });

  it("mints a new key when any field changes", () => {
    const first = ticketKeyFor("user-1", values);

    const country = ticketKeyFor("user-1", { ...values, countryCode: "SG" });
    const terms = ticketKeyFor("user-1", { ...values, countryCode: "SG", terms: false });
    const risk = ticketKeyFor("user-1", {
      ...values,
      countryCode: "SG",
      terms: false,
      risk: false,
    });
    const age = ticketKeyFor("user-1", {
      ...values,
      countryCode: "SG",
      terms: false,
      risk: false,
      age: false,
    });

    expect(new Set([first, country, terms, risk, age]).size).toBe(5);
    expect(globalThis.crypto.randomUUID).toHaveBeenCalledTimes(5);
  });

  it("discards the key on success so a later visit mints a fresh one", () => {
    const first = ticketKeyFor("user-1", values);
    discardTicket("user-1");

    expect(peekTicket("user-1")).toBeNull();
    const second = ticketKeyFor("user-1", values);
    expect(second).not.toBe(first);
  });

  it("never shows the previous key to a different session", () => {
    ticketKeyFor("user-1", values);

    expect(peekTicket("user-2")).toBeNull();
    const other = ticketKeyFor("user-2", values);

    expect(other).toBe("00000000-0000-4000-8000-000000000002");
    expect(peekTicket("user-1")).toBeNull();
    expect(peekTicket("user-2")).toBe(other);
  });

  it("writes nothing to localStorage or sessionStorage", () => {
    const local = vi.spyOn(globalThis.localStorage, "setItem");
    const session = vi.spyOn(globalThis.sessionStorage, "setItem");

    ticketKeyFor("user-1", values);
    discardTicket("user-1");

    expect(local).not.toHaveBeenCalled();
    expect(session).not.toHaveBeenCalled();
  });
});
