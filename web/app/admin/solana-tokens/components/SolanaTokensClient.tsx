"use client";

import { useState } from "react";
import { ApiError } from "@/lib/api-client";
import { useToast } from "@/contexts/ToastContext";
import { LoadingSkeleton, LoadingButton } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import ErrorAlert from "@/components/ErrorAlert";
import {
  useSolanaTokens,
  useCreateSolanaToken,
  useUpdateSolanaToken,
  useDeleteSolanaToken,
} from "@/hooks";
import type { SolanaToken } from "@/types/api";

export default function SolanaTokensClient() {
  const toast = useToast();
  const [showForm, setShowForm] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [editingToken, setEditingToken] = useState<SolanaToken | null>(null);
  const [formData, setFormData] = useState({
    symbol: "",
    mint_address: "",
    is_active: true,
  });

  const { data: tokens = [], isLoading, error: queryError } = useSolanaTokens();
  const createToken = useCreateSolanaToken();
  const deleteToken = useDeleteSolanaToken();
  const updateToken = useUpdateSolanaToken(editingToken?.id ?? "");

  const error = queryError instanceof ApiError ? queryError.message :
    queryError ? "Failed to load Solana tokens" : null;

  function openCreateForm() {
    setEditingToken(null);
    setFormData({ symbol: "", mint_address: "", is_active: true });
    setShowForm(true);
  }

  function openEditForm(token: SolanaToken) {
    setEditingToken(token);
    setFormData({
      symbol: token.symbol,
      mint_address: token.mint_address,
      is_active: token.is_active,
    });
    setShowForm(true);
  }

  function closeForm() {
    setShowForm(false);
    setEditingToken(null);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!formData.symbol.trim()) {
      toast.error("Symbol is required");
      return;
    }
    if (!editingToken && !formData.mint_address.trim()) {
      toast.error("Mint address is required");
      return;
    }

    try {
      if (editingToken) {
        await updateToken.mutateAsync({
          symbol: formData.symbol.trim(),
          is_active: formData.is_active,
        });
        toast.success("Token updated successfully!");
      } else {
        await createToken.mutateAsync({
          symbol: formData.symbol.trim(),
          mint_address: formData.mint_address.trim(),
          is_active: formData.is_active,
        });
        toast.success("Token added successfully!");
      }
      closeForm();
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Operation failed";
      toast.error(message);
    }
  }

  async function handleDelete(tokenId: string) {
    try {
      await deleteToken.mutateAsync(tokenId);
      toast.success("Token deleted successfully!");
      setDeletingId(null);
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to delete token";
      toast.error(message);
      setDeletingId(null);
    }
  }

  const isPending = createToken.isPending || updateToken.isPending;

  return (
    <div className="space-y-6">
      {/* Header row */}
      <div className="flex items-center justify-end">
        <button
          onClick={openCreateForm}
          className="px-4 py-2 rounded-lg bg-gradient-to-r from-fuchsia-600 to-violet-600 text-white font-medium text-sm hover:from-fuchsia-500 hover:to-violet-500 transition-all shadow-[0_0_15px_rgba(217,70,239,0.4)]"
        >
          + Add Token
        </button>
      </div>

      {/* Error */}
      {error && <ErrorAlert message={error} />}

      {/* Add / Edit form */}
      {showForm && (
        <div className="bg-slate-900/60 border border-fuchsia-500/30 rounded-xl p-5">
          <h4 className="text-lg font-bold text-fuchsia-300 mb-4">
            {editingToken ? "Edit Token" : "Add Solana Token"}
          </h4>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="block text-sm text-slate-300 mb-1">Symbol</label>
              <input
                type="text"
                value={formData.symbol}
                onChange={(e) => setFormData({ ...formData, symbol: e.target.value })}
                placeholder="e.g. USDC"
                className="w-full bg-slate-800/60 border border-slate-600/50 rounded-lg px-3 py-2 text-slate-200 text-sm focus:outline-none focus:border-fuchsia-500/70"
                required
              />
            </div>

            {!editingToken && (
              <div>
                <label className="block text-sm text-slate-300 mb-1">Mint Address</label>
                <input
                  type="text"
                  value={formData.mint_address}
                  onChange={(e) => setFormData({ ...formData, mint_address: e.target.value })}
                  placeholder="Base58 mint address, e.g. EPjFWdd5..."
                  className="w-full bg-slate-800/60 border border-slate-600/50 rounded-lg px-3 py-2 text-slate-200 font-mono text-sm focus:outline-none focus:border-fuchsia-500/70"
                  required
                />
              </div>
            )}

            <div className="flex items-center gap-3">
              <input
                type="checkbox"
                id="is_active_solana_token"
                checked={formData.is_active}
                onChange={(e) => setFormData({ ...formData, is_active: e.target.checked })}
                className="w-4 h-4 accent-fuchsia-500"
              />
              <label htmlFor="is_active_solana_token" className="text-sm text-slate-300">
                Active (include in account sync)
              </label>
            </div>

            <div className="flex gap-3 pt-2">
              <LoadingButton
                type="submit"
                loading={isPending}
                className="px-4 py-2 rounded-lg bg-gradient-to-r from-fuchsia-600 to-violet-600 text-white font-medium text-sm hover:from-fuchsia-500 hover:to-violet-500 transition-all"
              >
                {editingToken ? "Save Changes" : "Add Token"}
              </LoadingButton>
              <button
                type="button"
                onClick={closeForm}
                className="px-4 py-2 rounded-lg border border-slate-600/50 text-slate-300 text-sm hover:border-slate-400/60 transition-all"
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Token list */}
      {isLoading ? (
        <LoadingSkeleton count={4} />
      ) : tokens.length === 0 ? (
        <EmptyState
          title="No tokens found"
          description="No Solana SPL tokens configured yet. Add tokens to start tracking them during wallet sync."
        />
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/50">
                <th className="text-left py-3 px-3 text-slate-400 font-medium">Symbol</th>
                <th className="text-left py-3 px-3 text-slate-400 font-medium">Mint Address</th>
                <th className="text-left py-3 px-3 text-slate-400 font-medium">Status</th>
                <th className="text-right py-3 px-3 text-slate-400 font-medium">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-700/30">
              {tokens.map((token) => (
                <tr key={token.id} className="hover:bg-slate-800/30 transition-colors">
                  <td className="py-3 px-3 text-slate-200 font-medium">{token.symbol}</td>
                  <td className="py-3 px-3 text-slate-400 font-mono text-xs truncate max-w-xs">
                    {token.mint_address}
                  </td>
                  <td className="py-3 px-3">
                    <span
                      className={`px-2 py-1 rounded-md text-xs font-medium ${
                        token.is_active
                          ? "bg-emerald-500/20 text-emerald-300"
                          : "bg-slate-700/40 text-slate-400"
                      }`}
                    >
                      {token.is_active ? "Active" : "Inactive"}
                    </span>
                  </td>
                  <td className="py-3 px-3 text-right">
                    {deletingId === token.id ? (
                      <span className="flex items-center justify-end gap-2">
                        <span className="text-slate-400 text-xs">Confirm delete?</span>
                        <button
                          onClick={() => handleDelete(token.id)}
                          className="text-xs px-2 py-1 rounded bg-red-500/20 text-red-300 hover:bg-red-500/30"
                        >
                          Yes
                        </button>
                        <button
                          onClick={() => setDeletingId(null)}
                          className="text-xs px-2 py-1 rounded bg-slate-700/40 text-slate-300 hover:bg-slate-700/60"
                        >
                          No
                        </button>
                      </span>
                    ) : (
                      <span className="flex items-center justify-end gap-2">
                        <button
                          onClick={() => openEditForm(token)}
                          className="text-xs px-2 py-1 rounded bg-fuchsia-500/20 text-fuchsia-300 hover:bg-fuchsia-500/30 transition-colors"
                        >
                          Edit
                        </button>
                        <button
                          onClick={() => setDeletingId(token.id)}
                          className="text-xs px-2 py-1 rounded bg-red-500/20 text-red-300 hover:bg-red-500/30 transition-colors"
                        >
                          Delete
                        </button>
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
