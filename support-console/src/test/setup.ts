import "@testing-library/jest-dom/vitest";
import { transferableAbortController } from "node:util";
import { cleanup } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, vi } from "vitest";
import { clearSupportSession } from "../auth/session";
import { server } from "./server";

/*
 * Node's fetch rejects a signal created by jsdom's AbortController
 * ("RequestInit: Expected signal to be an AbortSignal"). The client gives every
 * U10 call a deadline through an AbortController, so the test environment maps
 * the global to a controller whose signal Node accepts.
 */
class TestAbortController {
  readonly #inner = transferableAbortController();

  get signal(): AbortSignal {
    return this.#inner.signal as unknown as AbortSignal;
  }

  abort(reason?: unknown): void {
    this.#inner.abort(reason);
  }
}

globalThis.AbortController = TestAbortController as unknown as typeof globalThis.AbortController;

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));

afterEach(() => {
  server.resetHandlers();
  cleanup();
  clearSupportSession();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  window.localStorage.clear();
  window.sessionStorage.clear();
});

afterAll(() => server.close());
