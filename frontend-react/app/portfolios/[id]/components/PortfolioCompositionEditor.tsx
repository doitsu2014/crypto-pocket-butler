"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { apiClient } from "@/lib/api-client";
import { useToast } from "@/contexts/ToastContext";
import ErrorAlert from "@/components/ErrorAlert";
import { LoadingSkeleton } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import { LoadingButton } from "@/components/Loading";

interface Account {
  id: string;
  name: string;
  account_type: string;
  exchange_name?: string;
  wallet_address?: string;
  is_active: boolean;
  last_synced_at?: string;
  created_at: string;
  updated_at: string;
}

interface PortfolioCompositionEditorProps {
  portfolioId: string;
  portfolioName: string;
  onUpdate?: () => void;
}

export default function PortfolioCompositionEditor({
  portfolioId,
  portfolioName,
  onUpdate,
}: PortfolioCompositionEditorProps) {
  const router = useRouter();
  const [showEditor, setShowEditor] = useState(false);
  const [allAccounts, setAllAccounts] = useState<Account[]>([]);
  const [portfolioAccounts, setPortfolioAccounts] = useState<Account[]>([]);
  const [selectedAccountIds, setSelectedAccountIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const toast = useToast();

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      // Load all user accounts and portfolio accounts in parallel
      const [accounts, portAccounts] = await Promise.all([
        apiClient<Account[]>("/v1/accounts"),
        apiClient<Account[]>(`/v1/portfolios/${portfolioId}/accounts`),
      ]);

      setAllAccounts(accounts);
      setPortfolioAccounts(portAccounts);
      
      // Initialize selected account IDs
      const selectedIds = new Set(portAccounts.map(a => a.id));
      setSelectedAccountIds(selectedIds);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load data');
    } finally {
      setLoading(false);
    }
  }, [portfolioId]);

  useEffect(() => {
    if (showEditor) {
      loadData();
    }
  }, [showEditor, loadData]);

  async function handleSave() {
    try {
      setSaving(true);
      setError(null);

      await apiClient(`/v1/portfolios/${portfolioId}/accounts`, {
        method: "PUT",
        body: {
          account_ids: Array.from(selectedAccountIds),
        },
      });

      toast.success("Portfolio composition updated successfully");

      setShowEditor(false);
      if (onUpdate) {
        onUpdate();
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : "Failed to save portfolio composition";
      setError(errorMessage);
      toast.error(errorMessage);
    } finally {
      setSaving(false);
    }
  }

  function toggleAccount(accountId: string) {
    const newSelected = new Set(selectedAccountIds);
    if (newSelected.has(accountId)) {
      newSelected.delete(accountId);
    } else {
      newSelected.add(accountId);
    }
    setSelectedAccountIds(newSelected);
  }

  function selectAll() {
    setSelectedAccountIds(new Set(allAccounts.map(a => a.id)));
  }

  function deselectAll() {
    setSelectedAccountIds(new Set());
  }

  return (
    <>
      {/* Edit Button */}
      <button
        onClick={() => setShowEditor(true)}
        className="inline-flex items-center gap-2 px-4 py-2 border-2 border-violet-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-violet-600 to-purple-700 hover:from-violet-500 hover:to-purple-600 shadow-[0_0_15px_rgba(139,92,246,0.4)] hover:shadow-[0_0_20px_rgba(139,92,246,0.6)] transition-all"
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
        </svg>
        Edit Accounts
      </button>

      {/* Modal */}
      {showEditor && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
          <div className="bg-slate-950 border-2 border-violet-500/50 shadow-[0_0_40px_rgba(139,92,246,0.5)] rounded-2xl w-full max-w-2xl max-h-[90vh] overflow-hidden flex flex-col">
            {/* Header */}
            <div className="p-6 border-b-2 border-violet-500/30">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-2xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent drop-shadow-[0_0_12px_rgba(232,121,249,0.4)]">
                    Edit Portfolio Composition
                  </h2>
                  <p className="text-slate-400 text-sm mt-1">
                    Select which accounts to include in "{portfolioName}"
                  </p>
                </div>
                <button
                  onClick={() => setShowEditor(false)}
                  className="text-slate-400 hover:text-white transition-colors"
                >
                  <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto p-6">
              {loading ? (
                <LoadingSkeleton type="list" count={4} />
              ) : error ? (
                <ErrorAlert message={error} onRetry={loadData} />
              ) : allAccounts.length === 0 ? (
                <EmptyState
                  icon="account"
                  title="No accounts available"
                  description="Create an account first to add it to this portfolio."
                  action={{
                    label: "Go to Accounts",
                    onClick: () => router.push("/accounts")
                  }}
                />
              ) : (
                <>
                  {/* Select/Deselect All */}
                  <div className="flex gap-2 mb-4">
                    <button
                      onClick={selectAll}
                      className="px-3 py-1 text-xs font-bold text-cyan-300 border border-cyan-500/50 rounded-lg hover:bg-cyan-500/10 transition-all"
                    >
                      Select All
                    </button>
                    <button
                      onClick={deselectAll}
                      className="px-3 py-1 text-xs font-bold text-slate-300 border border-slate-500/50 rounded-lg hover:bg-slate-500/10 transition-all"
                    >
                      Deselect All
                    </button>
                    <div className="flex-1"></div>
                    <span className="text-xs text-slate-400 py-1">
                      {selectedAccountIds.size} of {allAccounts.length} selected
                    </span>
                  </div>

                  {/* Account List */}
                  <div className="space-y-2">
                    {allAccounts.map((account) => {
                      const isSelected = selectedAccountIds.has(account.id);
                      return (
                        <div
                          key={account.id}
                          onClick={() => toggleAccount(account.id)}
                          className={`
                            flex items-center gap-3 p-4 rounded-xl cursor-pointer transition-all
                            ${
                              isSelected
                                ? "bg-violet-900/30 border-2 border-violet-500/60 shadow-[0_0_15px_rgba(139,92,246,0.3)]"
                                : "bg-slate-900/50 border-2 border-slate-700/50 hover:border-slate-600/60"
                            }
                          `}
                        >
                          {/* Checkbox */}
                          <div
                            className={`
                              w-5 h-5 rounded border-2 flex items-center justify-center transition-all
                              ${
                                isSelected
                                  ? "bg-violet-500 border-violet-400 shadow-[0_0_10px_rgba(139,92,246,0.5)]"
                                  : "border-slate-500"
                              }
                            `}
                          >
                            {isSelected && (
                              <svg className="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 20 20">
                                <path
                                  fillRule="evenodd"
                                  d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                                  clipRule="evenodd"
                                />
                              </svg>
                            )}
                          </div>

                          {/* Account Info */}
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2">
                              <span
                                className={`font-bold truncate ${
                                  isSelected ? "text-violet-200" : "text-slate-300"
                                }`}
                              >
                                {account.name}
                              </span>
                              <span
                                className={`
                                  px-2 py-0.5 text-xs font-bold rounded border
                                  ${
                                    account.account_type === "exchange"
                                      ? "text-fuchsia-300 border-fuchsia-500/50"
                                      : "text-cyan-300 border-cyan-500/50"
                                  }
                                `}
                              >
                                {account.account_type === "exchange"
                                  ? account.exchange_name?.toUpperCase() || "Exchange"
                                  : "Wallet"}
                              </span>
                            </div>
                            {account.wallet_address && (
                              <p className="text-xs text-slate-500 truncate mt-1">
                                {account.wallet_address}
                              </p>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </>
              )}
            </div>

            {/* Footer */}
            <div className="p-6 border-t-2 border-violet-500/30 flex gap-3 justify-end">
              <button
                onClick={() => setShowEditor(false)}
                disabled={saving}
                className="px-4 py-2 border-2 border-slate-600 text-sm font-bold rounded-lg text-slate-300 hover:bg-slate-800/50 transition-all disabled:opacity-50"
              >
                Cancel
              </button>
              <LoadingButton
                onClick={handleSave}
                loading={saving}
                disabled={allAccounts.length === 0}
                className="inline-flex items-center gap-2 px-4 py-2 border-2 border-violet-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-violet-600 to-purple-700 hover:from-violet-500 hover:to-purple-600 shadow-[0_0_15px_rgba(139,92,246,0.4)] hover:shadow-[0_0_20px_rgba(139,92,246,0.6)] transition-all disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
                Save Changes
              </LoadingButton>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
