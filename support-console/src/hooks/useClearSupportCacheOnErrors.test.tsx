import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ApiError, NetworkError } from "../api/client";
import { clearSupportCache, createSupportQueryClient } from "../api/queryClient";
import { createTestQueryClient, queryWrapper } from "../test/fixtures";
import {
  useClearSupportCacheOnErrors,
  useClearSupportQueriesOnRouteErrors,
} from "./useClearSupportCacheOnErrors";

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

  it("still clears everything, including admin data, on both 401 and 403", () => {
    for (const error of [forbidden(), unauthorized()]) {
      const client = createTestQueryClient();
      client.setQueryData(["support", "grants", "mine"], []);
      client.setQueryData(["admin", "users", "lookup"], { id: "user-1" });

      const { rerender } = renderHook(({ next }) => useClearSupportCacheOnErrors(next), {
        wrapper: queryWrapper(client),
        initialProps: { next: undefined as unknown },
      });

      rerender({ next: error });

      expect(client.getQueryCache().getAll()).toHaveLength(0);
    }
  });

  it("clearSupportCache is the single clearing entry point", () => {
    const client = createSupportQueryClient();
    client.setQueryData(["support", "accounts"], { items: [] });
    clearSupportCache(client);
    expect(client.getQueryCache().getAll()).toHaveLength(0);
  });
});

describe("useClearSupportQueriesOnRouteErrors", () => {
  it("clears every query on a 401", () => {
    const client = createTestQueryClient();
    client.setQueryData(["support", "grants", "mine"], []);
    client.setQueryData(["admin", "users", "lookup"], { id: "user-1" });

    const { rerender } = renderHook(({ next }) => useClearSupportQueriesOnRouteErrors(next), {
      wrapper: queryWrapper(client),
      initialProps: { next: undefined as unknown },
    });

    rerender({ next: unauthorized() });

    expect(client.getQueryCache().getAll()).toHaveLength(0);
  });

  it("clears only the support prefix on a 403 so admin data survives", () => {
    const client = createTestQueryClient();
    client.setQueryData(["support", "grants", "mine"], []);
    client.setQueryData(["support", "metadata", "grant-1"], { userId: "user-1" });
    client.setQueryData(["admin", "users", "lookup"], { id: "user-1" });

    const { rerender } = renderHook(({ next }) => useClearSupportQueriesOnRouteErrors(next), {
      wrapper: queryWrapper(client),
      initialProps: { next: undefined as unknown },
    });

    rerender({ next: forbidden() });

    expect(client.getQueryData(["support", "grants", "mine"])).toBeUndefined();
    expect(client.getQueryData(["support", "metadata", "grant-1"])).toBeUndefined();
    expect(client.getQueryData(["admin", "users", "lookup"])).toEqual({ id: "user-1" });
  });

  it("leaves the cache alone for a network error", () => {
    const client = createTestQueryClient();
    client.setQueryData(["support", "grants", "mine"], []);

    const { rerender } = renderHook(({ next }) => useClearSupportQueriesOnRouteErrors(next), {
      wrapper: queryWrapper(client),
      initialProps: { next: undefined as unknown },
    });

    rerender({ next: new NetworkError("offline") });

    expect(client.getQueryData(["support", "grants", "mine"])).toEqual([]);
  });
});
