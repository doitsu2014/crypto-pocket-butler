import { Component, type ReactNode } from "react";
import { reportClientError, sessionId } from "../telemetry";

/**
 * A full-page recovery surface for a failed bootstrap or a render crash
 * (functional spec: "a fatal error renders a full-page recovery action").
 * The crash is reported once to the client telemetry route.
 */
export class ErrorBoundary extends Component<
  { children: ReactNode; fallback?: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError(): { failed: boolean } {
    return { failed: true };
  }

  componentDidCatch(): void {
    reportClientError({
      route: typeof window === "undefined" ? "/" : window.location.pathname,
      errorCode: "bootstrap_failed",
      correlationId: sessionId(),
    });
  }

  render(): ReactNode {
    if (this.state.failed) {
      return (
        this.props.fallback ?? (
          <main className="flex min-h-screen items-center justify-center bg-parchment p-6">
            <section className="max-w-md rounded-lg border border-border bg-surface p-8 text-center">
              <h1 className="font-display text-section font-medium text-ink">
                Something went wrong
              </h1>
              <p className="mt-2 text-small text-muted">
                The app could not start. Reload the page to try again.
              </p>
              <button
                type="button"
                data-testid="error-boundary-reload"
                onClick={() => window.location.reload()}
                className="mt-4 rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on"
              >
                Reload
              </button>
            </section>
          </main>
        )
      );
    }
    return this.props.children;
  }
}
