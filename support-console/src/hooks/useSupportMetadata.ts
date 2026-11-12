import { useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { fetchSupportMetadata } from "../api/client";
import { isForbiddenError, isUnauthorizedError } from "../api/queryClient";
import type { GrantStatus, SupportAccessGrant, SupportMetadata } from "../api/types";
import { useClearSupportQueriesOnRouteErrors } from "./useClearSupportCacheOnErrors";

/**
 * S3: one read of the closed metadata projection for the live grant. It is
 * enabled only while the clock-derived status is `active`; a 403 ends the
 * session's access (`onAccessEnded`) and removes the cached projection.
 */
export function useSupportMetadata(
  grant: SupportAccessGrant,
  token: string | null,
  status: GrantStatus | null,
  onAccessEnded?: () => void,
): UseQueryResult<SupportMetadata, Error> {
  const queryClient = useQueryClient();
  const grantId = grant.id;
  const [ended, setEnded] = useState(false);

  const onAccessEndedRef = useRef(onAccessEnded);
  useEffect(() => {
    onAccessEndedRef.current = onAccessEnded;
  }, [onAccessEnded]);

  const query = useQuery<SupportMetadata, Error>({
    queryKey: ["support", "metadata", grantId],
    queryFn: () => fetchSupportMetadata(grantId, token),
    enabled: status === "active" && token !== null && !ended,
    staleTime: 10_000,
    retry: false,
  });

  useClearSupportQueriesOnRouteErrors(query.error);

  useEffect(() => {
    if (isUnauthorizedError(query.error) || isForbiddenError(query.error)) {
      setEnded(true);
    }
    if (isForbiddenError(query.error)) {
      queryClient.removeQueries({ queryKey: ["support", "metadata", grantId] });
      onAccessEndedRef.current?.();
    }
  }, [query.error, queryClient, grantId]);

  return query;
}
