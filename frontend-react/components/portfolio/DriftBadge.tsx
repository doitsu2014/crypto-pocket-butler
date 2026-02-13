"use client";

interface DriftBadgeProps {
  currentPercentage: number;
  targetPercentage: number;
  asset: string;
}

function formatPercentage(value: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'percent',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value / 100);
}

export default function DriftBadge({ currentPercentage, targetPercentage, asset }: DriftBadgeProps) {
  // Calculate drift as the difference between current and target
  const drift = currentPercentage - targetPercentage;
  const driftAbs = Math.abs(drift);
  
  // Determine severity: green (<5%), yellow (5-10%), red (>10%)
  let severity: 'success' | 'warning' | 'danger';
  let bgClass: string;
  let textClass: string;
  let shadowClass: string;
  let icon: string;
  
  if (driftAbs < 5) {
    severity = 'success';
    bgClass = 'bg-gradient-to-r from-green-600 to-emerald-600';
    textClass = 'text-green-300';
    shadowClass = 'shadow-[0_0_10px_rgba(34,197,94,0.4)]';
    icon = '✓';
  } else if (driftAbs < 10) {
    severity = 'warning';
    bgClass = 'bg-gradient-to-r from-yellow-600 to-amber-600';
    textClass = 'text-yellow-300';
    shadowClass = 'shadow-[0_0_10px_rgba(234,179,8,0.4)]';
    icon = '⚠';
  } else {
    severity = 'danger';
    bgClass = 'bg-gradient-to-r from-red-600 to-rose-600';
    textClass = 'text-red-300';
    shadowClass = 'shadow-[0_0_10px_rgba(239,68,68,0.4)]';
    icon = '!';
  }
  
  const sign = drift > 0 ? '+' : '';
  
  return (
    <div className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-lg ${bgClass} ${shadowClass} border-2 border-white/20`}>
      <span className="text-white font-bold text-xs">{icon}</span>
      <div className="flex flex-col">
        <span className="text-white font-bold text-xs leading-tight">
          {sign}{formatPercentage(drift)}
        </span>
        <span className="text-white/70 text-[10px] leading-tight">
          Target: {formatPercentage(targetPercentage)}
        </span>
      </div>
    </div>
  );
}
