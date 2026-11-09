import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ApiError, NetworkError } from "../api/client";
import { clearSupportCache, createSupportQueryClient } from "../api/queryClient";
import { createTestQueryClient, queryWrapper } from "../test/fixtures";
import { useClearSupportCacheOnErrors } from "./useClearSupportCacheOnErrors";

function forbidden() {
  return new ApiError({ type: "t", title: "Access ended", status: 403, correlationId: "c" });
}

function unauthorized() {
  return new ApiError({ type: "t", title: "Sign in again", status: 401, correlationId: "c" });
}

describe("useClearSupportCacheOnErrors", () => {
  it("clears the cache on a 403", () => {
    const client = createTestQueryClient();
    client.setQueryData(["support", "accounts"], { items: [] });

    const { rerender } = renderHook(({ error }) => useClearSupportCacheOnErrors(error), {
      wrapper: queryWrapper(client),
      initialProps: { error: undefined as unknown },
    });

    rerender({ error: forbidden() });

    expect(client.getQueryCache().getAll()).toHaveLength(0);
  });

  it("clears the cache on a 401", () => {
    const client = createTestQueryClient();
    client.setQueryData(["support", "metadata"], { id: "acct-1" });

    const { rerender } = renderHook(({ error }) => useClearSupportCacheOnErrors(error), {
      wrapper: queryWrapper(client),
      initialProps: { error: undefined as unknown },
    });

    rerender({ error: unauthorized() });

    expect(client.getQueryCache().getAll()).toHaveLength(0);
  });

  it("leaves the cache alone for a network error", () => {
    const client = createTestQueryClient();
    client.setQueryData(["support", "metadata"], { id: "acct-1" });

    const { rerender } = renderHook(({ error }) => useClearSupportCacheOnErrors(error), {
      wrapper: queryWrapper(client),
      initialProps: { error: undefined as unknown },
    });

    rerender({ error: new NetworkError("offline") });

    expect(client.getQueryData(["support", "metadata"])).toEqual({ id: "acct-1" });
  });

  it("clearSupportCache is the single clearing entry point", () => {
    const client = createSupportQueryClient();
    client.setQueryData(["support", "accounts"], { items: [] });
    clearSupportCache(client);
    expect(client.getQueryCache().getAll()).toHaveLength(0);
  });
});
