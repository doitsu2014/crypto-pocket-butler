import { renderHook, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { describe, expect, it, vi } from "vitest";
import { activeGrant, createTestQueryClient, problemDetails, queryWrapper } from "../test/fixtures";
import { server } from "../test/server";
import { useMyGrants } from "./useMyGrants";

const MINE = "*/v1/support-grants/mine";

describe("useMyGrants", () => {
  it("stays idle without a token", () => {
    const fetchSpy = vi.fn();
    server.use(
      http.get(MINE, () => {
        fetchSpy();
        return HttpResponse.json([]);
      }),
    );

    const client = createTestQueryClient();
    const { result } = renderHook(() => useMyGrants(null, "support"), {
      wrapper: queryWrapper(client),
    });

    expect(result.current.fetchStatus).toBe("idle");
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("stays idle for a non-support role", () => {
    const fetchSpy = vi.fn();
    server.use(
      http.get(MINE, () => {
        fetchSpy();
        return HttpResponse.json([]);
      }),
    );

    const client = createTestQueryClient();
    const { result } = renderHook(() => useMyGrants("token-1", "admin"), {
      wrapper: queryWrapper(client),
    });

    expect(result.current.fetchStatus).toBe("idle");
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("reads GET /support-grants/mine with the bearer token under the mine key", async () => {
    const grant = activeGrant();
    let authorization: string | null = null;
    server.use(
      http.get(MINE, ({ request }) => {
        authorization = request.headers.get("Authorization");
        return HttpResponse.json([grant]);
      }),
    );

    const client = createTestQueryClient();
    const { result } = renderHook(() => useMyGrants("token-1", "support"), {
      wrapper: queryWrapper(client),
    });

    await waitFor(() => expect(result.current.data).toEqual([grant]));
    expect(authorization).toBe("Bearer token-1");
    expect(client.getQueryData(["support", "grants", "mine"])).toEqual([grant]);
  });

  it("stays stale immediately and issues no extra request on focus or timers", async () => {
    let requests = 0;
    server.use(
      http.get(MINE, () => {
        requests += 1;
        return HttpResponse.json([activeGrant()]);
      }),
    );

    const client = createTestQueryClient();
    const { result } = renderHook(() => useMyGrants("token-1", "support"), {
      wrapper: queryWrapper(client),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.isStale).toBe(true);

    window.dispatchEvent(new Event("focus"));
    vi.useFakeTimers();
    await vi.advanceTimersByTimeAsync(60_000);
    vi.useRealTimers();

    expect(requests).toBe(1);
  });

  it("does not retry a 403 and clears only the support entries", async () => {
    let requests = 0;
    server.use(
      http.get(MINE, () => {
        requests += 1;
        return HttpResponse.json(problemDetails(403, "Access ended"), {
          status: 403,
          headers: { "Content-Type": "application/problem+json" },
        });
      }),
    );

    const client = createTestQueryClient();
    client.setQueryData(["support", "metadata", "grant-1"], { userId: "user-1" });
    client.setQueryData(["admin", "users", "lookup"], { id: "user-1" });

    const { result } = renderHook(() => useMyGrants("token-1", "support"), {
      wrapper: queryWrapper(client),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(requests).toBe(1);
    expect(client.getQueryData(["support", "metadata", "grant-1"])).toBeUndefined();
    expect(client.getQueryData(["admin", "users", "lookup"])).toEqual({ id: "user-1" });
  });
});
