import { describe, expect, it, vi } from "vitest";
import {
  isolateSupportFromServiceWorkers,
  scopeCoversSupport,
  type ServiceWorkerContainerLike,
  type ServiceWorkerRegistrationLike,
} from "./isolate";

function registration(scope: string): ServiceWorkerRegistrationLike {
  return { scope, unregister: vi.fn().mockResolvedValue(true) };
}

function container(registrations: ServiceWorkerRegistrationLike[]): ServiceWorkerContainerLike {
  return { getRegistrations: vi.fn().mockResolvedValue(registrations) };
}

describe("scopeCoversSupport", () => {
  it("treats a root or /support scope as covering the console", () => {
    expect(scopeCoversSupport("/")).toBe(true);
    expect(scopeCoversSupport("https://example.test/")).toBe(true);
    expect(scopeCoversSupport("/support")).toBe(true);
    expect(scopeCoversSupport("/support/")).toBe(true);
    expect(scopeCoversSupport("https://example.test/support/")).toBe(true);
  });

  it("does not treat a sibling or narrower scope as covering /support", () => {
    expect(scopeCoversSupport("/other/")).toBe(false);
    expect(scopeCoversSupport("/app/")).toBe(false);
    expect(scopeCoversSupport("/support/embedded/")).toBe(false);
  });
});

describe("isolateSupportFromServiceWorkers", () => {
  it("unregisters a root-scoped customer worker that would control /support and /v1", async () => {
    const root = registration("https://example.test/");
    const support = registration("/support/");
    const other = registration("/other/");

    const count = await isolateSupportFromServiceWorkers(container([root, support, other]));

    expect(count).toBe(2);
    expect(root.unregister).toHaveBeenCalledTimes(1);
    expect(support.unregister).toHaveBeenCalledTimes(1);
    expect(other.unregister).not.toHaveBeenCalled();
  });

  it("does nothing when the browser has no Service Worker API", async () => {
    await expect(isolateSupportFromServiceWorkers(null)).resolves.toBe(0);
    await expect(isolateSupportFromServiceWorkers(undefined)).resolves.toBe(0);
  });

  it("surfaces a registration read failure instead of swallowing it", async () => {
    const failing: ServiceWorkerContainerLike = {
      getRegistrations: vi.fn().mockRejectedValue(new Error("worker API unavailable")),
    };

    await expect(isolateSupportFromServiceWorkers(failing)).rejects.toThrow(
      /worker api unavailable/i,
    );
  });
});
