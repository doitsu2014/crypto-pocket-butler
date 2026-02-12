"use client";

import { useState, useEffect, useCallback } from "react";
import { apiClient } from "@/lib/api-client";
import Link from "next/link";

const MAX_ORDERS_PREVIEW = 4; // Maximum number of orders to show in list view

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
    [key: string]: any;
  };
  created_at: string;
  updated_at: string;
  executed_at?: string;
}

interface RecommendationsListResponse {
  portfolio_id: string;
  recommendations: Recommendation[];
  total_count: number;
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
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
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

export default function RecommendationsClient({ portfolioId }: { portfolioId: string }) {
  const [recommendations, setRecommendations] = useState<Recommendation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);

  const fetchRecommendations = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      const data = await apiClient<RecommendationsListResponse>(
        `/v1/portfolios/${portfolioId}/recommendations`
      );
      
      setRecommendations(data.recommendations || []);
    } catch (err) {
      console.error('Error fetching recommendations:', err);
      setError(err instanceof Error ? err.message : 'Failed to load recommendations');
    } finally {
      setLoading(false);
    }
  }, [portfolioId]);

  const generateMockRecommendations = async () => {
    try {
      setGenerating(true);
      setError(null);
      
      await apiClient<RecommendationsListResponse>(
        `/v1/portfolios/${portfolioId}/recommendations/generate`,
        { method: 'POST' }
      );
      
      // Refetch after generation
      await fetchRecommendations();
    } catch (err) {
      console.error('Error generating recommendations:', err);
      setError(err instanceof Error ? err.message : 'Failed to generate recommendations');
    } finally {
      setGenerating(false);
    }
  };

  useEffect(() => {
    fetchRecommendations();
  }, [fetchRecommendations]);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="animate-pulse text-cyan-300 text-lg drop-shadow-[0_0_10px_rgba(103,232,249,0.6)]">
          Loading recommendations...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-red-400 text-lg drop-shadow-[0_0_10px_rgba(239,68,68,0.6)]">
          Error: {error}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-3xl font-bold bg-gradient-to-r from-fuchsia-300 via-violet-300 to-cyan-300 bg-clip-text text-transparent drop-shadow-[0_0_10px_rgba(232,121,249,0.5)]">
            Portfolio Recommendations
          </h2>
          <p className="mt-2 text-slate-400">AI-powered rebalancing suggestions (suggest-only, no execution)</p>
        </div>
        <button
          onClick={generateMockRecommendations}
          disabled={generating}
          className="px-6 py-3 bg-gradient-to-r from-fuchsia-600 to-violet-600 hover:from-fuchsia-500 hover:to-violet-500 text-white font-medium rounded-lg border-2 border-fuchsia-500/50 shadow-[0_0_20px_rgba(217,70,239,0.4)] hover:shadow-[0_0_30px_rgba(217,70,239,0.6)] transition-all duration-300 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {generating ? 'Generating...' : 'Generate Mock Recommendations'}
        </button>
      </div>

      {recommendations.length === 0 ? (
        <div className="bg-slate-950/70 backdrop-blur-sm rounded-xl border-2 border-fuchsia-500/30 p-12 text-center shadow-[0_0_20px_rgba(217,70,239,0.2)]">
          <svg className="mx-auto h-16 w-16 text-fuchsia-400/50 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
          </svg>
          <p className="text-lg text-slate-300 mb-4">No recommendations available yet</p>
          <p className="text-sm text-slate-400">Click "Generate Mock Recommendations" to create sample recommendations for this portfolio.</p>
        </div>
      ) : (
        <div className="grid gap-6">
          {recommendations.map((rec) => (
            <Link
              key={rec.id}
              href={`/portfolios/${portfolioId}/recommendations/${rec.id}`}
              className="block group"
            >
              <div className="bg-slate-950/70 backdrop-blur-sm rounded-xl border-2 border-fuchsia-500/30 p-6 hover:border-fuchsia-400/50 transition-all duration-300 shadow-[0_0_20px_rgba(217,70,239,0.2)] hover:shadow-[0_0_30px_rgba(217,70,239,0.4)] hover:scale-[1.01]">
                {/* Header */}
                <div className="flex justify-between items-start mb-4">
                  <div className="flex gap-3">
                    <span className={`px-3 py-1 rounded-lg text-xs font-medium border ${getStatusColor(rec.status)}`}>
                      {rec.status.toUpperCase()}
                    </span>
                    <span className={`px-3 py-1 rounded-lg text-xs font-medium border ${getTypeColor(rec.recommendation_type)}`}>
                      {rec.recommendation_type.replace('_', ' ').toUpperCase()}
                    </span>
                  </div>
                  <div className="text-right text-sm text-slate-400">
                    {formatDateTime(rec.created_at)}
                  </div>
                </div>

                {/* Rationale */}
                <p className="text-slate-300 mb-4 leading-relaxed">{rec.rationale}</p>

                {/* Proposed Orders Summary */}
                <div className="bg-slate-900/50 rounded-lg p-4 mb-4">
                  <h4 className="text-sm font-medium text-cyan-300 mb-2 drop-shadow-[0_0_6px_rgba(103,232,249,0.4)]">
                    Proposed Orders ({rec.proposed_orders.length})
                  </h4>
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                    {rec.proposed_orders.slice(0, MAX_ORDERS_PREVIEW).map((order, idx) => (
                      <div key={idx} className="text-xs">
                        <span className={`font-medium ${order.action === 'buy' ? 'text-green-400' : 'text-red-400'}`}>
                          {order.action.toUpperCase()}
                        </span>
                        <span className="text-slate-300"> {order.asset}</span>
                        <div className="text-slate-400">Qty: {order.quantity}</div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Footer */}
                <div className="flex justify-between items-center text-sm">
                  {rec.expected_impact && (
                    <div className="text-slate-400">
                      Expected Impact: <span className="text-green-400 font-medium">{formatCurrency(rec.expected_impact)}</span>
                    </div>
                  )}
                  {rec.metadata && (
                    <div className="flex gap-4 text-slate-400">
                      {rec.metadata.confidence && (
                        <span>Confidence: <span className="text-cyan-300">{(rec.metadata.confidence * 100).toFixed(0)}%</span></span>
                      )}
                      {rec.metadata.risk_score !== undefined && (
                        <span>Risk: <span className="text-violet-300">{(rec.metadata.risk_score * 10).toFixed(1)}/10</span></span>
                      )}
                    </div>
                  )}
                </div>
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
