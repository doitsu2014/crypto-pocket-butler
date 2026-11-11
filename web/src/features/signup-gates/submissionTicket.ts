/**
 * The submission ticket: a memory-only idempotency key for G3
 * (security-design §4; NFR11.1/NFR11.2).
 *
 * The holder lives above the screen component so an in-app remount during a
 * pending submission reuses the same key. It is stamped with the session id it
 * was minted for and bound to one exact payload: an identical resubmission
 * reuses the key, any field edit mints a new one, and a different session never
 * sees it. Nothing is ever persisted (NFR10.1).
 *
 * Requirements: FR19.4, NFR11.1, NFR11.2, NFR10.1.
 */

import type { GateFormValues } from "./gateStatus";

interface Ticket {
  sessionUserId: string;
  signature: string;
  key: string;
}

let ticket: Ticket | null = null;

function randomKey(): string {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi && typeof cryptoApi.randomUUID === "function") {
    return cryptoApi.randomUUID();
  }
  return `gate-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function signatureOf(values: GateFormValues): string {
  return JSON.stringify([values.countryCode, values.terms, values.risk, values.age]);
}

/**
 * Returns the key for this exact payload and session, minting one on the first
 * intent or after any field edit.
 */
export function ticketKeyFor(sessionUserId: string, values: GateFormValues): string {
  const signature = signatureOf(values);
  if (ticket && ticket.sessionUserId === sessionUserId && ticket.signature === signature) {
    return ticket.key;
  }
  ticket = { sessionUserId, signature, key: randomKey() };
  return ticket.key;
}

/** The key currently bound to this session, if any (no minting). */
export function peekTicket(sessionUserId: string): string | null {
  return ticket?.sessionUserId === sessionUserId ? ticket.key : null;
}

/** Drops the key after a successful submit; the next intent mints a new one. */
export function discardTicket(sessionUserId: string): void {
  if (ticket?.sessionUserId === sessionUserId) {
    ticket = null;
  }
}

/** Test hook: forget the holder between cases. Not called by app code. */
export function resetSubmissionTicket(): void {
  ticket = null;
}
