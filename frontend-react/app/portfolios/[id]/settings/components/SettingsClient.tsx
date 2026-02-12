"use client";

import { useState, useEffect, useCallback } from "react";
import { apiClient, ApiError } from "@/lib/api-client";
import { useToast } from "@/contexts/ToastContext";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { LoadingSkeleton, LoadingButton } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import ErrorAlert from "@/components/ErrorAlert";

interface Portfolio {
  id: string;
  user_id: string;
  name: string;
  description?: string;
  is_default: boolean;
  target_allocation?: Record<string, number> | null;
  guardrails?: {
    drift_band?: number;
    stablecoin_min?: number;
    futures_cap?: number;
    max_alt_cap?: number;
  } | null;
  created_at: string;
  updated_at: string;
}

interface TargetAllocationEntry {
  asset: string;
  percentage: number;
}

export default function SettingsClient({ portfolioId }: { portfolioId: string }) {
  const router = useRouter();
  const toast = useToast();
  const [portfolio, setPortfolio] = useState<Portfolio | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Target allocation state
  const [targetEntries, setTargetEntries] = useState<TargetAllocationEntry[]>([
    { asset: "", percentage: 0 }
  ]);
  
  // Guardrails state
  const [guardrails, setGuardrails] = useState({
    drift_band: 5,
    stablecoin_min: 0,
    futures_cap: 100,
    max_alt_cap: 100,
  });

  const loadPortfolio = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await apiClient<Portfolio>(`/v1/portfolios/${portfolioId}`);
      setPortfolio(data);
      
      // Initialize target allocation
      if (data.target_allocation) {
        const entries = Object.entries(data.target_allocation).map(([asset, percentage]) => ({
          asset,
          percentage: Number(percentage),
        }));
        if (entries.length > 0) {
          setTargetEntries(entries);
        }
      }
      
      // Initialize guardrails
      if (data.guardrails) {
        setGuardrails({
          drift_band: data.guardrails.drift_band ?? 5,
          stablecoin_min: data.guardrails.stablecoin_min ?? 0,
          futures_cap: data.guardrails.futures_cap ?? 100,
          max_alt_cap: data.guardrails.max_alt_cap ?? 100,
        });
      }
    } catch (err) {
      const errorMessage = err instanceof ApiError ? err.message : "Failed to load portfolio";
      setError(errorMessage);
      toast.error(errorMessage);
    } finally {
      setLoading(false);
    }
  }, [portfolioId, toast]);

  useEffect(() => {
    loadPortfolio();
  }, [loadPortfolio]);

  function addTargetEntry() {
    setTargetEntries([...targetEntries, { asset: "", percentage: 0 }]);
  }

  function removeTargetEntry(index: number) {
    if (targetEntries.length > 1) {
      setTargetEntries(targetEntries.filter((_, i) => i !== index));
    }
  }

  function updateTargetEntry(index: number, field: "asset" | "percentage", value: string | number) {
    const newEntries = [...targetEntries];
    if (field === "asset") {
      newEntries[index].asset = (value as string).toUpperCase();
    } else {
      newEntries[index].percentage = Number(value);
    }
    setTargetEntries(newEntries);
  }

  function validateForm(): string | null {
    // Validate target allocation
    const totalPercentage = targetEntries.reduce((sum, entry) => sum + entry.percentage, 0);
    
    // Check for empty assets
    const hasEmptyAssets = targetEntries.some(entry => !entry.asset.trim());
    if (hasEmptyAssets) {
      return "All asset names must be filled in";
    }
    
    // Check for duplicate assets
    const assetNames = targetEntries.map(e => e.asset.trim().toUpperCase());
    const uniqueAssets = new Set(assetNames);
    if (assetNames.length !== uniqueAssets.size) {
      return "Duplicate asset names are not allowed";
    }
    
    // Check total percentage
    if (Math.abs(totalPercentage - 100) > 0.01) {
      return `Target allocation must sum to 100% (currently ${totalPercentage.toFixed(2)}%)`;
    }
    
    // Check individual percentages
    const hasNegative = targetEntries.some(entry => entry.percentage < 0);
    if (hasNegative) {
      return "Target percentages cannot be negative";
    }
    
    // Validate guardrails
    if (guardrails.drift_band < 0 || guardrails.drift_band > 100) {
      return "Drift band must be between 0 and 100";
    }
    if (guardrails.stablecoin_min < 0 || guardrails.stablecoin_min > 100) {
      return "Stablecoin minimum must be between 0 and 100";
    }
    if (guardrails.futures_cap < 0 || guardrails.futures_cap > 100) {
      return "Futures cap must be between 0 and 100";
    }
    if (guardrails.max_alt_cap < 0 || guardrails.max_alt_cap > 100) {
      return "Max alt cap must be between 0 and 100";
    }
    
    return null;
  }

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    
    const validationError = validateForm();
    if (validationError) {
      setError(validationError);
      toast.error(validationError);
      return;
    }
    
    try {
      setSaving(true);
      setError(null);
      
      // Convert target entries to object
      const target_allocation: Record<string, number> = {};
      targetEntries.forEach(entry => {
        if (entry.asset.trim()) {
          target_allocation[entry.asset.trim().toUpperCase()] = entry.percentage;
        }
      });
      
      await apiClient(`/v1/portfolios/${portfolioId}`, {
        method: "PUT",
        body: {
          target_allocation,
          guardrails,
        },
      });
      
      toast.success("Settings saved successfully!");
      await loadPortfolio();
    } catch (err) {
      const errorMessage = err instanceof ApiError ? err.message : "Failed to save settings";
      setError(errorMessage);
      toast.error(errorMessage);
    } finally {
      setSaving(false);
    }
  }

  const totalPercentage = targetEntries.reduce((sum, entry) => sum + entry.percentage, 0);
  const isValidTotal = Math.abs(totalPercentage - 100) < 0.01;

  if (loading) {
    return <LoadingSkeleton type="list" count={2} />;
  }

  if (!portfolio) {
    return (
      <EmptyState
        icon="settings"
        title="Portfolio not found"
        description="This portfolio doesn't exist or you don't have access to it"
      />
    );
  }

  return (
    <>
      {/* Breadcrumb */}
      <div className="mb-6 flex items-center gap-2 text-sm">
        <Link 
          href="/portfolios" 
          className="text-cyan-300 hover:text-cyan-200 transition-colors drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]"
        >
          Portfolios
        </Link>
        <span className="text-slate-500">/</span>
        <Link 
          href={`/portfolios/${portfolioId}`}
          className="text-cyan-300 hover:text-cyan-200 transition-colors drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]"
        >
          {portfolio.name}
        </Link>
        <span className="text-slate-500">/</span>
        <span className="text-slate-400">Settings</span>
      </div>

      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-fuchsia-500 to-violet-600 flex items-center justify-center shadow-[0_0_20px_rgba(217,70,239,0.5)]">
            <svg className="w-7 h-7 text-white drop-shadow-[0_0_8px_rgba(255,255,255,0.6)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </div>
          <div>
            <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent drop-shadow-[0_0_12px_rgba(232,121,249,0.4)]">
              Portfolio Settings
            </h2>
            <p className="text-slate-400 text-sm mt-1">{portfolio.name}</p>
          </div>
        </div>
        <Link
          href={`/portfolios/${portfolioId}`}
          className="inline-flex items-center px-4 py-2 border-2 border-cyan-500 text-sm font-bold rounded-lg text-white hover:bg-cyan-500/10 shadow-[0_0_15px_rgba(103,232,249,0.3)] hover:shadow-[0_0_25px_rgba(103,232,249,0.5)] transition-all duration-300"
        >
          <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
          </svg>
          Back to Portfolio
        </Link>
      </div>

      {/* Error Display */}
      {error && (
        <ErrorAlert 
          message={error}
          onDismiss={() => setError(null)}
        />
      )}

      <form onSubmit={handleSave} className="space-y-8">
        {/* Target Allocation Section */}
        <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40 shadow-[0_0_25px_rgba(217,70,239,0.3)] rounded-2xl p-6">
          <div className="flex items-center justify-between mb-6">
            <div>
              <h3 className="text-xl font-bold text-fuchsia-300 mb-2 drop-shadow-[0_0_8px_rgba(232,121,249,0.4)]">
                Target Allocation
              </h3>
              <p className="text-slate-400 text-sm">
                Define your desired asset allocation. Total must equal 100%.
              </p>
            </div>
            <div className={`text-lg font-bold ${isValidTotal ? 'text-green-400' : 'text-red-400'} ${isValidTotal ? 'drop-shadow-[0_0_8px_rgba(34,197,94,0.4)]' : 'drop-shadow-[0_0_8px_rgba(239,68,68,0.4)]'}`}>
              Total: {totalPercentage.toFixed(2)}%
            </div>
          </div>

          <div className="space-y-3">
            {targetEntries.map((entry, index) => (
              <div key={index} className="flex gap-3 items-center">
                <div className="flex-1">
                  <input
                    type="text"
                    value={entry.asset}
                    onChange={(e) => updateTargetEntry(index, "asset", e.target.value)}
                    placeholder="Asset (e.g., BTC)"
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                  />
                </div>
                <div className="w-32">
                  <input
                    type="number"
                    value={entry.percentage}
                    onChange={(e) => updateTargetEntry(index, "percentage", e.target.value)}
                    placeholder="0"
                    min="0"
                    max="100"
                    step="0.01"
                    className="w-full px-4 py-2 bg-slate-900/50 border-2 border-violet-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-fuchsia-500 focus:ring-2 focus:ring-fuchsia-500/40 shadow-[0_0_10px_rgba(139,92,246,0.15)] focus:shadow-[0_0_20px_rgba(217,70,239,0.3)] transition-all"
                  />
                </div>
                <span className="text-slate-400 w-6">%</span>
                <button
                  type="button"
                  onClick={() => removeTargetEntry(index)}
                  disabled={targetEntries.length === 1}
                  className="p-2 text-red-400 hover:text-red-300 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                >
                  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            ))}
          </div>

          <button
            type="button"
            onClick={addTargetEntry}
            className="mt-4 inline-flex items-center px-4 py-2 border-2 border-violet-500 text-sm font-bold rounded-lg text-white hover:bg-violet-500/10 shadow-[0_0_15px_rgba(139,92,246,0.3)] hover:shadow-[0_0_25px_rgba(139,92,246,0.5)] transition-all duration-300"
          >
            <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
            Add Asset
          </button>
        </div>

        {/* Guardrails Section */}
        <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-cyan-500/40 shadow-[0_0_25px_rgba(103,232,249,0.3)] rounded-2xl p-6">
          <div className="mb-6">
            <h3 className="text-xl font-bold text-cyan-300 mb-2 drop-shadow-[0_0_8px_rgba(103,232,249,0.4)]">
              Guardrails
            </h3>
            <p className="text-slate-400 text-sm">
              Set limits and thresholds for portfolio management.
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label htmlFor="drift_band" className="block text-sm font-medium text-slate-300 mb-2">
                Drift Band (%)
              </label>
              <p className="text-xs text-slate-500 mb-2">
                Maximum allowed deviation from target allocation
              </p>
              <input
                type="number"
                id="drift_band"
                value={guardrails.drift_band}
                onChange={(e) => setGuardrails({ ...guardrails, drift_band: Number(e.target.value) })}
                min="0"
                max="100"
                step="0.1"
                className="w-full px-4 py-2 bg-slate-900/50 border-2 border-cyan-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/40 shadow-[0_0_10px_rgba(103,232,249,0.15)] focus:shadow-[0_0_20px_rgba(103,232,249,0.3)] transition-all"
              />
            </div>

            <div>
              <label htmlFor="stablecoin_min" className="block text-sm font-medium text-slate-300 mb-2">
                Stablecoin Minimum (%)
              </label>
              <p className="text-xs text-slate-500 mb-2">
                Minimum percentage of stablecoins to maintain
              </p>
              <input
                type="number"
                id="stablecoin_min"
                value={guardrails.stablecoin_min}
                onChange={(e) => setGuardrails({ ...guardrails, stablecoin_min: Number(e.target.value) })}
                min="0"
                max="100"
                step="0.1"
                className="w-full px-4 py-2 bg-slate-900/50 border-2 border-cyan-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/40 shadow-[0_0_10px_rgba(103,232,249,0.15)] focus:shadow-[0_0_20px_rgba(103,232,249,0.3)] transition-all"
              />
            </div>

            <div>
              <label htmlFor="futures_cap" className="block text-sm font-medium text-slate-300 mb-2">
                Futures Cap (%)
              </label>
              <p className="text-xs text-slate-500 mb-2">
                Maximum percentage allocated to futures
              </p>
              <input
                type="number"
                id="futures_cap"
                value={guardrails.futures_cap}
                onChange={(e) => setGuardrails({ ...guardrails, futures_cap: Number(e.target.value) })}
                min="0"
                max="100"
                step="0.1"
                className="w-full px-4 py-2 bg-slate-900/50 border-2 border-cyan-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/40 shadow-[0_0_10px_rgba(103,232,249,0.15)] focus:shadow-[0_0_20px_rgba(103,232,249,0.3)] transition-all"
              />
            </div>

            <div>
              <label htmlFor="max_alt_cap" className="block text-sm font-medium text-slate-300 mb-2">
                Max Altcoin Cap (%)
              </label>
              <p className="text-xs text-slate-500 mb-2">
                Maximum percentage allocated to altcoins
              </p>
              <input
                type="number"
                id="max_alt_cap"
                value={guardrails.max_alt_cap}
                onChange={(e) => setGuardrails({ ...guardrails, max_alt_cap: Number(e.target.value) })}
                min="0"
                max="100"
                step="0.1"
                className="w-full px-4 py-2 bg-slate-900/50 border-2 border-cyan-500/50 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-500 focus:ring-2 focus:ring-cyan-500/40 shadow-[0_0_10px_rgba(103,232,249,0.15)] focus:shadow-[0_0_20px_rgba(103,232,249,0.3)] transition-all"
              />
            </div>
          </div>
        </div>

        {/* Save Button */}
        <div className="flex justify-end">
          <LoadingButton
            type="submit"
            loading={saving}
            className="inline-flex items-center px-8 py-3 border-2 border-fuchsia-500 text-base font-bold rounded-lg text-white bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 shadow-[0_0_20px_rgba(217,70,239,0.4)] hover:shadow-[0_0_30px_rgba(217,70,239,0.6)] transition-all duration-300 transform hover:scale-105 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
          >
            {saving ? (
              <>Saving...</>
            ) : (
              <>
                <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
                Save Settings
              </>
            )}
          </LoadingButton>
        </div>
      </form>
    </>
  );
}
