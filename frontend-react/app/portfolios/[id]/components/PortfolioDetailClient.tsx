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

function formatQuantity(quantity: string): string {
  try {
    const num = parseFloat(quantity);
    if (isNaN(num)) return quantity;
    
    // For very small numbers, use more decimal places
    if (num < 0.000001 && num > 0) {
      return num.toExponential(4);
    }
    
    // For small numbers (less than 0.01), show up to 8 decimals
    if (num < 0.01) {
      return num.toFixed(8).replace(/\.?0+$/, '');
    }
    
    // For regular numbers, show up to 4 decimals
    return num.toLocaleString('en-US', {
      minimumFractionDigits: 0,
      maximumFractionDigits: 4,
    });
  } catch {
    return quantity;
  }
}

export default function PortfolioDetailClient({ portfolioId }: { portfolioId: string }) {
  const [portfolio, setPortfolio] = useState<Portfolio | null>(null);
  const [holdings, setHoldings] = useState<PortfolioHoldingsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sortField, setSortField] = useState<'asset' | 'quantity' | 'value'>('value');
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('desc');

  useEffect(() => {
    loadData();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [portfolioId]);

  async function loadData() {
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
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load portfolio data");
    } finally {
      setLoading(false);
    }
  }

  function handleSort(field: 'asset' | 'quantity' | 'value') {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('desc');
    }
  }

  const sortedHoldings = holdings?.holdings.slice().sort((a, b) => {
    let compareValue = 0;
    
    switch (sortField) {
      case 'asset':
        compareValue = a.asset.localeCompare(b.asset);
        break;
      case 'quantity':
        compareValue = parseFloat(a.total_quantity) - parseFloat(b.total_quantity);
        break;
      case 'value':
        compareValue = a.value_usd - b.value_usd;
        break;
    }
    
    return sortDirection === 'asc' ? compareValue : -compareValue;
  });

  if (loading) {
    return (
      <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-2xl p-8">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-violet-900/50 rounded w-1/3 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
          <div className="h-20 bg-violet-900/50 rounded w-full shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
          <div className="h-40 bg-violet-900/50 rounded w-full shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-950/30 border-2 border-red-500/50 rounded-xl p-6 shadow-[0_0_20px_rgba(239,68,68,0.25)]">
        <h3 className="text-lg font-bold text-red-300 mb-2">Error</h3>
        <p className="text-red-300 text-sm">{error}</p>
        <button
          onClick={loadData}
          className="mt-4 inline-flex items-center px-4 py-2 border-2 border-red-500 text-sm font-bold rounded-lg text-white bg-gradient-to-r from-red-600 to-red-700 hover:from-red-500 hover:to-red-600 shadow-[0_0_15px_rgba(239,68,68,0.4)] transition-all"
        >
          Retry
        </button>
      </div>
    );
  }

  if (!portfolio || !holdings) {
    return (
      <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-cyan-500/40 shadow-[0_0_25px_rgba(34,211,238,0.3)] rounded-2xl p-8 text-center">
        <p className="text-slate-300 text-lg">Portfolio not found</p>
      </div>
    );
  }

  return (
    <>
      {/* Portfolio Header */}
      <div className="mb-6">
        <div className="flex items-center gap-3 mb-4">
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
        {portfolio.description && (
          <p className="text-slate-400 text-sm">{portfolio.description}</p>
        )}
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

      {/* Allocation Breakdown */}
      <div className="mb-8 bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-2xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-violet-500 to-purple-600 flex items-center justify-center shadow-[0_0_15px_rgba(139,92,246,0.5)]">
            <svg className="w-5 h-5 text-white drop-shadow-[0_0_6px_rgba(255,255,255,0.5)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 3.055A9.001 9.001 0 1020.945 13H11V3.055z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.488 9H15V3.512A9.025 9.025 0 0120.488 9z" />
            </svg>
          </div>
          <h3 className="text-xl font-bold text-violet-300 drop-shadow-[0_0_8px_rgba(167,139,250,0.4)]">
            Asset Allocation
          </h3>
        </div>
        
        {holdings.allocation.length === 0 ? (
          <p className="text-slate-400 text-center py-4">No holdings to display</p>
        ) : (
          <div className="space-y-3">
            {holdings.allocation.slice(0, 10).map((item) => (
              <div key={item.asset} className="flex items-center gap-4">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-sm font-bold text-fuchsia-300 truncate drop-shadow-[0_0_6px_rgba(232,121,249,0.4)]">
                      {item.asset}
                    </span>
                    <div className="flex items-center gap-3">
                      <span className="text-xs text-slate-400 tabular-nums">
                        {formatCurrency(item.value_usd)}
                      </span>
                      <span className="text-sm font-bold text-cyan-300 tabular-nums min-w-[4rem] text-right drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]">
                        {formatPercentage(item.percentage)}
                      </span>
                    </div>
                  </div>
                  <div className="w-full bg-slate-800/50 rounded-full h-2 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-fuchsia-500 via-violet-500 to-purple-500 shadow-[0_0_10px_rgba(217,70,239,0.5)] transition-all duration-500"
                      style={{ width: `${Math.min(item.percentage, 100)}%` }}
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Holdings Table */}
      <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-cyan-500/40 shadow-[0_0_25px_rgba(34,211,238,0.3)] rounded-2xl overflow-hidden">
        <div className="p-6 border-b-2 border-cyan-500/30">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center shadow-[0_0_15px_rgba(34,211,238,0.5)]">
              <svg className="w-5 h-5 text-white drop-shadow-[0_0_6px_rgba(255,255,255,0.5)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
            </div>
            <h3 className="text-xl font-bold text-cyan-300 drop-shadow-[0_0_8px_rgba(34,211,238,0.4)]">
              Top Holdings
            </h3>
          </div>
        </div>
        
        {sortedHoldings && sortedHoldings.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-slate-900/50">
                <tr>
                  <th className="px-6 py-4 text-left">
                    <button
                      onClick={() => handleSort('asset')}
                      className="flex items-center gap-2 text-sm font-bold text-slate-300 hover:text-fuchsia-300 transition-colors"
                    >
                      Asset
                      {sortField === 'asset' && (
                        <svg className={`w-4 h-4 transition-transform ${sortDirection === 'desc' ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                        </svg>
                      )}
                    </button>
                  </th>
                  <th className="px-6 py-4 text-right">
                    <button
                      onClick={() => handleSort('quantity')}
                      className="flex items-center gap-2 justify-end w-full text-sm font-bold text-slate-300 hover:text-fuchsia-300 transition-colors"
                    >
                      Quantity
                      {sortField === 'quantity' && (
                        <svg className={`w-4 h-4 transition-transform ${sortDirection === 'desc' ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                        </svg>
                      )}
                    </button>
                  </th>
                  <th className="px-6 py-4 text-right">
                    <span className="text-sm font-bold text-slate-300">Price (USD)</span>
                  </th>
                  <th className="px-6 py-4 text-right">
                    <button
                      onClick={() => handleSort('value')}
                      className="flex items-center gap-2 justify-end w-full text-sm font-bold text-slate-300 hover:text-fuchsia-300 transition-colors"
                    >
                      Value (USD)
                      {sortField === 'value' && (
                        <svg className={`w-4 h-4 transition-transform ${sortDirection === 'desc' ? 'rotate-180' : ''}`} fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
                        </svg>
                      )}
                    </button>
                  </th>
                  <th className="px-6 py-4 text-right">
                    <span className="text-sm font-bold text-slate-300">Allocation</span>
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/50">
                {sortedHoldings.map((holding, index) => {
                  const allocation = holdings.allocation.find(a => a.asset === holding.asset);
                  return (
                    <tr
                      key={holding.asset}
                      className="hover:bg-slate-900/30 transition-colors"
                    >
                      <td className="px-6 py-4">
                        <div className="flex items-center gap-2">
                          <span className="font-bold text-fuchsia-300 drop-shadow-[0_0_6px_rgba(232,121,249,0.4)]">
                            {holding.asset}
                          </span>
                          {index < 3 && (
                            <span className="inline-flex items-center px-2 py-0.5 text-xs font-bold rounded bg-gradient-to-r from-fuchsia-600 to-violet-600 text-white shadow-[0_0_10px_rgba(217,70,239,0.4)]">
                              Top {index + 1}
                            </span>
                          )}
                        </div>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <span className="text-sm text-slate-300 font-mono tabular-nums">
                          {formatQuantity(holding.total_quantity)}
                        </span>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <span className="text-sm text-slate-300 font-mono tabular-nums">
                          {formatCurrency(holding.price_usd)}
                        </span>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <span className="text-sm font-bold text-cyan-300 font-mono tabular-nums drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]">
                          {formatCurrency(holding.value_usd)}
                        </span>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <span className="text-sm font-bold text-violet-300 font-mono tabular-nums">
                          {allocation ? formatPercentage(allocation.percentage) : '0.00%'}
                        </span>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="p-8 text-center">
            <svg className="w-16 h-16 mx-auto mb-4 text-cyan-400 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
            </svg>
            <p className="text-slate-300 text-lg mb-2">No holdings found</p>
            <p className="text-slate-400 text-sm">This portfolio doesn&apos;t have any assets yet</p>
          </div>
        )}
      </div>

      {/* Drift Indicators Placeholder */}
      <div className="mt-8 bg-slate-950/70 backdrop-blur-sm border-2 border-yellow-500/40 shadow-[0_0_25px_rgba(234,179,8,0.3)] rounded-2xl p-6">
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
        <div className="text-center py-8">
          <p className="text-slate-400 text-sm mb-2">
            Drift indicators are coming soon!
          </p>
          <p className="text-slate-500 text-xs">
            This feature will help you track how your portfolio allocation drifts from your target allocation over time.
          </p>
        </div>
      </div>
    </>
  );
}
