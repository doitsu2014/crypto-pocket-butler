import {
  useMutation,
  useQuery,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import {
  changeUserRole,
  disableUser,
  listPendingGrants,
  lookupAdminUser,
  restoreUser,
} from "../api/client";
import type {
  AdminStatusResult,
  AdminUserRecord,
  ConsoleRole,
  RoleChangeBody,
  RoleChangeResult,
  SupportAccessGrant,
} from "../api/types";
import { useClearSupportCacheOnErrors } from "./useClearSupportCacheOnErrors";

/**
 * An administrator's lookup is an audited action, so it is one-shot: no
 * automatic retry, no background refresh, and nothing is cached once the
 * section is left (`["admin"]` is removed on unmount by `AdminUsersSection`).
 */
export function useAdminLookup(
  token: string | null,
  email: string,
  enabled: boolean,
): UseQueryResult<AdminUserRecord, Error> {
  const query = useQuery<AdminUserRecord, Error>({
    queryKey: ["admin", "user", email],
    queryFn: () => lookupAdminUser(token, email),
    enabled: enabled && email.trim().length > 0,
    staleTime: 0,
    retry: false,
    refetchOnWindowFocus: false,
    refetchInterval: false,
  });
  useClearSupportCacheOnErrors(query.error);
  return query;
}

/** The pending grants a fresh admin session can see, read from the route, not the shell. */
export function usePendingGrants(
  token: string | null,
): UseQueryResult<SupportAccessGrant[], Error> {
  const query = useQuery<SupportAccessGrant[], Error>({
    queryKey: ["admin", "pending-grants"],
    queryFn: () => listPendingGrants(token),
    staleTime: 0,
    retry: false,
    refetchOnWindowFocus: false,
    refetchInterval: false,
  });
  useClearSupportCacheOnErrors(query.error);
  return query;
}

export type ChangeRoleInput = { userId: string; body: RoleChangeBody };

export function useChangeRole(
  token: string | null,
): UseMutationResult<RoleChangeResult, Error, ChangeRoleInput> {
  return useMutation<RoleChangeResult, Error, ChangeRoleInput>({
    mutationFn: ({ userId, body }) => changeUserRole(token, userId, body),
  });
}

export type SetUserStatusInput = {
  userId: string;
  currentStatus: AdminUserRecord["status"];
  reason: string;
};

export function useSetUserStatus(
  token: string | null,
): UseMutationResult<AdminStatusResult, Error, SetUserStatusInput> {
  return useMutation<AdminStatusResult, Error, SetUserStatusInput>({
    mutationFn: ({ userId, currentStatus, reason }) =>
      currentStatus === "disabled"
        ? restoreUser(token, userId, { reason })
        : disableUser(token, userId, { reason }),
  });
}

export const CONSOLE_ROLES: ConsoleRole[] = ["owner", "admin", "support"];
