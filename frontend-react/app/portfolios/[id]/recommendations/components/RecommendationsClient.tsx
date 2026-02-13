"use client";

import { ApiError } from "@/lib/api-client";
import Link from "next/link";
import { LoadingSkeleton } from "@/components/Loading";
import EmptyState from "@/components/EmptyState";
import ErrorAlert from "@/components/ErrorAlert";
import { useRecommendations, useGenerateRecommendations } from "@/hooks";

const MAX_ORDERS_PREVIEW = 4; // Maximum number of orders to show in list view

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
  // Use TanStack Query hooks
  const { data: response, isLoading: loading, error: queryError, refetch } = useRecommendations(
    portfolioId,
    undefined
  );
  const generateRecommendations = useGenerateRecommendations();

  const recommendations = response?.recommendations || [];
  
  // Convert query error to string for display
  const error = queryError instanceof ApiError ? queryError.message : 
                queryError ? "Failed to load recommendations" : null;

  const generateMockRecommendations = async () => {
    try {
      await generateRecommendations.mutateAsync(portfolioId);
    } catch (err) {
      // Error is handled by TanStack Query's onError
      console.error('Failed to generate recommendations:', err);
    }
  };

  if (loading) {
    return <LoadingSkeleton type="list" count={3} />;
  }

  if (error) {
    return (
      <ErrorAlert 
        message={error}
        onRetry={() => refetch()}
      />
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
          disabled={generateRecommendations.isPending}
          onClick={generateMockRecommendations}
          className="px-6 py-3 bg-gradient-to-r from-fuchsia-600 to-violet-600 hover:from-fuchsia-500 hover:to-violet-500 text-white font-medium rounded-lg border-2 border-fuchsia-500/50 shadow-[0_0_20px_rgba(217,70,239,0.4)] hover:shadow-[0_0_30px_rgba(217,70,239,0.6)] transition-all duration-300 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {generateRecommendations.isPending ? 'Generating...' : 'Generate Mock Recommendations'}
        </button>
      </div>

      {recommendations.length === 0 ? (
        <EmptyState
          icon="recommendation"
          title="No recommendations available yet"
          description='Click "Generate Mock Recommendations" to create sample recommendations for this portfolio.'
          action={{
            label: "Generate Recommendations",
            onClick: generateMockRecommendations
          }}
        />
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
