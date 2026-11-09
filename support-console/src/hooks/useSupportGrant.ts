import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { getSupportGrant } from "../api/client";
import type { SupportAccessGrant } from "../api/types";
import { useClearSupportCacheOnErrors } from "./useClearSupportCacheOnErrors";

/** Reads one grant by id. A missing id leaves the query idle; a 401/403 clears the cache. */
export function useSupportGrant(
  grantId: string,
  token: string | null,
): UseQueryResult<SupportAccessGrant, Error> {
  const query = useQuery<SupportAccessGrant, Error>({
    queryKey: ["support", "grant", grantId],
    queryFn: () => getSupportGrant(grantId, token),
    enabled: grantId.trim().length > 0,
  });

  useClearSupportCacheOnErrors(query.error);
  return query;
}
