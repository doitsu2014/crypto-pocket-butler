import { describe, expect, it } from "vitest";
import { ApiError, NetworkError } from "./client";
import {
  clearSupportCache,
  createSupportQueryClient,
  isForbiddenError,
  isUnauthorizedError,
} from "./queryClient";

describe("support query client", () => {
  it("disables retries for reads and writes", () => {
    const client = createSupportQueryClient();
    expect(client.getDefaultOptions().queries?.retry).toBe(false);
    expect(client.getDefaultOptions().mutations?.retry).toBe(false);
  });

  it("clears every cached query and mutation", () => {
    const client = createSupportQueryClient();
    client.setQueryData(["support", "accounts"], { items: [] });
    client.setQueryData(["support", "tickets"], { items: [] });

    clearSupportCache(client);

    expect(client.getQueryData(["support", "accounts"])).toBeUndefined();
    expect(client.getQueryData(["support", "tickets"])).toBeUndefined();
    expect(client.getQueryCache().getAll()).toHaveLength(0);
  });

  it("recognises a 403 and a 401 problem", () => {
    expect(isForbiddenError(new ApiError({ type: "t", title: "No", status: 403, correlationId: "c" }))).toBe(
      true,
    );
    expect(isUnauthorizedError(new ApiError({ type: "t", title: "No", status: 401, correlationId: "c" }))).toBe(
      true,
    );
    expect(isForbiddenError(new ApiError({ type: "t", title: "No", status: 500, correlationId: "c" }))).toBe(
      false,
    );
    expect(isForbiddenError(new NetworkError("offline"))).toBe(false);
    expect(isUnauthorizedError(undefined)).toBe(false);
  });
});
