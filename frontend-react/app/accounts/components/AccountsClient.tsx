"use client";

import { useState } from "react";
import { ApiError } from "@/lib/api-client";
import { useToast } from "@/contexts/ToastContext";
import { LoadingSkeleton, LoadingButton } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import ErrorAlert from "@/components/ErrorAlert";
import { useAccounts, useCreateAccount, useSyncAccount, useDeleteAccount } from "@/hooks";
import type { CreateAccountInput } from "@/types/api";

type AccountFormType = "wallet" | "exchange" | null;

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

export default function AccountsClient() {
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [formType, setFormType] = useState<AccountFormType>(null);
  const [deletingAccount, setDeletingAccount] = useState<string | null>(null);
  const [walletFormData, setWalletFormData] = useState({
    name: "",
    wallet_address: "",
  });
  const [exchangeFormData, setExchangeFormData] = useState({
    name: "",
    exchange_name: "okx",
    api_key: "",
    api_secret: "",
    passphrase: "",
  });
  const toast = useToast();

  // Use TanStack Query hooks
  const { data: accounts = [], isLoading: loading, error: queryError, refetch } = useAccounts();
  const createAccount = useCreateAccount();
  const syncAccount = useSyncAccount();
  const deleteAccount = useDeleteAccount();

  // Convert query error to string for display
  const error = queryError instanceof ApiError ? queryError.message : 
                queryError ? "Failed to load accounts" : null;

  async function handleCreateWallet(e: React.FormEvent) {
    e.preventDefault();
    if (!walletFormData.name.trim() || !walletFormData.wallet_address.trim()) {
      toast.error("Name and wallet address are required");
      return;
    }

    try {
      await createAccount.mutateAsync({
        name: walletFormData.name.trim(),
        account_type: "wallet",
        wallet_address: walletFormData.wallet_address.trim(),
      });
      
      toast.success("Wallet created successfully!");
      
      setWalletFormData({ name: "", wallet_address: "" });
      setShowCreateForm(false);
      setFormType(null);
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to create wallet";
      toast.error(message);
    }
  }

  async function handleCreateExchange(e: React.FormEvent) {
    e.preventDefault();
    if (!exchangeFormData.name.trim() || !exchangeFormData.api_key.trim() || !exchangeFormData.api_secret.trim()) {
      toast.error("Name, API key, and API secret are required");
      return;
    }

    try {
      await createAccount.mutateAsync({
        name: exchangeFormData.name.trim(),
        account_type: "exchange",
        exchange_name: exchangeFormData.exchange_name,
        api_key: exchangeFormData.api_key.trim(),
        api_secret: exchangeFormData.api_secret.trim(),
        passphrase: exchangeFormData.passphrase.trim() || undefined,
      } as CreateAccountInput);
      
      toast.success("Exchange account created successfully!");
      
      setExchangeFormData({ name: "", exchange_name: "okx", api_key: "", api_secret: "", passphrase: "" });
      setShowCreateForm(false);
      setFormType(null);
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to create exchange account";
      toast.error(message);
    }
  }

  async function handleSyncAccount(accountId: string) {
    try {
      const result = await syncAccount.mutateAsync(accountId);
      if (result.success) {
        toast.success(`Account synced successfully! ${result.holdings_count} holdings updated.`);
      } else {
        toast.error(result.error || "Sync failed");
      }
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to sync account";
      toast.error(message);
    }
  }

  async function handleSyncAll() {
    // Sync all accounts sequentially
    try {
      for (const account of accounts) {
        await handleSyncAccount(account.id);
      }
      toast.success("All accounts synced successfully!");
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to sync accounts";
      toast.error(message);
    }
  }

  async function handleDeleteAccount(accountId: string) {
    try {
      await deleteAccount.mutateAsync(accountId);
      toast.success("Account deleted successfully!");
      setDeletingAccount(null);
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to delete account";
      toast.error(message);
      setDeletingAccount(null);
    }
  }

  function confirmDelete(accountId: string) {
    setDeletingAccount(accountId);
  }

  function cancelDelete() {
    setDeletingAccount(null);
  }

  function openCreateForm(type: AccountFormType) {
    setFormType(type);
    setShowCreateForm(true);
  }

  function closeForm() {
    setShowCreateForm(false);
    setFormType(null);
  }

  return (
    <>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-fuchsia-500 to-violet-600 flex items-center justify-center shadow-[0_0_20px_rgba(217,70,239,0.5)]">
            <svg className="w-7 h-7 text-white drop-shadow-[0_0_8px_rgba(255,255,255,0.6)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
          </div>
          <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent drop-shadow-[0_0_12px_rgba(232,121,249,0.4)]">
            Accounts
          </h2>
        </div>
        <div className="flex gap-3">
          <LoadingButton
            onClick={handleSyncAll}
            loading={syncAccount.isPending}
            disabled={syncAccount.isPending || accounts.length === 0}
            className="inline-flex items-center px-4 py-2 border-2 border-cyan-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-cyan-600 to-blue-600 hover:from-cyan-500 hover:to-blue-500 disabled:opacity-50 disabled:cursor-not-allowed shadow-[0_0_20px_rgba(6,182,212,0.4)] hover:shadow-[0_0_30px_rgba(6,182,212,0.6)] transition-all duration-300 transform hover:scale-105"
          >
            <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            {syncAccount.isPending ? "Syncing..." : "Sync All"}
          </LoadingButton>
          <button
            onClick={() => openCreateForm(null)}
            className="inline-flex items-center px-6 py-2 border-2 border-fuchsia-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 shadow-[0_0_20px_rgba(217,70,239,0.4)] hover:shadow-[0_0_30px_rgba(217,70,239,0.6)] transition-all duration-300 transform hover:scale-105"
          >
            <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
            New Account
          </button>
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className="mb-6">
          <ErrorAlert 
            message={error} 
            onRetry={() => refetch()}
            type="banner"
          />
        </div>
      )}

      {/* Account Type Selection or Create Form */}
      {showCreateForm && (
        <div className="mb-6 bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40 shadow-[0_0_25px_rgba(217,70,239,0.3)] rounded-2xl p-6">
          {formType === null ? (
            <>
              <h3 className="text-xl font-bold text-fuchsia-300 mb-4 drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
                Select Account Type
              </h3>
              <div className="grid grid-cols-2 gap-4">
                <button
                  onClick={() => setFormType("wallet")}
                  className="p-6 bg-slate-900/50 border-2 border-violet-500/50 rounded-xl hover:border-fuchsia-500 hover:bg-slate-800/70 transition-all group"
                >
                  <svg className="w-12 h-12 mx-auto mb-3 text-violet-400 group-hover:text-fuchsia-400 transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
                  </svg>
                  <h4 className="text-lg font-bold text-fuchsia-300 mb-1">EVM Wallet</h4>
                  <p className="text-sm text-slate-400">Ethereum, BSC, Arbitrum, etc.</p>
                </button>
                <button
                  onClick={() => setFormType("exchange")}
                  className="p-6 bg-slate-900/50 border-2 border-violet-500/50 rounded-xl hover:border-fuchsia-500 hover:bg-slate-800/70 transition-all group"
                >
                  <svg className="w-12 h-12 mx-auto mb-3 text-violet-400 group-hover:text-fuchsia-400 transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                  </svg>
                  <h4 className="text-lg font-bold text-fuchsia-300 mb-1">Exchange</h4>
                  <p className="text-sm text-slate-400">OKX API credentials</p>
                </button>
              </div>
              <div className="mt-4">
                <button
                  onClick={closeForm}
                  className="inline-flex items-center px-6 py-2 border-2 border-slate-600 text-sm font-medium rounded-lg text-slate-300 bg-slate-900/50 hover:bg-slate-800/70 hover:border-slate-500 transition-all duration-300"
                >
                  Cancel
                </button>
              </div>
            </>
          ) : formType === "wallet" ? (
            <>
              <h3 className="text-xl font-bold text-fuchsia-300 mb-4 drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
                Add EVM Wallet
              </h3>
              <form onSubmit={handleCreateWallet} className="space-y-4">
                <div>
                  <label htmlFor="wallet-name" className="block text-sm font-medium text-slate-300 mb-2">
                    Wallet Name <span className="text-fuchsia-400">*</span>
                  </label>
                  <input
                    type="text"
                    id="wallet-name"
                    value={walletFormData.name}
                    onChange={(e) => setWalletFormData({ ...walletFormData, name: e.target.value })}
                    placeholder="e.g., My Main Wallet"
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                    required
                  />
                </div>
                <div>
                  <label htmlFor="wallet-address" className="block text-sm font-medium text-slate-300 mb-2">
                    Wallet Address <span className="text-fuchsia-400">*</span>
                  </label>
                  <input
                    type="text"
                    id="wallet-address"
                    value={walletFormData.wallet_address}
                    onChange={(e) => setWalletFormData({ ...walletFormData, wallet_address: e.target.value })}
                    placeholder="0x..."
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 font-mono focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                    required
                  />
                </div>
                <div className="flex gap-3">
                  <LoadingButton
                    type="submit"
                    loading={createAccount.isPending}
                    disabled={createAccount.isPending}
                    className="inline-flex items-center px-6 py-2 border-2 border-fuchsia-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 disabled:opacity-50 disabled:cursor-not-allowed shadow-[0_0_20px_rgba(217,70,239,0.4)] hover:shadow-[0_0_30px_rgba(217,70,239,0.6)] transition-all duration-300"
                  >
                    {createAccount.isPending ? "Creating..." : "Create Wallet"}
                  </LoadingButton>
                  <button
                    type="button"
                    onClick={closeForm}
                    className="inline-flex items-center px-6 py-2 border-2 border-slate-600 text-sm font-medium rounded-lg text-slate-300 bg-slate-900/50 hover:bg-slate-800/70 hover:border-slate-500 transition-all duration-300"
                  >
                    Cancel
                  </button>
                </div>
              </form>
            </>
          ) : (
            <>
              <h3 className="text-xl font-bold text-fuchsia-300 mb-4 drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
                Add Exchange Account
              </h3>
              <form onSubmit={handleCreateExchange} className="space-y-4">
                <div>
                  <label htmlFor="exchange-name" className="block text-sm font-medium text-slate-300 mb-2">
                    Account Name <span className="text-fuchsia-400">*</span>
                  </label>
                  <input
                    type="text"
                    id="exchange-name"
                    value={exchangeFormData.name}
                    onChange={(e) => setExchangeFormData({ ...exchangeFormData, name: e.target.value })}
                    placeholder="e.g., My OKX Account"
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                    required
                  />
                </div>
                <div>
                  <label htmlFor="exchange-type" className="block text-sm font-medium text-slate-300 mb-2">
                    Exchange <span className="text-fuchsia-400">*</span>
                  </label>
                  <select
                    id="exchange-type"
                    value={exchangeFormData.exchange_name}
                    onChange={(e) => setExchangeFormData({ ...exchangeFormData, exchange_name: e.target.value })}
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                  >
                    <option value="okx">OKX</option>
                  </select>
                </div>
                <div>
                  <label htmlFor="api-key" className="block text-sm font-medium text-slate-300 mb-2">
                    API Key <span className="text-fuchsia-400">*</span>
                  </label>
                  <input
                    type="text"
                    id="api-key"
                    value={exchangeFormData.api_key}
                    onChange={(e) => setExchangeFormData({ ...exchangeFormData, api_key: e.target.value })}
                    placeholder="Enter API key"
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 font-mono focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                    required
                  />
                </div>
                <div>
                  <label htmlFor="api-secret" className="block text-sm font-medium text-slate-300 mb-2">
                    API Secret <span className="text-fuchsia-400">*</span>
                  </label>
                  <input
                    type="password"
                    id="api-secret"
                    value={exchangeFormData.api_secret}
                    onChange={(e) => setExchangeFormData({ ...exchangeFormData, api_secret: e.target.value })}
                    placeholder="Enter API secret"
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 font-mono focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                    required
                  />
                </div>
                <div>
                  <label htmlFor="passphrase" className="block text-sm font-medium text-slate-300 mb-2">
                    Passphrase (Optional for OKX)
                  </label>
                  <input
                    type="password"
                    id="passphrase"
                    value={exchangeFormData.passphrase}
                    onChange={(e) => setExchangeFormData({ ...exchangeFormData, passphrase: e.target.value })}
                    placeholder="Enter passphrase if required"
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 font-mono focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                  />
                </div>
                <div className="flex gap-3">
                  <LoadingButton
                    type="submit"
                    loading={createAccount.isPending}
                    disabled={createAccount.isPending}
                    className="inline-flex items-center px-6 py-2 border-2 border-fuchsia-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 disabled:opacity-50 disabled:cursor-not-allowed shadow-[0_0_20px_rgba(217,70,239,0.4)] hover:shadow-[0_0_30px_rgba(217,70,239,0.6)] transition-all duration-300"
                  >
                    {createAccount.isPending ? "Creating..." : "Create Exchange Account"}
                  </LoadingButton>
                  <button
                    type="button"
                    onClick={closeForm}
                    className="inline-flex items-center px-6 py-2 border-2 border-slate-600 text-sm font-medium rounded-lg text-slate-300 bg-slate-900/50 hover:bg-slate-800/70 hover:border-slate-500 transition-all duration-300"
                  >
                    Cancel
                  </button>
                </div>
              </form>
            </>
          )}
        </div>
      )}

      {/* Loading State */}
      {loading && <LoadingSkeleton count={3} type="card" />}

      {/* Empty State */}
      {!loading && accounts.length === 0 && (
        <EmptyState
          icon="account"
          title="No accounts yet"
          description="Add a wallet or exchange account to get started"
          action={{
            label: "New Account",
            onClick: () => openCreateForm(null),
          }}
        />
      )}

      {!loading && accounts.length > 0 && (
        <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {accounts.map((account) => (
            <div
              key={account.id}
              className="group bg-slate-900/60 backdrop-blur-sm rounded-xl border-2 border-fuchsia-500/50 hover:border-fuchsia-400 p-6 transition-all duration-300 shadow-[0_0_20px_rgba(217,70,239,0.25)] hover:shadow-[0_0_35px_rgba(217,70,239,0.45)] hover:transform hover:scale-[1.03]"
            >
              <div className="flex items-start justify-between mb-3">
                <div className="flex-1">
                  <h3 className="text-lg font-bold text-fuchsia-300 group-hover:text-fuchsia-200 transition-colors drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
                    {account.name}
                  </h3>
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
                <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-fuchsia-500 to-purple-600 flex items-center justify-center shadow-[0_0_15px_rgba(217,70,239,0.4)] group-hover:shadow-[0_0_20px_rgba(217,70,239,0.6)] transition-all">
                  <svg className="w-5 h-5 text-white drop-shadow-[0_0_6px_rgba(255,255,255,0.5)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    {account.account_type === "wallet" ? (
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
                    ) : (
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
                    )}
                  </svg>
                </div>
              </div>
              
              {account.account_type === "wallet" && account.wallet_address && (
                <p className="text-slate-400 text-xs mb-3 font-mono break-all">
                  {account.wallet_address.length > 18 
                    ? `${account.wallet_address.slice(0, 10)}...${account.wallet_address.slice(-8)}`
                    : account.wallet_address
                  }
                </p>
              )}
              
              {account.account_type === "exchange" && account.exchange_name && (
                <p className="text-slate-400 text-xs mb-3 uppercase">
                  {account.exchange_name}
                </p>
              )}
              
              <div className="text-xs text-slate-500 space-y-1 mb-4">
                <p className="flex items-center gap-2">
                  <span>Last sync:</span>
                  <span className={account.last_synced_at ? "text-cyan-400" : "text-slate-500"}>
                    {formatDate(account.last_synced_at)}
                  </span>
                </p>
              </div>

              <div className="flex gap-2">
                <LoadingButton
                  onClick={() => handleSyncAccount(account.id)}
                  loading={syncAccount.isPending}
                  disabled={syncAccount.isPending}
                  className="flex-1 inline-flex items-center justify-center px-3 py-2 border border-cyan-500/50 text-xs font-medium rounded-lg text-cyan-300 bg-cyan-950/30 hover:bg-cyan-900/50 hover:border-cyan-400 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
                >
                  <svg className="w-4 h-4 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                  {syncAccount.isPending ? "Syncing..." : "Sync"}
                </LoadingButton>
                {deletingAccount === account.id ? (
                  <>
                    <button
                      onClick={() => handleDeleteAccount(account.id)}
                      className="px-3 py-2 border border-red-500 text-xs font-medium rounded-lg text-white bg-red-600 hover:bg-red-700 transition-all"
                    >
                      Confirm
                    </button>
                    <button
                      onClick={cancelDelete}
                      className="px-3 py-2 border border-slate-600 text-xs font-medium rounded-lg text-slate-300 bg-slate-900/50 hover:bg-slate-800/70 transition-all"
                    >
                      Cancel
                    </button>
                  </>
                ) : (
                  <button
                    onClick={() => confirmDelete(account.id)}
                    className="px-3 py-2 border border-red-500/50 text-xs font-medium rounded-lg text-red-300 bg-red-950/30 hover:bg-red-900/50 hover:border-red-400 transition-all"
                  >
                    <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                    </svg>
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
