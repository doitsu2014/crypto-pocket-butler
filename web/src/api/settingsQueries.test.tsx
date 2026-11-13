import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";
import { stubGateway } from "../test/gateway";
import { useAccountStatus } from "./settingsQueries";

/**
 * D0 is the routing read: NFR2.9 requires every authenticated entry to refetch
 * it, overriding the app-wide 30 s stale default.
 */

function wrapperFor(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

describe("useAccountStatus", () => {
  it("refetches D0 on every entry despite the app-wide 30 s default", async () => {
    let reads = 0;
    stubGateway({
      "GET /v1/me/account-status": () => {
        reads += 1;
        return { body: { status: "active" } };
      },
    });
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false, staleTime: 30_000 } },
    });

    const first = renderHook(() => useAccountStatus(), { wrapper: wrapperFor(queryClient) });
    await waitFor(() => expect(first.result.current.isSuccess).toBe(true));
    first.unmount();

    const second = renderHook(() => useAccountStatus(), { wrapper: wrapperFor(queryClient) });
    await waitFor(() => expect(second.result.current.isSuccess).toBe(true));

    await waitFor(() => expect(reads).toBe(2));
  });
});