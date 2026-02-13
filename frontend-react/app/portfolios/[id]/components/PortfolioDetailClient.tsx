"use client";

import { useState, useEffect, useCallback } from "react";
import { apiClient, ApiError } from "@/lib/api-client";
import { useToast } from "@/contexts/ToastContext";
import Link from "next/link";
import PortfolioCompositionEditor from "./PortfolioCompositionEditor";
import { LoadingSkeleton } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import ErrorAlert from "@/components/ErrorAlert";
import HoldingsTable from "@/components/portfolio/HoldingsTable";
import AllocationBar from "@/components/portfolio/AllocationBar";
import AllocationPie from "@/components/portfolio/AllocationPie";
import DriftBadge from "@/components/portfolio/DriftBadge";

const MAX_ALLOCATION_ITEMS = 10;

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

interface AccountHoldingDetail {
  account_id: string;
  account_name: string;
  quantity: string;
  available: string;
  frozen: string;
}

interface AssetHolding {
  asset: string;
  total_quantity: string;
  total_available: string;
  total_frozen: string;
  price_usd: number;
  value_usd: number;
  accounts: AccountHoldingDetail[];
}

interface AllocationItem {
  asset: string;
  value_usd: number;
  percentage: number;
}

interface PortfolioHoldingsResponse {
  portfolio_id: string;
  total_value_usd: number;
  holdings: AssetHolding[];
  allocation: AllocationItem[];
  as_of: string;
}

interface AllocationHolding {
  asset: string;
  quantity: string;
  value_usd: number;
  weight: number;
  price_usd?: number;
  unpriced: boolean;
}

interface ConstructedAllocationResponse {
  portfolio_id: string;
  total_value_usd: number;
  holdings: AllocationHolding[];
  as_of: string;
}

function formatCurrency(value: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

function formatPercentage(value: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'percent',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value / 100);
}

export default function PortfolioDetailClient({ portfolioId }: { portfolioId: string }) {
  const toast = useToast();
  const [portfolio, setPortfolio] = useState<Portfolio | null>(null);
  const [holdings, setHoldings] = useState<PortfolioHoldingsResponse | null>(null);
  const [constructedAllocation, setConstructedAllocation] = useState<ConstructedAllocationResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [constructing, setConstructing] = useState(false);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      // Load portfolio details and holdings in parallel
      const [portfolioData, holdingsData] = await Promise.all([
        apiClient<Portfolio>(`/v1/portfolios/${portfolioId}`),
        apiClient<PortfolioHoldingsResponse>(`/v1/portfolios/${portfolioId}/holdings`),
      ]);
      
      setPortfolio(portfolioData);
      setHoldings(holdingsData);

      // Try to load existing allocation if available
      try {
        const allocationData = await apiClient<ConstructedAllocationResponse>(`/v1/portfolios/${portfolioId}/allocation`);
        setConstructedAllocation(allocationData);
      } catch (err) {
        // Only suppress 404 (allocation doesn't exist yet), log other errors
        if (err instanceof ApiError && err.message.includes('404')) {
          setConstructedAllocation(null);
        } else {
          console.error("Failed to load allocation:", err);
          setConstructedAllocation(null);
        }
      }
    } catch (err) {
      const errorMessage = err instanceof ApiError ? err.message : "Failed to load portfolio data";
      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  }, [portfolioId]);

  const constructAllocation = async () => {
    try {
      setConstructing(true);
      const allocationData = await apiClient<ConstructedAllocationResponse>(
        `/v1/portfolios/${portfolioId}/construct`,
        { method: 'POST' }
      );
      setConstructedAllocation(allocationData);
      toast.success("Portfolio allocation constructed successfully!");
    } catch (err) {
      const errorMessage = err instanceof ApiError ? err.message : "Failed to construct portfolio allocation";
      toast.error(errorMessage);
    } finally {
      setConstructing(false);
    }
  };

  useEffect(() => {
    loadData();
  }, [loadData]);

  if (loading) {
    return <LoadingSkeleton type="card" count={1} />;
  }

  if (error) {
    return (
      <ErrorAlert 
        message={error}
        onRetry={loadData}
        onDismiss={() => setError(null)}
      />
    );
  }

  if (!portfolio || !holdings) {
    return (
      <EmptyState
        icon="portfolio"
        title="Portfolio not found"
        description="This portfolio doesn't exist or you don't have access to it"
      />
    );
  }

  return (
    <>
      {/* Portfolio Header */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <Link
              href="/portfolios"
              className="text-cyan-300 hover:text-cyan-200 transition-colors drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]"
            >
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
              </svg>
            </Link>
            <h2 className="text-3xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 bg-clip-text text-transparent drop-shadow-[0_0_12px_rgba(232,121,249,0.4)]">
              {portfolio.name}
            </h2>
            {portfolio.is_default && (
              <span className="inline-flex items-center gap-1 px-3 py-1 text-xs font-bold text-cyan-400 border-2 border-cyan-500/50 rounded-lg shadow-[0_0_10px_rgba(34,211,238,0.3)]">
                <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                </svg>
                Default
              </span>
            )}
          </div>
          <PortfolioCompositionEditor
            portfolioId={portfolioId}
            portfolioName={portfolio.name}
            onUpdate={loadData}
          />
        </div>
        {portfolio.description && (
          <p className="text-slate-400 text-sm">{portfolio.description}</p>
        )}
        
        {/* Quick Actions */}
        <div className="mt-4 flex gap-3">
          <Link
            href={`/portfolios/${portfolioId}/snapshots`}
            className="inline-flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-violet-600 to-fuchsia-600 text-white font-semibold rounded-lg border-2 border-fuchsia-500 shadow-[0_0_20px_rgba(217,70,239,0.5)] hover:shadow-[0_0_25px_rgba(217,70,239,0.7)] hover:scale-105 transition-all"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
            View Snapshots
          </Link>
          <Link
            href={`/portfolios/${portfolioId}/recommendations`}
            className="inline-flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-cyan-600 to-blue-600 text-white font-semibold rounded-lg border-2 border-cyan-500 shadow-[0_0_20px_rgba(34,211,238,0.5)] hover:shadow-[0_0_25px_rgba(34,211,238,0.7)] hover:scale-105 transition-all"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
            </svg>
            Recommendations
          </Link>
          <Link
            href={`/portfolios/${portfolioId}/settings`}
            className="inline-flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-purple-600 to-violet-600 text-white font-semibold rounded-lg border-2 border-purple-500 shadow-[0_0_20px_rgba(168,85,247,0.5)] hover:shadow-[0_0_25px_rgba(168,85,247,0.7)] hover:scale-105 transition-all"
          >
            <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
            Settings
          </Link>
        </div>
      </div>

      {/* Total Value Hero Section */}
      <div className="mb-8 bg-gradient-to-br from-slate-900/80 via-slate-900/70 to-slate-950/80 backdrop-blur-sm border-2 border-fuchsia-500/50 shadow-[0_0_30px_rgba(217,70,239,0.3)] rounded-2xl p-8">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-slate-400 text-sm mb-2 uppercase tracking-wide">Total Portfolio Value</p>
            <h3 className="text-5xl font-extrabold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-cyan-300 bg-clip-text text-transparent drop-shadow-[0_0_20px_rgba(232,121,249,0.6)]">
              {formatCurrency(holdings.total_value_usd)}
            </h3>
            <p className="text-slate-500 text-xs mt-2">
              Last updated: {new Date(holdings.as_of).toLocaleString()}
            </p>
          </div>
          <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-fuchsia-500 to-purple-600 flex items-center justify-center shadow-[0_0_30px_rgba(217,70,239,0.5)]">
            <svg className="w-10 h-10 text-white drop-shadow-[0_0_10px_rgba(255,255,255,0.6)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
        </div>
      </div>

      {/* Portfolio Construction Section */}
      <div className="mb-8 bg-gradient-to-br from-slate-900/80 via-slate-900/70 to-slate-950/80 backdrop-blur-sm border-2 border-emerald-500/50 shadow-[0_0_30px_rgba(16,185,129,0.3)] rounded-2xl p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-emerald-500 to-teal-600 flex items-center justify-center shadow-[0_0_15px_rgba(16,185,129,0.5)]">
              <svg className="w-5 h-5 text-white drop-shadow-[0_0_6px_rgba(255,255,255,0.5)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 7h6m0 10v-3m-3 3h.01M9 17h.01M9 14h.01M12 14h.01M15 11h.01M12 11h.01M9 11h.01M7 21h10a2 2 0 002-2V5a2 2 0 00-2-2H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
              </svg>
            </div>
            <h3 className="text-xl font-bold text-emerald-300 drop-shadow-[0_0_8px_rgba(16,185,129,0.4)]">
              Portfolio Allocation
            </h3>
          </div>
          <button
            onClick={constructAllocation}
            disabled={constructing}
            className="inline-flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-emerald-600 to-teal-600 text-white font-semibold rounded-lg border-2 border-emerald-500 shadow-[0_0_20px_rgba(16,185,129,0.5)] hover:shadow-[0_0_25px_rgba(16,185,129,0.7)] hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
          >
            {constructing ? (
              <>
                <svg className="animate-spin h-5 w-5" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                Constructing...
              </>
            ) : (
              <>
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                Construct Portfolio
              </>
            )}
          </button>
        </div>

        {constructedAllocation ? (
          <div className="space-y-4">
            <div className="flex items-center justify-between border-b border-slate-700/50 pb-3">
              <div>
                <p className="text-slate-400 text-sm">Total Value (USD)</p>
                <p className="text-2xl font-bold text-emerald-300">
                  {formatCurrency(constructedAllocation.total_value_usd)}
                </p>
              </div>
              <div className="text-right">
                <p className="text-slate-400 text-sm">Constructed At</p>
                <p className="text-sm text-cyan-300">
                  {new Date(constructedAllocation.as_of).toLocaleString()}
                </p>
              </div>
            </div>

            <div className="space-y-2">
              <h4 className="text-sm font-semibold text-slate-300 uppercase tracking-wide">Asset Breakdown</h4>
              <div className="grid grid-cols-1 gap-2">
                {constructedAllocation.holdings.map((holding) => (
                  <div 
                    key={holding.asset}
                    className="flex items-center justify-between bg-slate-900/50 rounded-lg p-3 border border-slate-700/50"
                  >
                    <div className="flex items-center gap-3">
                      <span className="font-bold text-fuchsia-300 drop-shadow-[0_0_6px_rgba(232,121,249,0.4)]">
                        {holding.asset}
                      </span>
                      {holding.unpriced && (
                        <span className="text-xs px-2 py-1 bg-yellow-500/20 text-yellow-300 rounded border border-yellow-500/50">
                          Unpriced
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-6">
                      <div className="text-right">
                        <p className="text-xs text-slate-400">Quantity</p>
                        <p className="text-sm font-semibold text-slate-200">{holding.quantity}</p>
                      </div>
                      {holding.price_usd && (
                        <div className="text-right">
                          <p className="text-xs text-slate-400">Price</p>
                          <p className="text-sm font-semibold text-slate-200">{formatCurrency(holding.price_usd)}</p>
                        </div>
                      )}
                      <div className="text-right">
                        <p className="text-xs text-slate-400">Value</p>
                        <p className="text-sm font-semibold text-emerald-300">{formatCurrency(holding.value_usd)}</p>
                      </div>
                      <div className="text-right min-w-[60px]">
                        <p className="text-xs text-slate-400">Weight</p>
                        <p className="text-sm font-bold text-cyan-300">{formatPercentage(holding.weight)}</p>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        ) : (
          <div className="text-center py-8">
            <p className="text-slate-400 text-sm mb-4">
              No allocation has been constructed yet. Click the button above to construct the portfolio allocation.
            </p>
            <div className="flex items-center justify-center gap-2 text-xs text-slate-500">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              This will aggregate holdings from all linked accounts and compute current allocation
            </div>
          </div>
        )}
      </div>

      {/* Allocation Breakdown - Bar Chart */}
      {holdings.allocation.length === 0 ? (
        <div className="mb-8">
          <EmptyState
            icon="portfolio"
            title="No holdings to display"
            description="This portfolio doesn't have any assets yet"
          />
        </div>
      ) : (
        <>
          <div className="mb-8">
            <AllocationBar allocation={holdings.allocation} maxItems={MAX_ALLOCATION_ITEMS} />
          </div>
          
          {/* Allocation Pie Chart */}
          <div className="mb-8">
            <AllocationPie allocation={holdings.allocation} maxItems={MAX_ALLOCATION_ITEMS} />
          </div>
        </>
      )}

      {/* Holdings Table */}
      {holdings.holdings.length > 0 ? (
        <div className="mb-8">
          <HoldingsTable holdings={holdings.holdings} allocation={holdings.allocation} />
        </div>
      ) : (
        <div className="mb-8">
          <EmptyState
            icon="portfolio"
            title="No holdings found"
            description="This portfolio doesn't have any assets yet"
          />
        </div>
      )}

      {/* Drift Indicators */}
      {portfolio.target_allocation && Object.keys(portfolio.target_allocation).length > 0 && (
        <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-yellow-500/40 shadow-[0_0_25px_rgba(234,179,8,0.3)] rounded-2xl p-6">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-yellow-500 to-orange-600 flex items-center justify-center shadow-[0_0_15px_rgba(234,179,8,0.5)]">
              <svg className="w-5 h-5 text-white drop-shadow-[0_0_6px_rgba(255,255,255,0.5)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
              </svg>
            </div>
            <h3 className="text-xl font-bold text-yellow-300 drop-shadow-[0_0_8px_rgba(234,179,8,0.4)]">
              Portfolio Drift Indicators
            </h3>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {holdings.allocation.map((item) => {
              const targetPercentage = portfolio.target_allocation?.[item.asset];
              if (targetPercentage === undefined) return null;
              
              return (
                <div key={item.asset} className="bg-slate-900/50 rounded-lg p-4 border border-slate-700/50">
                  <div className="flex items-center justify-between mb-2">
                    <span className="font-bold text-fuchsia-300 drop-shadow-[0_0_6px_rgba(232,121,249,0.4)]">
                      {item.asset}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="text-sm">
                      <div className="text-slate-400">Current:</div>
                      <div className="text-cyan-300 font-bold">
                        {formatPercentage(item.percentage)}
                      </div>
                    </div>
                    <DriftBadge
                      currentPercentage={item.percentage}
                      targetPercentage={targetPercentage}
                      asset={item.asset}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </>
  );
}
