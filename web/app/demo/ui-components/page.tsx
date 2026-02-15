"use client";

import { useState } from "react";
import { useToast } from "@/contexts/ToastContext";
import { LoadingSkeleton, LoadingSpinner, LoadingButton } from "@/components/Loading";

export default function ComponentDemoPage() {
  const toast = useToast();
  const [showCardSkeleton, setShowCardSkeleton] = useState(false);
  const [showListSkeleton, setShowListSkeleton] = useState(false);
  const [showTableSkeleton, setShowTableSkeleton] = useState(false);
  const [showSpinner, setShowSpinner] = useState(false);
  const [buttonLoading, setButtonLoading] = useState(false);

  const simulateLoading = (setter: (val: boolean) => void) => {
    setter(true);
    setTimeout(() => setter(false), 3000);
  };

  const simulateButtonAction = async () => {
    setButtonLoading(true);
    await new Promise(resolve => setTimeout(resolve, 2000));
    setButtonLoading(false);
    toast.success("Action completed successfully!");
  };

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 p-8">
      <div className="max-w-7xl mx-auto space-y-12">
        {/* Header */}
        <div className="text-center space-y-4">
          <h1 className="text-4xl font-bold bg-gradient-to-r from-violet-400 to-fuchsia-400 bg-clip-text text-transparent">
            UI Components Demo
          </h1>
          <p className="text-slate-400 text-lg">
            Interactive showcase of toast notifications and loading components
          </p>
        </div>

        {/* Toast Notifications Section */}
        <section className="space-y-6">
          <div className="border-b-2 border-violet-500/30 pb-4">
            <h2 className="text-2xl font-semibold text-violet-400">
              🔔 Toast Notifications
            </h2>
            <p className="text-slate-400 mt-2">
              Click the buttons below to see different toast notification types
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <button
              onClick={() => toast.success("This is a success message! Everything went well.")}
              className="bg-green-950/50 border-2 border-green-500/50 hover:border-green-500 rounded-xl p-6 transition-all shadow-[0_0_15px_rgba(34,197,94,0.2)] hover:shadow-[0_0_25px_rgba(34,197,94,0.4)]"
            >
              <div className="text-green-400 text-4xl mb-2">✓</div>
              <div className="font-semibold text-green-300">Success Toast</div>
              <div className="text-sm text-slate-400 mt-1">
                Positive feedback for successful actions
              </div>
            </button>

            <button
              onClick={() => toast.error("An error occurred! Please try again or contact support.")}
              className="bg-red-950/50 border-2 border-red-500/50 hover:border-red-500 rounded-xl p-6 transition-all shadow-[0_0_15px_rgba(239,68,68,0.2)] hover:shadow-[0_0_25px_rgba(239,68,68,0.4)]"
            >
              <div className="text-red-400 text-4xl mb-2">✕</div>
              <div className="font-semibold text-red-300">Error Toast</div>
              <div className="text-sm text-slate-400 mt-1">
                Alert users about errors and failures
              </div>
            </button>

            <button
              onClick={() => toast.warning("Warning: This action may have unintended consequences.")}
              className="bg-yellow-950/50 border-2 border-yellow-500/50 hover:border-yellow-500 rounded-xl p-6 transition-all shadow-[0_0_15px_rgba(234,179,8,0.2)] hover:shadow-[0_0_25px_rgba(234,179,8,0.4)]"
            >
              <div className="text-yellow-400 text-4xl mb-2">⚠</div>
              <div className="font-semibold text-yellow-300">Warning Toast</div>
              <div className="text-sm text-slate-400 mt-1">
                Caution users about potential issues
              </div>
            </button>

            <button
              onClick={() => toast.info("Did you know? You can customize toast duration!")}
              className="bg-cyan-950/50 border-2 border-cyan-500/50 hover:border-cyan-500 rounded-xl p-6 transition-all shadow-[0_0_15px_rgba(34,211,238,0.2)] hover:shadow-[0_0_25px_rgba(34,211,238,0.4)]"
            >
              <div className="text-cyan-400 text-4xl mb-2">ℹ</div>
              <div className="font-semibold text-cyan-300">Info Toast</div>
              <div className="text-sm text-slate-400 mt-1">
                Share helpful information with users
              </div>
            </button>
          </div>

          <div className="bg-slate-900/50 border-2 border-violet-500/30 rounded-xl p-6">
            <h3 className="text-lg font-semibold text-violet-300 mb-4">Advanced Examples</h3>
            <div className="flex flex-wrap gap-3">
              <button
                onClick={() => toast.success("Quick message", 2000)}
                className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg transition-colors text-sm"
              >
                Quick Toast (2s)
              </button>
              <button
                onClick={() => toast.error("Long message that will stay visible", 10000)}
                className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg transition-colors text-sm"
              >
                Long Toast (10s)
              </button>
              <button
                onClick={() => {
                  toast.success("First notification");
                  setTimeout(() => toast.info("Second notification"), 500);
                  setTimeout(() => toast.warning("Third notification"), 1000);
                }}
                className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg transition-colors text-sm"
              >
                Multiple Toasts
              </button>
            </div>
          </div>
        </section>

        {/* Loading Skeletons Section */}
        <section className="space-y-6">
          <div className="border-b-2 border-violet-500/30 pb-4">
            <h2 className="text-2xl font-semibold text-violet-400">
              ⏳ Loading Skeletons
            </h2>
            <p className="text-slate-400 mt-2">
              Click buttons to see different skeleton loading patterns
            </p>
          </div>

          {/* Card Skeleton */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-slate-300">Card Grid Skeleton</h3>
              <button
                onClick={() => simulateLoading(setShowCardSkeleton)}
                className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg transition-colors text-sm"
                disabled={showCardSkeleton}
              >
                {showCardSkeleton ? "Loading..." : "Show Demo"}
              </button>
            </div>
            {showCardSkeleton ? (
              <LoadingSkeleton count={3} type="card" />
            ) : (
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
                {[1, 2, 3].map((i) => (
                  <div
                    key={i}
                    className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-2xl p-6"
                  >
                    <div className="flex items-center justify-between mb-4">
                      <h4 className="text-lg font-semibold text-slate-200">Portfolio {i}</h4>
                      <div className="text-2xl">💼</div>
                    </div>
                    <p className="text-slate-400">Total Value: $10,000</p>
                    <p className="text-green-400 text-sm">+5.2% today</p>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* List Skeleton */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-slate-300">List Skeleton</h3>
              <button
                onClick={() => simulateLoading(setShowListSkeleton)}
                className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg transition-colors text-sm"
                disabled={showListSkeleton}
              >
                {showListSkeleton ? "Loading..." : "Show Demo"}
              </button>
            </div>
            {showListSkeleton ? (
              <LoadingSkeleton count={4} type="list" />
            ) : (
              <div className="space-y-4">
                {["Bitcoin", "Ethereum", "Cardano", "Polkadot"].map((name) => (
                  <div
                    key={name}
                    className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-xl p-4"
                  >
                    <h4 className="text-lg font-semibold text-slate-200">{name}</h4>
                    <p className="text-slate-400 text-sm">Holdings: 1.5 units</p>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Table Skeleton */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-slate-300">Table Skeleton</h3>
              <button
                onClick={() => simulateLoading(setShowTableSkeleton)}
                className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg transition-colors text-sm"
                disabled={showTableSkeleton}
              >
                {showTableSkeleton ? "Loading..." : "Show Demo"}
              </button>
            </div>
            {showTableSkeleton ? (
              <LoadingSkeleton count={5} type="table" />
            ) : (
              <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-2xl overflow-hidden">
                <table className="w-full">
                  <thead className="bg-violet-900/30 border-b-2 border-violet-500/40">
                    <tr>
                      <th className="px-6 py-3 text-left text-slate-300">Asset</th>
                      <th className="px-6 py-3 text-left text-slate-300">Amount</th>
                      <th className="px-6 py-3 text-left text-slate-300">Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {["BTC", "ETH", "ADA", "DOT", "SOL"].map((asset, i) => (
                      <tr key={asset} className="border-b border-violet-500/20">
                        <td className="px-6 py-4 text-slate-200">{asset}</td>
                        <td className="px-6 py-4 text-slate-400">{(i + 1) * 0.5}</td>
                        <td className="px-6 py-4 text-slate-400">${(i + 1) * 1000}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </section>

        {/* Loading Spinners Section */}
        <section className="space-y-6">
          <div className="border-b-2 border-violet-500/30 pb-4">
            <h2 className="text-2xl font-semibold text-violet-400">
              🌀 Loading Spinners
            </h2>
            <p className="text-slate-400 mt-2">
              Different spinner sizes for various loading contexts
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="bg-slate-900/50 border-2 border-violet-500/30 rounded-xl p-6">
              <h3 className="text-center text-slate-300 mb-4">Small Spinner</h3>
              <LoadingSpinner size="sm" />
            </div>
            <div className="bg-slate-900/50 border-2 border-violet-500/30 rounded-xl p-6">
              <h3 className="text-center text-slate-300 mb-4">Medium Spinner</h3>
              <LoadingSpinner size="md" message="Loading data..." />
            </div>
            <div className="bg-slate-900/50 border-2 border-violet-500/30 rounded-xl p-6">
              <h3 className="text-center text-slate-300 mb-4">Large Spinner</h3>
              <LoadingSpinner size="lg" message="Please wait..." />
            </div>
          </div>

          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-slate-300">Full Section Loading</h3>
              <button
                onClick={() => simulateLoading(setShowSpinner)}
                className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg transition-colors text-sm"
                disabled={showSpinner}
              >
                {showSpinner ? "Loading..." : "Show Demo"}
              </button>
            </div>
            <div className="bg-slate-900/50 border-2 border-violet-500/30 rounded-xl min-h-[200px]">
              {showSpinner ? (
                <LoadingSpinner size="lg" message="Loading your content..." />
              ) : (
                <div className="p-12 text-center text-slate-400">
                  <p className="text-lg">Your content would appear here</p>
                  <p className="text-sm mt-2">Click the button above to see the loading state</p>
                </div>
              )}
            </div>
          </div>
        </section>

        {/* Loading Button Section */}
        <section className="space-y-6">
          <div className="border-b-2 border-violet-500/30 pb-4">
            <h2 className="text-2xl font-semibold text-violet-400">
              🔘 Loading Button
            </h2>
            <p className="text-slate-400 mt-2">
              Button component with integrated loading state
            </p>
          </div>

          <div className="bg-slate-900/50 border-2 border-violet-500/30 rounded-xl p-6">
            <div className="flex flex-wrap gap-4 items-center justify-center">
              <LoadingButton
                loading={buttonLoading}
                onClick={simulateButtonAction}
                className="px-6 py-3 bg-violet-600 hover:bg-violet-700 disabled:bg-violet-600/50 text-white rounded-lg transition-colors font-medium shadow-[0_0_15px_rgba(139,92,246,0.3)]"
              >
                {buttonLoading ? "Processing..." : "Click to Simulate Action"}
              </LoadingButton>

              <LoadingButton
                loading={true}
                className="px-6 py-3 bg-green-600 text-white rounded-lg font-medium opacity-50 cursor-not-allowed"
              >
                Always Loading Button
              </LoadingButton>

              <LoadingButton
                loading={false}
                className="px-6 py-3 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors font-medium"
                onClick={() => toast.error("Action cancelled")}
              >
                Delete Item
              </LoadingButton>
            </div>
          </div>
        </section>

        {/* Code Examples Section */}
        <section className="space-y-6">
          <div className="border-b-2 border-violet-500/30 pb-4">
            <h2 className="text-2xl font-semibold text-violet-400">
              📝 Usage Examples
            </h2>
            <p className="text-slate-400 mt-2">
              Common patterns and code snippets
            </p>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div className="bg-slate-900/50 border-2 border-violet-500/30 rounded-xl p-6">
              <h3 className="text-lg font-semibold text-slate-300 mb-4">Toast Notifications</h3>
              <pre className="text-xs text-slate-400 bg-slate-950 p-4 rounded-lg overflow-x-auto">
{`import { useToast } from "@/contexts/ToastContext";

const toast = useToast();

// Show different toast types
toast.success("Portfolio saved!");
toast.error("Failed to connect");
toast.warning("Low balance");
toast.info("Tip: Enable 2FA");

// Custom duration (ms)
toast.success("Quick!", 2000);`}
              </pre>
            </div>

            <div className="bg-slate-900/50 border-2 border-violet-500/30 rounded-xl p-6">
              <h3 className="text-lg font-semibold text-slate-300 mb-4">Loading States</h3>
              <pre className="text-xs text-slate-400 bg-slate-950 p-4 rounded-lg overflow-x-auto">
{`import { LoadingSkeleton } from "@/components/Loading";

// Card grid skeleton
if (loading) {
  return <LoadingSkeleton type="card" count={6} />;
}

// List skeleton
if (loading) {
  return <LoadingSkeleton type="list" count={4} />;
}

// Table skeleton
if (loading) {
  return <LoadingSkeleton type="table" count={10} />;
}`}
              </pre>
            </div>
          </div>
        </section>

        {/* Footer */}
        <div className="text-center text-slate-500 text-sm py-8 border-t-2 border-violet-500/20">
          <p>For more details, see the components README at <code className="text-violet-400">/web/components/README.md</code></p>
        </div>
      </div>
    </div>
  );
}
