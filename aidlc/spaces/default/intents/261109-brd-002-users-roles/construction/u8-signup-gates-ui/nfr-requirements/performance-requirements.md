# Performance Requirements — u8-signup-gates-ui

**Upstream:** `functional-spec.md` (revision 2 — §2 journey, §6 G1/G2/G3), `requirements.md`
(NFR21–NFR26), `contract-summary.md` (timeouts/retries, error envelope), codekb
`technology-stack.md` (size-limit budgets, Vitest/MSW tooling).
**Prefix note:** latency/screen-response rows inherit the BRD-001 intent's `NFR2` prefix
(dashboard response p95 < 2 s) and refine it per path for this UI unit; the bundle budgets are
the repository's existing size-limit gates. Sub-numbers are unit-local (the same `NFRx.y` in
two units is two requirements).

## Screen and network budgets

| ID | Path | Target | Basis |
|---|---|---|---|
| NFR2.1 | Gate route chunk (gate screen + country data module) | ≤ 150 kB gzip; the 300 kB initial entry chunk is unchanged by this unit | Existing `web/` size-limit budgets; the country module (~250 entries × two short names) is a static generated module, est. 10–20 kB |
| NFR2.2 | Entry gate-state resolution (G1 + G2, sequential reads) | p95 < 2 s perceived; each call is client-bounded by a 15 s deadline (the contract's connect 5 s + read 10 s): on deadline or failure the full-page retry is guaranteed to appear — the screen never hangs unresolved and never routes into the app. G1's "no record" 404 is a **normal routing answer** (Gate Screen), not an error, and is never retried | Inherits BRD-001 NFR2; `contract-summary.md` timeout rules |
| NFR2.3 | Submit (G3: gateway-derived IP + C8 → C3 → C6) | p95 < 2 s perceived; the `submitting` state is visible within 100 ms of activation and disables Continue for the round trip. The call is client-bounded by a **15 s deadline** (aligned to the contract's connect 5 s + read 10 s): when it fires, the screen leaves `submitting` and shows the inline retryable error (`incomplete` variant — spec §2.6), field values kept, retry reuses the same idempotency key; a hung gateway can never strand the screen | Server sequence is three internal hops; double-submit is prevented by the disabled control and backstopped by the idempotency key |
| NFR2.4 | Country combobox filtering | keystroke-to-render < 100 ms over the full ISO 3166-1 list; in-memory only, no network per keystroke. **Guidance, not a mechanical gate:** checked by a devtools profiling pass during Build and Test review, not by CI | Static module (§2.2 of the spec); a search that hits the network would put a third-party call in the browser |
| NFR2.5 | Controlled input responsiveness | every field responds < 100 ms; no main-thread task > 200 ms on the gate route. **Guidance, not a mechanical gate:** checked by the same devtools profiling pass | Standard SPA responsiveness floor for a form-only screen |

## Retry and load behaviour

| ID | Requirement |
|---|---|
| NFR2.6 | No automatic retries of G3: retries are manual only (spec §2.6). G1/G2 queries may use TanStack Query's bounded retry, capped at the contract's 3 attempts with backoff, and — per the same contract clause — **only on network failures and 5xx; never on 400/403/404/422**. G1's no-record 404 is a normal routing answer, not a retryable error (NFR2.2); G3's mutation retry count is 0 — the visitor re-submits with the same idempotency key |
| NFR2.7 | Gate-state queries use `staleTime: 0` for entry resolution (freshness is a security requirement, NFR24.1 in `security-requirements.md`); no polling and no background refetch loops on the gate route |

## Resource constraints

- No new runtime dependencies (MSW is dev-only; the country module is generated data — see
  `tech-stack-decisions.md` T1/T2).
- No business or compliance computation in the browser: the client-side validation is four
  presence checks; the jurisdiction decision, consent sufficiency, and provisioning all run
  server-side (spec §5).

## Measurement ownership

The p95 < 2 s targets split by ownership: **this unit owns the client side** — issuing the
calls promptly, the 15 s deadlines, the submitting/error surfaces, and no added latency of its
own (measured in the MSW-backed suite, which asserts the deadline behaviour: a delayed handler
must produce the retryable-error surface, and the submitting/disabled and no-auto-retry rules).
**The server hops are owned by the gateway and Identity** (C8 → C3 → C6 budgets belong to
U6/U1/U4's own NFR records) and are measured by their service metrics, not here; a
network-latency regression shows up in the gateway's metrics, not in this unit's suite.

## Benchmarks

No benchmark suite is introduced. NFR2.1 is guarded mechanically by the existing `yarn size`
size-limit gate; NFR2.2/NFR2.3/NFR2.6 by the MSW-backed Vitest suite; NFR2.4/NFR2.5 are
guidance checked by a devtools profiling pass during Build and Test review — no automated
check exists for them and none is assumed.
