import { renderHook, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import { createTestQueryClient, activeGrant, problemDetails, queryWrapper } from "../test/fixtures";
import { server } from "../test/server";
import { useSupportGrant } from "./useSupportGrant";

describe("useSupportGrant", () => {
  it("reads one grant with the bearer token", async () => {
    const grant = activeGrant();
    let authorization: string | null = null;
    server.use(
      http.get("*/v1/support-grants/*", ({ request }) => {
        authorization = request.headers.get("Authorization");
        return HttpResponse.json(grant);
      }),
    );

    const client = createTestQueryClient();
    const { result } = renderHook(() => useSupportGrant("grant-1", "token-1"), {
      wrapper: queryWrapper(client),
    });

    await waitFor(() => expect(result.current.data).toEqual(grant));
    expect(authorization).toBe("Bearer token-1");
  });

  it("clears the cache when the read is forbidden", async () => {
    server.use(
      http.get("*/v1/support-grants/*", () =>
        HttpResponse.json(problemDetails(403, "Access ended"), {
          status: 403,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );

    const client = createTestQueryClient();
    client.setQueryData(["support", "accounts"], { items: [] });

    const { result } = renderHook(() => useSupportGrant("grant-1", "token-1"), {
      wrapper: queryWrapper(client),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(client.getQueryData(["support", "accounts"])).toBeUndefined();
  });

  it("does not fetch without a grant id", () => {
    const client = createTestQueryClient();
    const { result } = renderHook(() => useSupportGrant("", "token-1"), {
      wrapper: queryWrapper(client),
    });

    expect(result.current.fetchStatus).toBe("idle");
    expect(result.current.data).toBeUndefined();
  });
});
