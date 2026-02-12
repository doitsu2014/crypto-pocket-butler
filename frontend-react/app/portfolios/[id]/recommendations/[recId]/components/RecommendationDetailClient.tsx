"use client";

import { useState, useEffect, useCallback } from "react";
import { apiClient } from "@/lib/api-client";
import Link from "next/link";

interface ProposedOrder {
  action: string;
  asset: string;
  quantity: string;
  estimated_price: string;
  estimated_value_usd: string;
}

interface Recommendation {
  id: string;
  portfolio_id: string;
  status: string;
  recommendation_type: string;
  rationale: string;
  proposed_orders: ProposedOrder[];
  expected_impact?: string;
  metadata?: {
    risk_score?: number;
    confidence?: number;
    drift_percentage?: number;
    price_appreciation_30d?: number;
    resistance_level?: string;
    [key: string]: any;
  };
  created_at: string;
  updated_at: string;
  executed_at?: string;
}

function formatCurrency(value: string | number): string {
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  if (isNaN(numValue)) return '$0.00';
  
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(numValue);
}

function formatDateTime(dateString: string): string {
  return new Date(dateString).toLocaleString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function getStatusColor(status: string): string {
  switch (status.toLowerCase()) {
    case 'pending':
      return 'bg-yellow-500/20 border-yellow-500/50 text-yellow-300';
    case 'approved':
      return 'bg-green-500/20 border-green-500/50 text-green-300';
    case 'rejected':
      return 'bg-red-500/20 border-red-500/50 text-red-300';
    case 'executed':
      return 'bg-blue-500/20 border-blue-500/50 text-blue-300';
    default:
      return 'bg-slate-500/20 border-slate-500/50 text-slate-300';
  }
}

function getTypeColor(type: string): string {
  switch (type.toLowerCase()) {
    case 'rebalance':
      return 'bg-violet-500/20 border-violet-500/50 text-violet-300';
    case 'take_profit':
      return 'bg-cyan-500/20 border-cyan-500/50 text-cyan-300';
    case 'stop_loss':
      return 'bg-fuchsia-500/20 border-fuchsia-500/50 text-fuchsia-300';
    default:
      return 'bg-slate-500/20 border-slate-500/50 text-slate-300';
  }
}

export default function RecommendationDetailClient({ 
  portfolioId, 
  recommendationId 
}: { 
  portfolioId: string; 
  recommendationId: string;
}) {
  const [recommendation, setRecommendation] = useState<Recommendation | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchRecommendation = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      const data = await apiClient<Recommendation>(
        `/v1/portfolios/${portfolioId}/recommendations/${recommendationId}`
      );
      
      setRecommendation(data);
    } catch (err) {
      console.error('Error fetching recommendation:', err);
      setError(err instanceof Error ? err.message : 'Failed to load recommendation');
    } finally {
      setLoading(false);
    }
  }, [portfolioId, recommendationId]);

  useEffect(() => {
    fetchRecommendation();
  }, [fetchRecommendation]);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="animate-pulse text-cyan-300 text-lg drop-shadow-[0_0_10px_rgba(103,232,249,0.6)]">
          Loading recommendation details...
        </div>
      </div>
    );
  }

  if (error || !recommendation) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-red-400 text-lg drop-shadow-[0_0_10px_rgba(239,68,68,0.6)]">
          Error: {error || 'Recommendation not found'}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Back Link */}
      <Link
        href={`/portfolios/${portfolioId}/recommendations`}
        className="inline-flex items-center text-cyan-300 hover:text-cyan-200 transition-colors drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]"
      >
        <svg className="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        Back to Recommendations
      </Link>

      {/* Header */}
      <div className="bg-slate-950/70 backdrop-blur-sm rounded-xl border-2 border-fuchsia-500/30 p-6 shadow-[0_0_20px_rgba(217,70,239,0.2)]">
        <div className="flex justify-between items-start mb-4">
          <div>
            <h2 className="text-3xl font-bold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-cyan-300 bg-clip-text text-transparent drop-shadow-[0_0_10px_rgba(232,121,249,0.5)] mb-3">
              Recommendation Details
            </h2>
            <div className="flex gap-3">
              <span className={`px-4 py-2 rounded-lg text-sm font-medium border ${getStatusColor(recommendation.status)}`}>
                {recommendation.status.toUpperCase()}
              </span>
              <span className={`px-4 py-2 rounded-lg text-sm font-medium border ${getTypeColor(recommendation.recommendation_type)}`}>
                {recommendation.recommendation_type.replace('_', ' ').toUpperCase()}
              </span>
            </div>
          </div>
          <div className="text-right text-sm text-slate-400">
            <div className="mb-1">Created: {formatDateTime(recommendation.created_at)}</div>
            <div>Updated: {formatDateTime(recommendation.updated_at)}</div>
            {recommendation.executed_at && (
              <div className="text-blue-400 mt-2">Executed: {formatDateTime(recommendation.executed_at)}</div>
            )}
          </div>
        </div>
      </div>

      {/* Rationale */}
      <div className="bg-slate-950/70 backdrop-blur-sm rounded-xl border-2 border-violet-500/30 p-6 shadow-[0_0_20px_rgba(139,92,246,0.2)]">
        <h3 className="text-xl font-bold text-violet-300 mb-3 drop-shadow-[0_0_8px_rgba(167,139,250,0.5)]">
          Rationale
        </h3>
        <p className="text-slate-300 leading-relaxed text-lg">{recommendation.rationale}</p>
      </div>

      {/* Metadata */}
      {recommendation.metadata && Object.keys(recommendation.metadata).length > 0 && (
        <div className="bg-slate-950/70 backdrop-blur-sm rounded-xl border-2 border-cyan-500/30 p-6 shadow-[0_0_20px_rgba(34,211,238,0.2)]">
          <h3 className="text-xl font-bold text-cyan-300 mb-4 drop-shadow-[0_0_8px_rgba(103,232,249,0.5)]">
            Analysis Metrics
          </h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {recommendation.metadata.confidence !== undefined && (
              <div className="bg-slate-900/50 rounded-lg p-4">
                <div className="text-sm text-slate-400 mb-1">Confidence</div>
                <div className="text-2xl font-bold text-cyan-300">
                  {(recommendation.metadata.confidence * 100).toFixed(0)}%
                </div>
              </div>
            )}
            {recommendation.metadata.risk_score !== undefined && (
              <div className="bg-slate-900/50 rounded-lg p-4">
                <div className="text-sm text-slate-400 mb-1">Risk Score</div>
                <div className="text-2xl font-bold text-violet-300">
                  {(recommendation.metadata.risk_score * 10).toFixed(1)}/10
                </div>
              </div>
            )}
            {recommendation.metadata.drift_percentage !== undefined && (
              <div className="bg-slate-900/50 rounded-lg p-4">
                <div className="text-sm text-slate-400 mb-1">Allocation Drift</div>
                <div className="text-2xl font-bold text-yellow-300">
                  {recommendation.metadata.drift_percentage.toFixed(1)}%
                </div>
              </div>
            )}
            {recommendation.metadata.price_appreciation_30d !== undefined && (
              <div className="bg-slate-900/50 rounded-lg p-4">
                <div className="text-sm text-slate-400 mb-1">30-Day Appreciation</div>
                <div className="text-2xl font-bold text-green-300">
                  +{(recommendation.metadata.price_appreciation_30d * 100).toFixed(0)}%
                </div>
              </div>
            )}
            {recommendation.metadata.resistance_level && (
              <div className="bg-slate-900/50 rounded-lg p-4">
                <div className="text-sm text-slate-400 mb-1">Resistance Level</div>
                <div className="text-2xl font-bold text-fuchsia-300">
                  {recommendation.metadata.resistance_level}
                </div>
              </div>
            )}
            {recommendation.expected_impact && (
              <div className="bg-slate-900/50 rounded-lg p-4">
                <div className="text-sm text-slate-400 mb-1">Expected Impact</div>
                <div className="text-2xl font-bold text-green-300">
                  {formatCurrency(recommendation.expected_impact)}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Proposed Orders */}
      <div className="bg-slate-950/70 backdrop-blur-sm rounded-xl border-2 border-fuchsia-500/30 p-6 shadow-[0_0_20px_rgba(217,70,239,0.2)]">
        <h3 className="text-xl font-bold text-fuchsia-300 mb-4 drop-shadow-[0_0_8px_rgba(232,121,249,0.5)]">
          Proposed Orders ({recommendation.proposed_orders.length})
        </h3>
        <div className="space-y-4">
          {recommendation.proposed_orders.map((order, idx) => (
            <div
              key={idx}
              className="bg-slate-900/50 rounded-lg p-5 border border-slate-700/50 hover:border-fuchsia-500/30 transition-all"
            >
              <div className="flex justify-between items-start mb-3">
                <div className="flex items-center gap-3">
                  <span className={`px-3 py-1 rounded-lg text-sm font-bold ${
                    order.action === 'buy' 
                      ? 'bg-green-500/20 border border-green-500/50 text-green-300' 
                      : 'bg-red-500/20 border border-red-500/50 text-red-300'
                  }`}>
                    {order.action.toUpperCase()}
                  </span>
                  <span className="text-xl font-bold text-white">{order.asset}</span>
                </div>
                <div className="text-right">
                  <div className="text-sm text-slate-400">Estimated Value</div>
                  <div className="text-lg font-bold text-cyan-300">{formatCurrency(order.estimated_value_usd)}</div>
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-slate-400">Quantity: </span>
                  <span className="text-white font-medium">{order.quantity}</span>
                </div>
                <div>
                  <span className="text-slate-400">Estimated Price: </span>
                  <span className="text-white font-medium">{formatCurrency(order.estimated_price)}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Action Buttons (Placeholder - no execution in this version) */}
      <div className="bg-slate-950/70 backdrop-blur-sm rounded-xl border-2 border-yellow-500/30 p-6 shadow-[0_0_20px_rgba(234,179,8,0.2)]">
        <div className="flex items-start gap-4">
          <svg className="w-6 h-6 text-yellow-400 mt-0.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <div className="flex-1">
            <h4 className="text-lg font-bold text-yellow-300 mb-2">Suggest-Only Mode</h4>
            <p className="text-slate-300 mb-4">
              This is a suggest-only interface. No orders will be executed automatically. Review the recommendations and execute trades manually through your exchange or wallet if you choose to proceed.
            </p>
            <div className="flex gap-3">
              <button
                disabled
                className="px-6 py-2 bg-slate-700/50 text-slate-400 font-medium rounded-lg border border-slate-600 cursor-not-allowed"
              >
                Approve (Coming Soon)
              </button>
              <button
                disabled
                className="px-6 py-2 bg-slate-700/50 text-slate-400 font-medium rounded-lg border border-slate-600 cursor-not-allowed"
              >
                Reject (Coming Soon)
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
