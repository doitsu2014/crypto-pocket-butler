/**
 * Reusable loading skeleton components with cyberpunk theme
 */

interface LoadingSkeletonProps {
  count?: number;
  type?: "card" | "list" | "table";
}

export function LoadingSkeleton({ count = 3, type = "card" }: LoadingSkeletonProps) {
  const items = Array.from({ length: count }, (_, i) => i);

  if (type === "card") {
    return (
      <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
        {items.map((i) => (
          <div
            key={i}
            className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-2xl p-6"
          >
            <div className="animate-pulse space-y-4">
              <div className="flex items-center justify-between">
                <div className="h-6 bg-violet-900/50 rounded w-2/3 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
                <div className="h-10 w-10 bg-violet-900/50 rounded-lg shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
              </div>
              <div className="h-4 bg-violet-900/50 rounded w-1/2 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
              <div className="h-4 bg-violet-900/50 rounded w-3/4 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (type === "list") {
    return (
      <div className="space-y-4">
        {items.map((i) => (
          <div
            key={i}
            className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-xl p-4"
          >
            <div className="animate-pulse space-y-3">
              <div className="h-5 bg-violet-900/50 rounded w-1/3 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
              <div className="h-4 bg-violet-900/50 rounded w-full shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
            </div>
          </div>
        ))}
      </div>
    );
  }

  // Table skeleton
  return (
    <div className="bg-slate-950/70 backdrop-blur-sm border-2 border-violet-500/40 shadow-[0_0_25px_rgba(139,92,246,0.3)] rounded-2xl overflow-hidden">
      <div className="animate-pulse">
        <div className="h-12 bg-violet-900/30 border-b-2 border-violet-500/40"></div>
        {items.map((i) => (
          <div key={i} className="h-16 border-b border-violet-500/20 flex items-center px-6 gap-4">
            <div className="h-4 bg-violet-900/50 rounded w-1/4 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
            <div className="h-4 bg-violet-900/50 rounded w-1/3 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
            <div className="h-4 bg-violet-900/50 rounded w-1/4 shadow-[0_0_10px_rgba(139,92,246,0.2)]"></div>
          </div>
        ))}
      </div>
    </div>
  );
}

interface LoadingSpinnerProps {
  size?: "sm" | "md" | "lg";
  message?: string;
}

export function LoadingSpinner({ size = "md", message }: LoadingSpinnerProps) {
  const sizeClasses = {
    sm: "w-6 h-6",
    md: "w-10 h-10",
    lg: "w-16 h-16",
  };

  return (
    <div className="flex flex-col items-center justify-center py-12">
      <div className={`${sizeClasses[size]} relative`}>
        <div className="absolute inset-0 border-4 border-violet-500/30 rounded-full"></div>
        <div className="absolute inset-0 border-4 border-transparent border-t-fuchsia-500 rounded-full animate-spin shadow-[0_0_15px_rgba(217,70,239,0.6)]"></div>
      </div>
      {message && (
        <p className="mt-4 text-slate-400 text-sm animate-pulse">{message}</p>
      )}
    </div>
  );
}

interface LoadingButtonProps {
  loading?: boolean;
  children: React.ReactNode;
  className?: string;
  onClick?: () => void;
  type?: "button" | "submit" | "reset";
  disabled?: boolean;
}

export function LoadingButton({
  loading,
  children,
  className = "",
  onClick,
  type = "button",
  disabled,
}: LoadingButtonProps) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled || loading}
      className={className}
    >
      {loading && (
        <svg
          className="animate-spin -ml-1 mr-2 h-4 w-4 inline-block"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          />
        </svg>
      )}
      {children}
    </button>
  );
}
