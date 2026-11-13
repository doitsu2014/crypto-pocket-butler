import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { listMyGrants } from "../api/client";
import type { SupportAccessGrant } from "../api/types";
import type { SupportRole } from "../auth/session";
import { useClearSupportQueriesOnRouteErrors } from "./useClearSupportCacheOnErrors";

/**
 * S2: the signed-in agent's own grants (the gateway scopes the list to the
 * caller). It always refetches on mount, never on focus or a timer, and never
 * retries; a 403 drops the support entries, a 401 drops everything.
 */
export function useMyGrants(
  token: string | null,
  role: SupportRole | null,
): UseQueryResult<SupportAccessGrant[], Error> {
  const query = useQuery<SupportAccessGrant[], Error>({
    queryKey: ["support", "grants", "mine"],
    queryFn: () => listMyGrants(token),
    enabled: token !== null && role === "support",
    staleTime: 0,
    refetchOnWindowFocus: false,
    refetchInterval: false,
    retry: false,
  });

  useClearSupportQueriesOnRouteErrors(query.error);
  return query;
}
