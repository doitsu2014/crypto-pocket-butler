"use client";

import { useState, useEffect } from "react";
import { apiClient } from "@/lib/api-client";
import Link from "next/link";

interface Portfolio {
  id: string;
  user_id: string;
  name: string;
  description?: string;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

function formatDate(dateString: string): string {
  try {
    const date = new Date(dateString);
    if (isNaN(date.getTime())) {
      return 'N/A';
    }
    return date.toLocaleDateString();
  } catch {
    return 'N/A';
  }
}

export default function PortfoliosClient() {
  const [portfolios, setPortfolios] = useState<Portfolio[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [creating, setCreating] = useState(false);
  const [formData, setFormData] = useState({
    name: "",
    description: "",
    is_default: false,
  });

  useEffect(() => {
    loadPortfolios();
  }, []);

  async function loadPortfolios() {
    try {
      setLoading(true);
      setError(null);
      const data = await apiClient<Portfolio[]>("/v1/portfolios");
      setPortfolios(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load portfolios");
    } finally {
      setLoading(false);
    }
  }

  async function handleCreatePortfolio(e: React.FormEvent) {
    e.preventDefault();
    if (!formData.name.trim()) {
      setError("Portfolio name is required");
      return;
    }

    try {
      setCreating(true);
      setError(null);
      await apiClient<Portfolio>("/v1/portfolios", {
        method: "POST",
        body: {
          name: formData.name.trim(),
          description: formData.description.trim() || undefined,
          is_default: formData.is_default,
        },
      });
      
      // Reset form and reload portfolios
      setFormData({ name: "", description: "", is_default: false });
      setShowCreateForm(false);
      await loadPortfolios();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create portfolio");
    } finally {
      setCreating(false);
    }
  }

  return (
    <>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-fuchsia-500 to-violet-600 flex items-center justify-center shadow-[0_0_20px_rgba(217,70,239,0.5)]">
            <svg className="w-7 h-7 text-white drop-shadow-[0_0_8px_rgba(255,255,255,0.6)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
          </div>
          <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent drop-shadow-[0_0_12px_rgba(232,121,249,0.4)]">
            Portfolios
          </h2>
        </div>
        <button
          onClick={() => setShowCreateForm(!showCreateForm)}
          className="inline-flex items-center px-6 py-2 border-2 border-fuchsia-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 shadow-[0_0_20px_rgba(217,70,239,0.4)] hover:shadow-[0_0_30px_rgba(217,70,239,0.6)] transition-all duration-300 transform hover:scale-105"
        >
          <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          {showCreateForm ? "Cancel" : "New Portfolio"}
        </button>
      </div>

      {/* Error Display */}
      {error && (
        <div className="mb-6 bg-red-950/30 border-2 border-red-500/50 rounded-xl p-4 shadow-[0_0_20px_rgba(239,68,68,0.25)]">
          <p className="text-red-300 text-sm">
            {error}
          </p>
        </div>
      )}

      {/* Create Portfolio Form */}
      {showCreateForm && (
        <div className="mb-6 bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40 shadow-[0_0_25px_rgba(217,70,239,0.3)] rounded-2xl p-6">
          <h3 className="text-xl font-bold text-fuchsia-300 mb-4 drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
            Create New Portfolio
          </h3>
          <form onSubmit={handleCreatePortfolio} className="space-y-4">
            <div>
              <label htmlFor="name" className="block text-sm font-medium text-slate-300 mb-2">
                Portfolio Name <span className="text-fuchsia-400">*</span>
              </label>
              <input
                type="text"
                id="name"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="e.g., Main Portfolio"
                className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                required
              />
            </div>
            <div>
              <label htmlFor="description" className="block text-sm font-medium text-slate-300 mb-2">
                Description (Optional)
              </label>
              <textarea
                id="description"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                placeholder="Describe your portfolio..."
                rows={3}
                className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
              />
            </div>
            <div>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.is_default}
                  onChange={(e) => setFormData({ ...formData, is_default: e.target.checked })}
                  className="w-5 h-5 rounded border-2 border-violet-500 bg-slate-900/50 text-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/50"
                />
                <span className="text-sm text-slate-300">Set as default portfolio</span>
              </label>
            </div>
            <div className="flex gap-3">
              <button
                type="submit"
                disabled={creating}
                className="inline-flex items-center px-6 py-2 border-2 border-fuchsia-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 disabled:opacity-50 disabled:cursor-not-allowed shadow-[0_0_20px_rgba(217,70,239,0.4)] hover:shadow-[0_0_30px_rgba(217,70,239,0.6)] transition-all duration-300"
              >
                {creating ? "Creating..." : "Create Portfolio"}
              </button>
              <button
                type="button"
                onClick={() => {
                  setShowCreateForm(false);
                  setFormData({ name: "", description: "", is_default: false });
                  setError(null);
                }}
                className="inline-flex items-center px-6 py-2 border-2 border-slate-600 text-sm font-medium rounded-lg text-slate-300 bg-slate-900/50 hover:bg-slate-800/70 hover:border-slate-500 transition-all duration-300"
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Loading State */}
      {loading && (
        <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-2xl p-8">
          <div className="animate-pulse space-y-4">
            <div className="h-6 bg-violet-900/50 rounded w-3/4 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
            <div className="h-4 bg-violet-900/50 rounded w-1/2 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
          </div>
        </div>
      )}

      {/* Portfolios List */}
      {!loading && portfolios.length === 0 && (
        <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-cyan-500/40 shadow-[0_0_25px_rgba(34,211,238,0.3)] rounded-2xl p-8 text-center">
          <svg className="w-16 h-16 mx-auto mb-4 text-cyan-400 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
          <p className="text-slate-300 text-lg mb-2">No portfolios yet</p>
          <p className="text-slate-400 text-sm">Create your first portfolio to get started</p>
        </div>
      )}

      {!loading && portfolios.length > 0 && (
        <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {portfolios.map((portfolio) => (
            <div
              key={portfolio.id}
              className="group bg-slate-900/60 backdrop-blur-sm rounded-xl border-2 border-fuchsia-500/50 hover:border-fuchsia-400 p-6 transition-all duration-300 shadow-[0_0_20px_rgba(217,70,239,0.25)] hover:shadow-[0_0_35px_rgba(217,70,239,0.45)] hover:transform hover:scale-[1.03] cursor-pointer"
            >
              <div className="flex items-start justify-between mb-3">
                <div className="flex-1">
                  <h3 className="text-lg font-bold text-fuchsia-300 group-hover:text-fuchsia-200 transition-colors drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
                    {portfolio.name}
                  </h3>
                  {portfolio.is_default && (
                    <span className="inline-flex items-center gap-1 mt-1 text-xs text-cyan-400 drop-shadow-[0_0_8px_rgba(34,211,238,0.4)]">
                      <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                        <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                      </svg>
                      Default
                    </span>
                  )}
                </div>
                <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-fuchsia-500 to-purple-600 flex items-center justify-center shadow-[0_0_15px_rgba(217,70,239,0.4)] group-hover:shadow-[0_0_20px_rgba(217,70,239,0.6)] transition-all">
                  <svg className="w-5 h-5 text-white drop-shadow-[0_0_6px_rgba(255,255,255,0.5)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                  </svg>
                </div>
              </div>
              {portfolio.description && (
                <p className="text-slate-400 text-sm mb-3 line-clamp-2">
                  {portfolio.description}
                </p>
              )}
              <div className="text-xs text-slate-500 space-y-1">
                <p>Created: {formatDate(portfolio.created_at)}</p>
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
