/**
 * Reusable error alert component with cyberpunk theme
 */

interface ErrorAlertProps {
  message: string;
  onRetry?: () => void;
  onDismiss?: () => void;
  type?: "inline" | "banner";
}

export default function ErrorAlert({ message, onRetry, onDismiss, type = "inline" }: ErrorAlertProps) {
  const baseClasses = "bg-red-950/30 border-2 border-red-500/50 rounded-xl shadow-[0_0_20px_rgba(239,68,68,0.25)]";
  const inlineClasses = "p-4";
  const bannerClasses = "p-4 mb-6";

  return (
    <div className={`${baseClasses} ${type === "banner" ? bannerClasses : inlineClasses}`}>
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0 text-red-400 mt-0.5">
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <div className="flex-1">
          <p className="text-red-300 text-sm">{message}</p>
        </div>
        <div className="flex-shrink-0 flex items-center gap-2">
          {onRetry && (
            <button
              onClick={onRetry}
              className="text-red-300 hover:text-red-200 text-xs font-medium underline transition-colors"
            >
              Retry
            </button>
          )}
          {onDismiss && (
            <button
              onClick={onDismiss}
              className="text-red-400 hover:text-red-200 transition-colors"
              aria-label="Dismiss"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
