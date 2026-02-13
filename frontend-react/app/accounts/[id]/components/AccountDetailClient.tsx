"use client";

import { useParams, useRouter } from "next/navigation";
import { useAccount, useSyncAccount } from "@/hooks";
import { LoadingSkeleton, LoadingButton } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import ErrorAlert from "@/components/ErrorAlert";
import { useToast } from "@/contexts/ToastContext";
import { ApiError } from "@/lib/api-client";

// Helper function to get user-friendly chain name
function getChainDisplayName(chainId: string): string {
  const chainMap: Record<string, string> = {
    ethereum: 'Ethereum',
    arbitrum: 'Arbitrum',
    optimism: 'Optimism',
    base: 'Base',
    bsc: 'BNB Chain',
  };
  return chainMap[chainId] || chainId;
}

function formatDate(dateString: string | undefined): string {
  if (!dateString) return 'Never';
  try {
    const date = new Date(dateString);
    if (isNaN(date.getTime())) {
      return 'Never';
    }
    return date.toLocaleString();
  } catch {
    return 'Never';
  }
}

export default function AccountDetailClient() {
  const params = useParams();
  const router = useRouter();
  const accountId = params.id as string;
  const toast = useToast();

  const { data: account, isLoading, error: queryError, refetch } = useAccount(accountId);
  const syncAccount = useSyncAccount();

  // Convert query error to string for display
  const error = queryError instanceof ApiError ? queryError.message : 
                queryError ? "Failed to load account" : null;

  async function handleSyncAccount() {
    try {
      const result = await syncAccount.mutateAsync(accountId);
      if (result.success) {
        toast.success(`Account synced successfully! ${result.holdings_count} holdings updated.`);
        refetch();
      } else {
        toast.error(result.error || "Sync failed");
      }
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to sync account";
      toast.error(message);
    }
  }

  if (isLoading) {
    return <LoadingSkeleton />;
  }

  if (error) {
    return (
      <ErrorAlert 
        message={error} 
        onRetry={() => refetch()}
        type="banner"
      />
    );
  }

  if (!account) {
    return (
      <EmptyState
        icon={
          <svg className="w-16 h-16 text-fuchsia-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z" />
          </svg>
        }
        title="Account not found"
        description="The account you're looking for doesn't exist or you don't have access to it."
      />
    );
  }

  return (
    <>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-4">
          <button
            onClick={() => router.push("/accounts")}
            className="inline-flex items-center px-3 py-2 border-2 border-slate-600 text-sm font-medium rounded-lg text-slate-300 bg-slate-900/50 hover:bg-slate-800/70 hover:border-slate-500 transition-all duration-300"
          >
            <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
            </svg>
            Back
          </button>
          <div className="flex items-center gap-3">
            <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-fuchsia-500 to-violet-600 flex items-center justify-center shadow-[0_0_20px_rgba(217,70,239,0.5)]">
              <svg className="w-7 h-7 text-white drop-shadow-[0_0_8px_rgba(255,255,255,0.6)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                {account.account_type === "wallet" ? (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
                ) : (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                )}
              </svg>
            </div>
            <div>
              <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent drop-shadow-[0_0_12px_rgba(232,121,249,0.4)]">
                {account.name}
              </h2>
              <div className="flex items-center gap-2 mt-1">
                <span className={`inline-flex items-center gap-1 text-xs px-2 py-1 rounded-full ${
                  account.account_type === "wallet" 
                    ? "bg-violet-900/50 text-violet-300 border border-violet-500/50" 
                    : "bg-cyan-900/50 text-cyan-300 border border-cyan-500/50"
                }`}>
                  {account.account_type === "wallet" ? "Wallet" : "Exchange"}
                </span>
                {!account.is_active && (
                  <span className="inline-flex items-center gap-1 text-xs px-2 py-1 rounded-full bg-slate-800/50 text-slate-400 border border-slate-600/50">
                    Inactive
                  </span>
                )}
              </div>
            </div>
          </div>
        </div>
        <LoadingButton
          onClick={handleSyncAccount}
          loading={syncAccount.isPending}
          disabled={syncAccount.isPending}
          className="inline-flex items-center px-6 py-2 border-2 border-cyan-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-cyan-600 to-blue-600 hover:from-cyan-500 hover:to-blue-500 disabled:opacity-50 disabled:cursor-not-allowed shadow-[0_0_20px_rgba(6,182,212,0.4)] hover:shadow-[0_0_30px_rgba(6,182,212,0.6)] transition-all duration-300 transform hover:scale-105"
        >
          <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          {syncAccount.isPending ? "Syncing..." : "Sync"}
        </LoadingButton>
      </div>

      {/* Account Details */}
      <div className="bg-slate-900/60 backdrop-blur-sm rounded-2xl border-2 border-fuchsia-500/50 p-6 mb-6 shadow-[0_0_25px_rgba(217,70,239,0.3)]">
        <h3 className="text-xl font-bold text-fuchsia-300 mb-4 drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
          Account Information
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {account.account_type === "wallet" && account.wallet_address && (
            <div>
              <label className="block text-sm font-medium text-slate-400 mb-1">Wallet Address</label>
              <p className="text-slate-200 font-mono break-all">{account.wallet_address}</p>
            </div>
          )}
          {account.account_type === "exchange" && account.exchange_name && (
            <div>
              <label className="block text-sm font-medium text-slate-400 mb-1">Exchange</label>
              <p className="text-slate-200 uppercase">{account.exchange_name}</p>
            </div>
          )}
          {account.enabled_chains && account.enabled_chains.length > 0 && (
            <div>
              <label className="block text-sm font-medium text-slate-400 mb-1">Enabled Chains</label>
              <div className="flex flex-wrap gap-2 mt-2">
                {account.enabled_chains.map((chain) => (
                  <span 
                    key={chain}
                    className="inline-flex items-center text-xs px-3 py-1 rounded-full bg-violet-900/40 text-violet-300 border border-violet-500/40"
                  >
                    {getChainDisplayName(chain)}
                  </span>
                ))}
              </div>
            </div>
          )}
          <div>
            <label className="block text-sm font-medium text-slate-400 mb-1">Last Synced</label>
            <p className={`text-slate-200 ${account.last_synced_at ? "text-cyan-400" : ""}`}>
              {formatDate(account.last_synced_at)}
            </p>
          </div>
          <div>
            <label className="block text-sm font-medium text-slate-400 mb-1">Created</label>
            <p className="text-slate-200">{formatDate(account.created_at)}</p>
          </div>
        </div>
      </div>

      {/* Holdings */}
      <div className="bg-slate-900/60 backdrop-blur-sm rounded-2xl border-2 border-fuchsia-500/50 p-6 shadow-[0_0_25px_rgba(217,70,239,0.3)]">
        <h3 className="text-xl font-bold text-fuchsia-300 mb-4 drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
          Holdings
        </h3>
        {!account.holdings || account.holdings.length === 0 ? (
          <EmptyState
            icon={
              <svg className="w-12 h-12 text-violet-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
              </svg>
            }
            title="No holdings"
            description="This account has no holdings. Sync the account to fetch the latest holdings data."
          />
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b-2 border-violet-500/30">
                  <th className="text-left py-3 px-4 text-sm font-bold text-violet-300">Asset</th>
                  <th className="text-right py-3 px-4 text-sm font-bold text-violet-300">Quantity</th>
                </tr>
              </thead>
              <tbody>
                {account.holdings.map((holding, index) => (
                  <tr 
                    key={index}
                    className="border-b border-slate-700/50 hover:bg-slate-800/30 transition-colors"
                  >
                    <td className="py-3 px-4 text-slate-200 font-medium">{holding.asset}</td>
                    <td className="py-3 px-4 text-slate-200 text-right font-mono">{holding.quantity}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </>
  );
}
