"use client";

import { useState, useMemo } from "react";
import { ApiError } from "@/lib/api-client";
import { useToast } from "@/contexts/ToastContext";
import Link from "next/link";
import { LoadingSpinner } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import ErrorAlert from "@/components/ErrorAlert";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { useSnapshots } from "@/hooks";
import type { Snapshot } from "@/types/api";

interface ChartDataPoint {
  date: string;
  value: number;
  formattedValue: string;
}

function formatCurrency(value: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function formatDateTime(dateStr: string): string {
  const date = new Date(dateStr);
  return date.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export default function SnapshotsClient({
  portfolioId,
}: {
  portfolioId: string;
}) {
  const toast = useToast();
  const [dateRange, setDateRange] = useState<"7" | "30" | "90" | "all">("30");
  const [selectedType, setSelectedType] = useState<string>("all");

  // Calculate query params
  const queryParams = useMemo(() => {
    const params: { snapshot_type?: string; start_date?: string; end_date?: string } = {};
    
    if (dateRange !== "all") {
      const endDate = new Date();
      const startDate = new Date();
      startDate.setDate(endDate.getDate() - parseInt(dateRange));
      params.start_date = startDate.toISOString().split("T")[0];
      params.end_date = endDate.toISOString().split("T")[0];
    }
    
    if (selectedType !== "all") {
      params.snapshot_type = selectedType;
    }
    
    return params;
  }, [dateRange, selectedType]);

  // Use TanStack Query hook
  const { data: response, isLoading: loading, error: queryError, refetch } = useSnapshots(portfolioId, queryParams);

  const snapshots = response?.snapshots || [];
  
  // Convert query error to string for display
  const error = queryError instanceof ApiError ? queryError.message : 
                queryError ? "Failed to fetch snapshots" : null;

  // Prepare chart data
  const chartData: ChartDataPoint[] = useMemo(() => snapshots
    .map((snapshot) => ({
      date: snapshot.snapshot_date,
      value: parseFloat(snapshot.total_value_usd),
      formattedValue: formatCurrency(parseFloat(snapshot.total_value_usd)),
    }))
    .sort((a, b) => new Date(a.date).getTime() - new Date(b.date).getTime()), [snapshots]);

  return (
    <div className="space-y-8">
      {/* Header with filters */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
        <div>
          <h2 className="text-3xl font-bold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-cyan-300 bg-clip-text text-transparent drop-shadow-[0_0_10px_rgba(232,121,249,0.5)]">
            Portfolio Snapshots
          </h2>
          <p className="text-cyan-300/70 mt-2">
            Historical portfolio value over time
          </p>
        </div>

        <div className="flex gap-3 flex-wrap">
          {/* Date Range Selector */}
          <select
            value={dateRange}
            onChange={(e) =>
              setDateRange(e.target.value as "7" | "30" | "90" | "all")
            }
            className="px-4 py-2 bg-slate-900/80 border-2 border-violet-500/50 rounded-lg text-cyan-300 focus:outline-none focus:border-fuchsia-500 transition-colors shadow-[0_0_15px_rgba(139,92,246,0.3)]"
          >
            <option value="7">Last 7 days</option>
            <option value="30">Last 30 days</option>
            <option value="90">Last 90 days</option>
            <option value="all">All time</option>
          </select>

          {/* Type Filter */}
          <select
            value={selectedType}
            onChange={(e) => setSelectedType(e.target.value)}
            className="px-4 py-2 bg-slate-900/80 border-2 border-violet-500/50 rounded-lg text-cyan-300 focus:outline-none focus:border-fuchsia-500 transition-colors shadow-[0_0_15px_rgba(139,92,246,0.3)]"
          >
            <option value="all">All types</option>
            <option value="eod">EOD</option>
            <option value="manual">Manual</option>
            <option value="hourly">Hourly</option>
          </select>

          {/* Refresh Button */}
          <button
            disabled={loading}
            onClick={() => refetch()}
            className="px-4 py-2 bg-gradient-to-r from-violet-600 to-fuchsia-600 text-white font-semibold rounded-lg border-2 border-fuchsia-500 shadow-[0_0_20px_rgba(217,70,239,0.5)] hover:shadow-[0_0_25px_rgba(217,70,239,0.7)] hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loading ? "Refreshing..." : "Refresh"}
          </button>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <ErrorAlert 
          message={error}
          onRetry={() => refetch()}
        />
      )}

      {/* Loading State */}
      {loading && !error && (
        <LoadingSpinner size="lg" message="Loading snapshots..." />
      )}

      {/* Content */}
      {!loading && !error && (
        <>
          {/* Chart Section */}
          {chartData.length > 0 ? (
            <div className="bg-slate-900/60 backdrop-blur-md rounded-lg p-6 border-2 border-violet-500/30 shadow-[0_0_20px_rgba(139,92,246,0.2)]">
              <h3 className="text-xl font-bold text-fuchsia-300 mb-4 drop-shadow-[0_0_8px_rgba(217,70,239,0.4)]">
                Portfolio Value Over Time
              </h3>
              <div className="h-[400px] w-full">
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={chartData}>
                    <CartesianGrid
                      strokeDasharray="3 3"
                      stroke="rgba(139, 92, 246, 0.2)"
                    />
                    <XAxis
                      dataKey="date"
                      stroke="#67e8f9"
                      tick={{ fill: "#67e8f9" }}
                      tickFormatter={formatDate}
                    />
                    <YAxis
                      stroke="#67e8f9"
                      tick={{ fill: "#67e8f9" }}
                      tickFormatter={(value) =>
                        `$${(value / 1000).toFixed(0)}k`
                      }
                    />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: "rgba(15, 23, 42, 0.95)",
                        border: "2px solid rgba(217, 70, 239, 0.5)",
                        borderRadius: "8px",
                        boxShadow: "0 0 20px rgba(217, 70, 239, 0.3)",
                      }}
                      labelStyle={{ color: "#67e8f9" }}
                      itemStyle={{ color: "#d946ef" }}
                      formatter={(value: number | undefined) => {
                        if (value === undefined) return null;
                        return [formatCurrency(value), "Value"];
                      }}
                      labelFormatter={(label) => `Date: ${formatDate(label)}`}
                    />
                    <Line
                      type="monotone"
                      dataKey="value"
                      stroke="#d946ef"
                      strokeWidth={3}
                      dot={{ fill: "#d946ef", strokeWidth: 2, r: 4 }}
                      activeDot={{ r: 6, fill: "#d946ef" }}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </div>
          ) : (
            <EmptyState
              icon="snapshot"
              title="No snapshots found"
              description="No snapshots found for the selected date range. Snapshots are created automatically by the EOD job or can be created manually."
            />
          )}

          {/* Snapshots Table */}
          {snapshots.length > 0 && (
            <div className="bg-slate-900/60 backdrop-blur-md rounded-lg border-2 border-violet-500/30 overflow-hidden shadow-[0_0_20px_rgba(139,92,246,0.2)]">
              <div className="px-6 py-4 border-b-2 border-violet-500/30">
                <h3 className="text-xl font-bold text-fuchsia-300 drop-shadow-[0_0_8px_rgba(217,70,239,0.4)]">
                  Snapshot History ({snapshots.length})
                </h3>
              </div>
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead className="bg-slate-800/50">
                    <tr>
                      <th className="px-6 py-3 text-left text-xs font-semibold text-cyan-300 uppercase tracking-wider">
                        Date
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-semibold text-cyan-300 uppercase tracking-wider">
                        Type
                      </th>
                      <th className="px-6 py-3 text-right text-xs font-semibold text-cyan-300 uppercase tracking-wider">
                        Total Value
                      </th>
                      <th className="px-6 py-3 text-left text-xs font-semibold text-cyan-300 uppercase tracking-wider">
                        Created At
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-violet-500/20">
                    {snapshots.map((snapshot) => (
                      <tr
                        key={snapshot.id}
                        className="hover:bg-slate-800/30 transition-colors"
                      >
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-cyan-300">
                          {formatDate(snapshot.snapshot_date)}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <span
                            className={`inline-flex px-3 py-1 text-xs font-semibold rounded-full border ${
                              snapshot.snapshot_type === "eod"
                                ? "bg-violet-900/30 text-violet-300 border-violet-500/50"
                                : snapshot.snapshot_type === "manual"
                                ? "bg-fuchsia-900/30 text-fuchsia-300 border-fuchsia-500/50"
                                : "bg-cyan-900/30 text-cyan-300 border-cyan-500/50"
                            }`}
                          >
                            {snapshot.snapshot_type.toUpperCase()}
                          </span>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-right font-semibold text-fuchsia-300">
                          {formatCurrency(parseFloat(snapshot.total_value_usd))}
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-cyan-300/70">
                          {formatDateTime(snapshot.created_at)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
