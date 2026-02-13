"use client";

import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, Legend } from "recharts";

export interface AllocationItem {
  asset: string;
  value_usd: number;
  percentage: number;
}

interface AllocationPieProps {
  allocation: AllocationItem[];
  maxItems?: number;
}

// Cyberpunk color palette
const COLORS = [
  '#e879f9', // fuchsia-400
  '#a78bfa', // violet-400
  '#c084fc', // purple-400
  '#22d3ee', // cyan-400
  '#60a5fa', // blue-400
  '#f472b6', // pink-400
  '#a855f7', // purple-500
  '#ec4899', // pink-500
  '#8b5cf6', // violet-500
  '#06b6d4', // cyan-500
];

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

export default function AllocationPie({ allocation, maxItems = 10 }: AllocationPieProps) {
  const displayData = allocation.slice(0, maxItems).map((item) => ({
    name: item.asset,
    value: item.percentage,
    valueUsd: item.value_usd,
  }));

  const CustomTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      const data = payload[0].payload;
      return (
        <div className="bg-slate-900/95 backdrop-blur-sm border-2 border-fuchsia-500/50 rounded-lg p-3 shadow-[0_0_20px_rgba(217,70,239,0.5)]">
          <p className="text-fuchsia-300 font-bold mb-1">{data.name}</p>
          <p className="text-cyan-300 text-sm">{formatCurrency(data.valueUsd)}</p>
          <p className="text-violet-300 text-sm font-bold">{formatPercentage(data.value)}</p>
        </div>
      );
    }
    return null;
  };

  const CustomLegend = ({ payload }: any) => {
    return (
      <div className="flex flex-wrap gap-3 justify-center mt-4">
        {payload.map((entry: any, index: number) => (
          <div key={`legend-${index}`} className="flex items-center gap-2">
            <div
              className="w-3 h-3 rounded-full shadow-[0_0_8px_rgba(232,121,249,0.5)]"
              style={{ backgroundColor: entry.color }}
            />
            <span className="text-xs text-slate-300 font-medium">{entry.value}</span>
          </div>
        ))}
      </div>
    );
  };

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
          Allocation Distribution
        </h3>
      </div>
      
      <ResponsiveContainer width="100%" height={300}>
        <PieChart>
          <Pie
            data={displayData}
            cx="50%"
            cy="50%"
            labelLine={false}
            outerRadius={100}
            fill="#8884d8"
            dataKey="value"
            label={({ name, value }) => `${name} (${formatPercentage(value)})`}
          >
            {displayData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip content={<CustomTooltip />} />
          <Legend content={<CustomLegend />} />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
}
