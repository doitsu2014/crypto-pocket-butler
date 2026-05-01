"use client";

import { useState } from "react";

export interface AccountHoldingDetail {
  account_id: string;
  account_name: string;
  /** Normalized (human-readable) quantity, e.g. "1.5" for 1.5 ETH */
  quantity: string;
  /** Normalized available quantity */
  available: string;
  /** Normalized frozen quantity */
  frozen: string;
  /** Token decimal places metadata */
  decimals?: number;
  /** Same as quantity – kept for backwards compatibility */
  normalized_quantity?: string;
}

export interface AssetHolding {
  asset: string;
  /** Chain label when asset is chain-specific (e.g. "ethereum", "bsc", "solana") */
  chain?: string;
  /** Normalized (human-readable) total quantity */
  total_quantity: string;
  /** Normalized total available quantity */
  total_available: string;
  /** Normalized total frozen quantity */
  total_frozen: string;
  /** Token decimal places metadata */
  decimals?: number;
  /** Same as total_quantity – kept for backwards compatibility */
  normalized_quantity?: string;
  price_usd: number;
  value_usd: number;
  accounts: AccountHoldingDetail[];
}

export interface AllocationItem {
  asset: string;
  /** Chain label – mirrors AssetHolding.chain */
  chain?: string;
  value_usd: number;
  percentage: number;
}

interface HoldingsTableProps {
  holdings: AssetHolding[];
  allocation: AllocationItem[];
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

/**
 * Format a normalized (human-readable) quantity string for display.
 * Quantities stored in the DB are already normalized decimals (e.g. "1.5" for 1.5 ETH),
 * so this function only applies locale formatting — no further normalization is performed.
 */
function formatQuantity(quantity: string): string {
  try {
    const num = parseFloat(quantity);
    if (isNaN(num)) return quantity;
    
    // For very small numbers, use exponential notation
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

/** Unique key for a holding row, combining asset symbol and optional chain */
function holdingKey(h: AssetHolding): string {
  return h.chain ? `${h.asset}:${h.chain}` : h.asset;
}

export default function HoldingsTable({ holdings, allocation }: HoldingsTableProps) {
  const [sortField, setSortField] = useState<'asset' | 'quantity' | 'value'>('value');
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('desc');
  const [expandedAsset, setExpandedAsset] = useState<string | null>(null);

  function handleSort(field: 'asset' | 'quantity' | 'value') {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('desc');
    }
  }

  const sortedHoldings = holdings.slice().sort((a, b) => {
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

  return (
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
                  Qty (normalized)
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
              <th className="px-6 py-4 text-center">
                <span className="text-sm font-bold text-slate-300">Accounts</span>
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/50">
            {sortedHoldings.map((holding, index) => {
              const key = holdingKey(holding);
              const allocationItem = allocation.find(
                a => a.asset === holding.asset && a.chain === holding.chain
              );
              const isExpanded = expandedAsset === key;
              return (
                <>
                  <tr
                    key={key}
                    className="hover:bg-slate-900/30 transition-colors cursor-pointer"
                    onClick={() => setExpandedAsset(isExpanded ? null : key)}
                  >
                    <td className="px-6 py-4">
                      <div className="flex items-center gap-2">
                        <span className="font-bold text-fuchsia-300 drop-shadow-[0_0_6px_rgba(232,121,249,0.4)]">
                          {holding.asset}
                        </span>
                        {holding.chain && (
                          <span className="inline-flex items-center px-1.5 py-0.5 text-xs rounded bg-blue-900/50 text-blue-300 border border-blue-600/50 capitalize">
                            {holding.chain}
                          </span>
                        )}
                        {index < 3 && (
                          <span className="inline-flex items-center px-2 py-0.5 text-xs font-bold rounded bg-gradient-to-r from-fuchsia-600 to-violet-600 text-white shadow-[0_0_10px_rgba(217,70,239,0.4)]">
                            Top {index + 1}
                          </span>
                        )}
                        {holding.decimals !== undefined && (
                          <span className="inline-flex items-center px-1.5 py-0.5 text-xs rounded bg-slate-700/60 text-slate-400 border border-slate-600/50">
                            {holding.decimals}d
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
                        {allocationItem ? formatPercentage(allocationItem.percentage) : '0.00%'}
                      </span>
                    </td>
                    <td className="px-6 py-4 text-center">
                      <div className="flex items-center justify-center gap-1">
                        <span className="text-xs text-slate-400">{holding.accounts.length}</span>
                        <svg
                          className={`w-4 h-4 text-slate-400 transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                          fill="none" viewBox="0 0 24 24" stroke="currentColor"
                        >
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                        </svg>
                      </div>
                    </td>
                  </tr>
                  {isExpanded && holding.accounts.length > 0 && (
                    <tr key={`${key}-detail`} className="bg-slate-900/40">
                      <td colSpan={6} className="px-6 py-3">
                        <div className="ml-4 border-l-2 border-cyan-500/30 pl-4">
                          <p className="text-xs font-semibold text-slate-400 uppercase tracking-wide mb-2">
                            Per-account breakdown
                          </p>
                          <div className="space-y-1">
                            {holding.accounts.map((acc) => (
                              <div
                                key={acc.account_id}
                                className="flex items-center justify-between text-xs bg-slate-800/40 rounded-lg px-3 py-2"
                              >
                                <span className="text-slate-300 font-medium">{acc.account_name}</span>
                                <div className="flex items-center gap-4">
                                  <div className="text-right">
                                    <p className="text-slate-500">Qty</p>
                                    <p className="text-slate-200 font-mono">
                                      {formatQuantity(acc.normalized_quantity ?? acc.quantity)}
                                    </p>
                                  </div>
                                  {acc.frozen && acc.frozen !== '0' && (
                                    <div className="text-right">
                                      <p className="text-slate-500">Frozen</p>
                                      <p className="text-orange-300 font-mono">
                                        {formatQuantity(acc.frozen)}
                                      </p>
                                    </div>
                                  )}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      </td>
                    </tr>
                  )}
                </>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
