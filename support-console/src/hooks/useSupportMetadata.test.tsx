import { renderHook, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { describe, expect, it, vi } from "vitest";
import {
  activeGrant,
  createTestQueryClient,
  pendingGrant,
  problemDetails,
  queryWrapper,
  supportMetadata,
} from "../test/fixtures";
import { server } from "../test/server";
import { useSupportMetadata } from "./useSupportMetadata";

const METADATA = "*/v1/support-grants/*/metadata";

describe("useSupportMetadata", () => {
  it("stays idle while the clock-derived status is not active", () => {
    const fetchSpy = vi.fn();
    server.use(
      http.get(METADATA, () => {
        fetchSpy();
        return HttpResponse.json(supportMetadata());
      }),
    );

    const client = createTestQueryClient();
    const { result } = renderHook(
      () => useSupportMetadata(pendingGrant(), "token-1", "pending_approval", vi.fn()),
      { wrapper: queryWrapper(client) },
    );

    expect(result.current.fetchStatus).toBe("idle");
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("reads one grant's metadata once and maps it through the picker", async () => {
    let requests = 0;
    let seenPath: string | null = null;
    server.use(
      http.get(METADATA, ({ request }) => {
        requests += 1;
        seenPath = new URL(request.url).pathname;
        return HttpResponse.json({
          ...supportMetadata(),
          credentials: { apiKey: "never-render" },
        });
      }),
    );

    const client = createTestQueryClient();
    const { result } = renderHook(
      () => useSupportMetadata(activeGrant(), "token-1", "active", vi.fn()),
      { wrapper: queryWrapper(client) },
    );

    await waitFor(() => expect(result.current.data).toEqual(supportMetadata()));
    expect(seenPath).toBe("/v1/support-grants/grant-1/metadata");
    expect(requests).toBe(1);
    expect(result.current.isStale).toBe(false);
  });

  it("keeps the read cached for ten seconds and refetches only on Refresh", async () => {
    let requests = 0;
    server.use(
      http.get(METADATA, () => {
        requests += 1;
        return HttpResponse.json(supportMetadata());
      }),
    );

    const client = createTestQueryClient();
    const { result } = renderHook(
      () => useSupportMetadata(activeGrant(), "token-1", "active", vi.fn()),
      { wrapper: queryWrapper(client) },
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.isStale).toBe(false);

    window.dispatchEvent(new Event("focus"));
    expect(requests).toBe(1);

    await result.current.refetch();
    expect(requests).toBe(2);
  });

  it("calls onAccessEnded and drops the key on a 403", async () => {
    let requests = 0;
    server.use(
      http.get(METADATA, () => {
        requests += 1;
        return HttpResponse.json(problemDetails(403, "Access ended"), {
          status: 403,
          headers: { "Content-Type": "application/problem+json" },
        });
      }),
    );

    const onAccessEnded = vi.fn();
    const client = createTestQueryClient();
    const { result } = renderHook(
      () => useSupportMetadata(activeGrant(), "token-1", "active", onAccessEnded),
      { wrapper: queryWrapper(client) },
    );

    await waitFor(() => expect(onAccessEnded).toHaveBeenCalledTimes(1));
    expect(requests).toBe(1);
    expect(result.current.data).toBeUndefined();
    expect(client.getQueryData(["support", "metadata", "grant-1"])).toBeUndefined();
  });

  it("clears every cached entry on a 401", async () => {
    let requests = 0;
    server.use(
      http.get(METADATA, () => {
        requests += 1;
        return HttpResponse.json(problemDetails(401, "Sign in again"), {
          status: 401,
          headers: { "Content-Type": "application/problem+json" },
        });
      }),
    );

    const client = createTestQueryClient();
    client.setQueryData(["admin", "users", "lookup"], { id: "user-1" });

    renderHook(() => useSupportMetadata(activeGrant(), "token-1", "active", vi.fn()), {
      wrapper: queryWrapper(client),
    });

    await waitFor(() => expect(requests).toBe(1));
    await waitFor(() => expect(client.getQueryData(["admin", "users", "lookup"])).toBeUndefined());
    expect(client.getQueryData(["support", "metadata", "grant-1"])).toBeUndefined();
  });
});
