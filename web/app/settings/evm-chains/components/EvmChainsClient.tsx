"use client";

import { useState } from "react";
import { ApiError } from "@/lib/api-client";
import { useToast } from "@/contexts/ToastContext";
import { LoadingSkeleton, LoadingButton } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import ErrorAlert from "@/components/ErrorAlert";
import {
  useEvmChains,
  useCreateEvmChain,
  useUpdateEvmChain,
  useDeleteEvmChain,
} from "@/hooks";
import type { EvmChain } from "@/types/api";

const defaultFormData = {
  chain_id: "",
  name: "",
  rpc_url: "",
  is_active: true,
};

export default function EvmChainsClient() {
  const toast = useToast();
  const [showForm, setShowForm] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [editingChain, setEditingChain] = useState<EvmChain | null>(null);
  const [formData, setFormData] = useState(defaultFormData);

  const { data: chains = [], isLoading, error: queryError } = useEvmChains();
  const createChain = useCreateEvmChain();
  const deleteChain = useDeleteEvmChain();
  const updateChain = useUpdateEvmChain(editingChain?.id ?? "");

  const error = queryError instanceof ApiError ? queryError.message :
    queryError ? "Failed to load EVM chains" : null;

  function openCreateForm() {
    setEditingChain(null);
    setFormData(defaultFormData);
    setShowForm(true);
  }

  function openEditForm(chain: EvmChain) {
    setEditingChain(chain);
    setFormData({
      chain_id: chain.chain_id,
      name: chain.name,
      rpc_url: chain.rpc_url,
      is_active: chain.is_active,
    });
    setShowForm(true);
  }

  function closeForm() {
    setShowForm(false);
    setEditingChain(null);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!formData.name.trim() || !formData.rpc_url.trim()) {
      toast.error("Name and RPC URL are required");
      return;
    }
    if (!editingChain && !formData.chain_id.trim()) {
      toast.error("Chain ID is required");
      return;
    }

    try {
      if (editingChain) {
        await updateChain.mutateAsync({
          name: formData.name.trim(),
          rpc_url: formData.rpc_url.trim(),
          is_active: formData.is_active,
        });
        toast.success("Chain updated successfully!");
      } else {
        await createChain.mutateAsync({
          chain_id: formData.chain_id.trim().toLowerCase(),
          name: formData.name.trim(),
          rpc_url: formData.rpc_url.trim(),
          is_active: formData.is_active,
        });
        toast.success("Chain added successfully!");
      }
      closeForm();
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Operation failed";
      toast.error(message);
    }
  }

  async function handleDelete(chainUuid: string) {
    try {
      await deleteChain.mutateAsync(chainUuid);
      toast.success("Chain deleted successfully!");
      setDeletingId(null);
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to delete chain";
      toast.error(message);
      setDeletingId(null);
    }
  }

  const isPending = createChain.isPending || updateChain.isPending;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <p className="text-slate-400 text-sm">
          {chains.length} chain{chains.length !== 1 ? "s" : ""} configured
        </p>
        <button
          onClick={openCreateForm}
          className="px-4 py-2 rounded-lg bg-gradient-to-r from-fuchsia-600 to-violet-600 text-white font-medium text-sm hover:from-fuchsia-500 hover:to-violet-500 transition-all shadow-[0_0_15px_rgba(217,70,239,0.4)]"
        >
          + Add Chain
        </button>
      </div>

      {/* Error */}
      {error && <ErrorAlert message={error} />}

      {/* Add / Edit form */}
      {showForm && (
        <div className="bg-slate-900/60 border border-fuchsia-500/30 rounded-xl p-5">
          <h4 className="text-lg font-bold text-fuchsia-300 mb-4">
            {editingChain ? "Edit Chain" : "Add EVM Chain"}
          </h4>
          <form onSubmit={handleSubmit} className="space-y-4">
            {!editingChain && (
              <div>
                <label className="block text-sm text-slate-300 mb-1">
                  Chain ID
                  <span className="text-slate-500 ml-1 text-xs">(unique identifier, e.g. &quot;ethereum&quot;)</span>
                </label>
                <input
                  type="text"
                  value={formData.chain_id}
                  onChange={(e) => setFormData({ ...formData, chain_id: e.target.value })}
                  placeholder="e.g. avalanche"
                  className="w-full bg-slate-800/60 border border-slate-600/50 rounded-lg px-3 py-2 text-slate-200 font-mono text-sm focus:outline-none focus:border-fuchsia-500/70"
                  required
                />
              </div>
            )}

            <div>
              <label className="block text-sm text-slate-300 mb-1">Name</label>
              <input
                type="text"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="e.g. Avalanche C-Chain"
                className="w-full bg-slate-800/60 border border-slate-600/50 rounded-lg px-3 py-2 text-slate-200 text-sm focus:outline-none focus:border-fuchsia-500/70"
                required
              />
            </div>

            <div>
              <label className="block text-sm text-slate-300 mb-1">RPC URL</label>
              <input
                type="url"
                value={formData.rpc_url}
                onChange={(e) => setFormData({ ...formData, rpc_url: e.target.value })}
                placeholder="https://rpc.example.com"
                className="w-full bg-slate-800/60 border border-slate-600/50 rounded-lg px-3 py-2 text-slate-200 font-mono text-sm focus:outline-none focus:border-fuchsia-500/70"
                required
              />
            </div>

            <div className="flex items-center gap-3">
              <input
                type="checkbox"
                id="is_active_chain"
                checked={formData.is_active}
                onChange={(e) => setFormData({ ...formData, is_active: e.target.checked })}
                className="w-4 h-4 accent-fuchsia-500"
              />
              <label htmlFor="is_active_chain" className="text-sm text-slate-300">
                Active (include in account sync)
              </label>
            </div>

            <div className="flex gap-3 pt-2">
              <LoadingButton
                type="submit"
                loading={isPending}
                className="px-4 py-2 rounded-lg bg-gradient-to-r from-fuchsia-600 to-violet-600 text-white font-medium text-sm hover:from-fuchsia-500 hover:to-violet-500 transition-all"
              >
                {editingChain ? "Save Changes" : "Add Chain"}
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

      {/* Chain list */}
      {isLoading ? (
        <LoadingSkeleton count={5} />
      ) : chains.length === 0 ? (
        <EmptyState
          title="No chains configured"
          description="Add your first EVM chain to get started."
        />
      ) : (
        <div className="space-y-3">
          {chains.map((chain) => (
            <div
              key={chain.id}
              className="bg-slate-900/40 border border-slate-700/40 rounded-xl p-4 hover:border-fuchsia-500/30 transition-colors"
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3 mb-1">
                    <span className="font-semibold text-slate-100">{chain.name}</span>
                    <span className="px-2 py-0.5 rounded-md bg-violet-500/20 text-violet-300 text-xs font-mono">
                      {chain.chain_id}
                    </span>
                    <span
                      className={`px-2 py-0.5 rounded-md text-xs font-medium ${
                        chain.is_active
                          ? "bg-emerald-500/20 text-emerald-300"
                          : "bg-slate-700/40 text-slate-400"
                      }`}
                    >
                      {chain.is_active ? "Active" : "Inactive"}
                    </span>
                  </div>
                  <p className="text-slate-400 font-mono text-xs truncate">{chain.rpc_url}</p>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  {deletingId === chain.id ? (
                    <>
                      <span className="text-slate-400 text-xs">Confirm?</span>
                      <button
                        onClick={() => handleDelete(chain.id)}
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
                    </>
                  ) : (
                    <>
                      <button
                        onClick={() => openEditForm(chain)}
                        className="text-xs px-3 py-1.5 rounded-lg bg-fuchsia-500/20 text-fuchsia-300 hover:bg-fuchsia-500/30 transition-colors"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => setDeletingId(chain.id)}
                        className="text-xs px-3 py-1.5 rounded-lg bg-red-500/20 text-red-300 hover:bg-red-500/30 transition-colors"
                      >
                        Delete
                      </button>
                    </>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
