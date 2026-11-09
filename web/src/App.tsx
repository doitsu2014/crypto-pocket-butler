import { useMutation } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { ApiError, NetworkError, resolveAsset, type Asset } from "./api/client";
import { getAccessToken } from "./auth/session";

const inputClass =
  "rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink focus:border-focus focus:outline-none";
const labelClass = "flex flex-col gap-1 text-small font-medium text-ink-2";

export default function App() {
  const [chain, setChain] = useState("ethereum");
  const [address, setAddress] = useState("");
  const [symbol, setSymbol] = useState("");

  const resolve = useMutation<Asset, Error, void>({
    mutationFn: () =>
      resolveAsset({ chain, contractAddress: address, symbol }, getAccessToken()),
  });

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (resolve.isPending) {
      return;
    }
    resolve.mutate();
  }

  const problem = resolve.error instanceof ApiError ? resolve.error.problem : null;
  const unresolved = problem?.status === 404;

  return (
    <main className="min-h-screen bg-parchment px-6 py-12 text-ink">
      <div className="mx-auto flex w-full max-w-3xl flex-col gap-8">
        <header>
          <p className="text-overline font-medium uppercase tracking-overline text-meta">
            Crypto Pocket Butler
          </p>
          <h1 className="font-display text-display font-medium leading-display text-ink">
            Asset registry
          </h1>
          <p className="mt-2 max-w-prose text-body leading-body text-muted">
            Resolve a chain observation to one canonical asset. An unmapped token is quarantined
            and excluded, never guessed.
          </p>
        </header>

        <form
          data-testid="asset-resolve-form"
          onSubmit={onSubmit}
          className="rounded-lg border border-border bg-surface p-6 shadow-[rgba(0,0,0,0.05)_0_4px_24px]"
        >
          <div className="grid gap-4 sm:grid-cols-3">
            <label className={labelClass}>
              Chain
              <select
                data-testid="asset-resolve-chain"
                value={chain}
                onChange={(event) => setChain(event.target.value)}
                className={inputClass}
              >
                <option value="ethereum">Ethereum</option>
                <option value="bnb-smart-chain">BNB Smart Chain</option>
                <option value="solana">Solana</option>
              </select>
            </label>
            <label className={labelClass}>
              Contract address
              <input
                data-testid="asset-resolve-address"
                value={address}
                onChange={(event) => setAddress(event.target.value)}
                placeholder="0x…"
                autoComplete="off"
                spellCheck={false}
                className={`${inputClass} font-mono`}
              />
            </label>
            <label className={labelClass}>
              Symbol
              <input
                data-testid="asset-resolve-symbol"
                value={symbol}
                onChange={(event) => setSymbol(event.target.value)}
                placeholder="ETH"
                autoComplete="off"
                spellCheck={false}
                className={`${inputClass} font-mono uppercase`}
              />
            </label>
          </div>
          <button
            data-testid="asset-resolve-submit"
            type="submit"
            disabled={resolve.isPending}
            className="mt-6 rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:opacity-45"
          >
            {resolve.isPending ? "Resolving…" : "Resolve asset"}
          </button>
        </form>

        {resolve.data ? (
          <section
            data-testid="asset-result"
            aria-live="polite"
            className="rounded-lg border border-border bg-surface p-6 shadow-[rgba(0,0,0,0.05)_0_4px_24px]"
          >
            <p className="text-overline font-medium uppercase tracking-overline text-meta">
              Canonical asset
            </p>
            <div className="mt-2 flex flex-wrap items-baseline gap-3">
              <h2 className="font-display text-kpi font-medium leading-display text-ink">
                {resolve.data.symbol}
              </h2>
              <span className="text-small text-muted">{resolve.data.name}</span>
            </div>
            <dl className="mt-4 grid grid-cols-2 gap-4 text-small">
              <div>
                <dt className="text-meta">Decimals</dt>
                <dd className="font-mono tabular-nums text-ink">{resolve.data.decimals}</dd>
              </div>
              <div>
                <dt className="text-meta">Status</dt>
                <dd className="font-medium text-ink">
                  {resolve.data.canonical ? "Canonical" : "Placeholder"}
                </dd>
              </div>
            </dl>
          </section>
        ) : null}

        {unresolved && problem ? (
          <section
            data-testid="asset-unresolved"
            aria-live="polite"
            className="rounded-lg border border-border bg-surface-warm p-6"
          >
            <h2 className="font-display text-section font-medium leading-display text-ink">
              Unresolved observation
            </h2>
            <p className="mt-2 text-small text-ink-2">
              This token is quarantined and excluded from portfolio totals. An operator must map
              it or ignore it; it is never guessed.
            </p>
            <p className="mt-3 text-small text-muted">
              Correlation ID <span className="font-mono text-ink">{problem.correlationId}</span>
            </p>
          </section>
        ) : null}

        {resolve.error && !unresolved ? (
          <section
            data-testid="asset-error"
            role="alert"
            aria-live="assertive"
            className="rounded-lg border border-danger bg-surface p-6"
          >
            <h2 className="font-display text-section font-medium leading-display text-ink">
              Could not resolve the asset
            </h2>
            <p className="mt-2 text-small text-ink-2">
              {resolve.error instanceof NetworkError
                ? "Could not reach the gateway. Check your connection and try again."
                : resolve.error.message}
            </p>
            {problem ? (
              <p className="mt-3 text-small text-muted">
                Correlation ID <span className="font-mono text-ink">{problem.correlationId}</span>
              </p>
            ) : null}
          </section>
        ) : null}
      </div>
    </main>
  );
}
