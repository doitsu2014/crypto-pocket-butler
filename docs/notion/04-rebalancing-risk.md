# 04 — Rebalancing & Risk Engine

# Rebalancing & Risk Engine
## Rebalancing modes
- Target weights (fixed): BTC 50%, ETH 20%, Stable 20%, Alts 10% (example).
- Drift bands: only rebalance when drift > X% (e.g., 3–5%).
## Constraints
- Min trade size + fees + slippage estimate.
- Exposure caps per asset and per category (memes, L2, DeFi, etc.).
- Stablecoin buffer (e.g., keep 10–30% liquid).
## Risk signals (for the agent)
1. Volatility spike → split buys / avoid chasing.
1. Liquidity stress → reduce leverage / raise stablecoin buffer.
1. Concentration risk → suggest trimming oversized positions.
## Output format
- Proposed orders: venue, symbol, side, size, expected post-trade allocation.
## Decision
Selected approach: Fixed target allocation + simple guardrails (no fully dynamic risk-parity).

### Guardrails (to finalize)
- Drift band: rebalance only if bucket deviates more than ±X% (suggest 3–7%).
- Stablecoin buffer: keep at least Y% in stables (suggest 10–30%).
- Futures cap: total futures margin/exposure limited to Z% (suggest 5–10%).
- Max single-asset cap (non-BTC/ETH): W% (suggest 5–15%).
