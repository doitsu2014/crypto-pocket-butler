import { useMutation, useQuery } from "@tanstack/react-query";
import { gatewayClient } from "./client";
import type {
  Account,
  Brief,
  ExportJob,
  Incident,
  NotificationItem,
  NotificationPreferences,
  Pnl,
  Portfolio,
  PortfolioValuation,
  Profile,
  Snapshot,
  SupportArticle,
  Transaction,
} from "./types";

/**
 * Server-state hooks. Business math never happens here; figures are rendered
 * exactly as the gateway supplied them (BR1.1).
 */

const MINUTE = 60_000;

export const queryKeys = {
  profile: ["profile"] as const,
  portfolios: ["portfolios"] as const,
  portfolio: (id: string) => ["portfolio", id] as const,
  accounts: (userId: string) => ["accounts", userId] as const,
  transactions: (userId: string, cursor?: string) =>
    ["transactions", userId, cursor ?? null] as const,
  pnl: (userId: string) => ["pnl", userId] as const,
  briefs: (userId: string, period?: string, cursor?: string) =>
    ["briefs", userId, period ?? null, cursor ?? null] as const,
  brief: (userId: string, briefId: string) => ["brief", userId, briefId] as const,
  notifications: (userId: string, cursor?: string) =>
    ["notifications", userId, cursor ?? null] as const,
  notificationPreferences: (userId: string) => ["notificationPreferences", userId] as const,
  snapshots: (portfolioId: string) => ["snapshots", portfolioId] as const,
  export: (exportId: string) => ["export", exportId] as const,
  articles: ["articles"] as const,
  incidents: ["incidents"] as const,
};

/**
 * The signed-in user's identity as the shell and other journeys read it (the
 * account id behind their lists). The settings Profile panel reads P1 through
 * `settingsQueries.useProfile` instead; this hook is not its source.
 */
export function useProfile(enabled = true) {
  return useQuery<Profile>({
    queryKey: queryKeys.profile,
    queryFn: () => gatewayClient.getProfile(),
    staleTime: MINUTE,
    enabled,
  });
}

export function usePortfolios() {
  return useQuery<Portfolio[]>({
    queryKey: queryKeys.portfolios,
    queryFn: () => gatewayClient.listPortfolios(),
    staleTime: MINUTE,
  });
}

export function usePortfolio(portfolioId: string | undefined) {
  return useQuery<PortfolioValuation>({
    queryKey: queryKeys.portfolio(portfolioId ?? "none"),
    queryFn: () => gatewayClient.getPortfolio(portfolioId as string),
    staleTime: 30_000,
    enabled: Boolean(portfolioId),
  });
}

export function useAccounts(userId: string | undefined) {
  return useQuery<Account[]>({
    queryKey: queryKeys.accounts(userId ?? "none"),
    queryFn: () => gatewayClient.listAccounts(userId as string),
    staleTime: 30_000,
    enabled: Boolean(userId),
  });
}

export function useTransactions(userId: string | undefined, cursor?: string) {
  return useQuery<Transaction[]>({
    queryKey: queryKeys.transactions(userId ?? "none", cursor),
    queryFn: () => gatewayClient.listTransactions(userId as string, cursor),
    staleTime: 30_000,
    enabled: Boolean(userId),
    placeholderData: (previous) => previous,
  });
}

export function usePnl(userId: string | undefined) {
  return useQuery<Pnl>({
    queryKey: queryKeys.pnl(userId ?? "none"),
    queryFn: () => gatewayClient.getPnl(userId as string),
    staleTime: 30_000,
    enabled: Boolean(userId),
  });
}

export function useBriefs(
  userId: string | undefined,
  period?: "daily" | "weekly",
  cursor?: string,
) {
  return useQuery<Brief[]>({
    queryKey: queryKeys.briefs(userId ?? "none", period, cursor),
    queryFn: () => gatewayClient.listBriefs(userId as string, period, cursor),
    staleTime: 5 * MINUTE,
    enabled: Boolean(userId),
    placeholderData: (previous) => previous,
  });
}

export function useBrief(userId: string | undefined, briefId: string | undefined) {
  return useQuery<Brief>({
    queryKey: queryKeys.brief(userId ?? "none", briefId ?? "none"),
    queryFn: () => gatewayClient.getBrief(userId as string, briefId as string),
    staleTime: 5 * MINUTE,
    enabled: Boolean(userId && briefId),
  });
}

export function useNotifications(userId: string | undefined, cursor?: string, enabled = true) {
  return useQuery<NotificationItem[]>({
    queryKey: queryKeys.notifications(userId ?? "none", cursor),
    queryFn: () => gatewayClient.listNotifications(userId as string, cursor),
    staleTime: 30_000,
    enabled: enabled && Boolean(userId),
    placeholderData: (previous) => previous,
  });
}

export function useNotificationPreferences(userId: string | undefined, enabled = true) {
  return useQuery<NotificationPreferences>({
    queryKey: queryKeys.notificationPreferences(userId ?? "none"),
    queryFn: () => gatewayClient.getNotificationPreferences(userId as string),
    staleTime: 5 * MINUTE,
    enabled: enabled && Boolean(userId),
  });
}

export function useSnapshots(portfolioId: string | undefined) {
  return useQuery<Snapshot[]>({
    queryKey: queryKeys.snapshots(portfolioId ?? "none"),
    queryFn: () => gatewayClient.listSnapshots(portfolioId as string),
    staleTime: MINUTE,
    enabled: Boolean(portfolioId),
  });
}

export function useExport(exportId: string | undefined) {
  return useQuery<ExportJob>({
    queryKey: queryKeys.export(exportId ?? "none"),
    queryFn: () => gatewayClient.getExport(exportId as string),
    staleTime: 5_000,
    enabled: Boolean(exportId),
  });
}

export function useArticles(enabled = true) {
  return useQuery<SupportArticle[]>({
    queryKey: queryKeys.articles,
    queryFn: () => gatewayClient.listArticles(),
    staleTime: 5 * MINUTE,
    enabled,
  });
}

export function useIncidents(enabled = true) {
  return useQuery<Incident[]>({
    queryKey: queryKeys.incidents,
    queryFn: () => gatewayClient.listIncidents(),
    staleTime: 30_000,
    enabled,
  });
}

export function useStartExport() {
  return useMutation<ExportJob, Error, void>({
    mutationFn: () => gatewayClient.startExport(),
  });
}
