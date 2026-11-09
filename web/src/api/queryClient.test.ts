import { describe, expect, it, vi } from "vitest";
import { ApiError, NetworkError } from "./client";
import { createQueryClient, shouldRetry } from "./queryClient";

function problem(status: number): ApiError {
  return new ApiError({
    type: "about:blank",
    title: `status ${status}`,
    status,
    correlationId: `c-${status}`,
  });
}

describe("query client policy", () => {
  it("retries a 5xx exactly once", () => {
    expect(shouldRetry(0, problem(500))).toBe(true);
    expect(shouldRetry(1, problem(500))).toBe(false);
  });

  it("does not retry a 4xx or a network error", () => {
    expect(shouldRetry(0, problem(422))).toBe(false);
    expect(shouldRetry(0, problem(403))).toBe(false);
    expect(shouldRetry(0, new NetworkError("offline"))).toBe(false);
  });

  it("retries one transient server failure before surfacing it", async () => {
    const queryFn = vi
      .fn()
      .mockRejectedValueOnce(problem(500))
      .mockResolvedValueOnce("ok");
    const client = createQueryClient();

    await expect(
      client.fetchQuery({ queryKey: ["retry"], queryFn }),
    ).resolves.toBe("ok");
    expect(queryFn).toHaveBeenCalledTimes(2);
  });

  it("signs out on a 401 from a query", async () => {
    const onUnauthorized = vi.fn();
    const client = createQueryClient({ onUnauthorized });
    const queryFn = vi.fn().mockRejectedValue(problem(401));

    await expect(
      client.fetchQuery({ queryKey: ["unauthorized"], queryFn }),
    ).rejects.toMatchObject({ problem: { status: 401 } });
    expect(queryFn).toHaveBeenCalledTimes(1);
    expect(onUnauthorized).toHaveBeenCalledOnce();
  });

  it("signs out on a 401 from a mutation", async () => {
    const onUnauthorized = vi.fn();
    const client = createQueryClient({ onUnauthorized });

    await expect(
      client
        .getMutationCache()
        .build(client, { mutationFn: () => Promise.reject(problem(401)) })
        .execute(undefined),
    ).rejects.toMatchObject({ problem: { status: 401 } });
    expect(onUnauthorized).toHaveBeenCalledOnce();
  });
});
