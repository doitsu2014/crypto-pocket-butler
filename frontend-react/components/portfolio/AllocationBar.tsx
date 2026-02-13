"use client";

export interface AllocationItem {
  asset: string;
  value_usd: number;
  percentage: number;
}

interface AllocationBarProps {
  allocation: AllocationItem[];
  maxItems?: number;
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

export default function AllocationBar({ allocation, maxItems = 10 }: AllocationBarProps) {
  return (
    <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-2xl p-6">
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
      
      <div className="space-y-3">
        {allocation.slice(0, maxItems).map((item) => (
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
    </div>
  );
}
