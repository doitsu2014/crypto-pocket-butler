# Feasibility & Constraints — Working Diary

- **2026-11-05T07:05:20Z — Interpretation** — The USD 150,000 budget was read as non-salary spend: a 3–5 engineer × 6–9 month plan is ~18–45 engineer-months and would otherwise exceed it by 1.2–4.5×. The user confirmed the salary exclusion (Q5).
- **2026-11-05T07:05:20Z — Deviation** — The confirmed 6–9 month / 3–5 engineer target does not close against the estimated 25–44 engineer-months. Raised as Q6; the user chose Option B (10 services, 5 engineers, 9-month target with a written cut list, 10–11 months for full scope).
- **2026-11-05T07:05:20Z — Deviation** — The full EU+UK legal workstream puts non-salary spend at USD 130k–165k. Raised as Q7; the user chose to keep EU+UK first with legal capped at ~USD 50k so total spend stays ≤ USD 150k. The UK stays a gated launch market with a dated go/no-go, not parked.
- **2026-11-05T07:05:20Z — Deviation** — Notification, Analytics and the Support Console are BRD Must scope but the 10-service plan defers them. Raised as Q8; the user chose to defer with documented degraded paths and record an explicit BRD deviation.
- **2026-11-05T07:05:20Z — Tradeoff** — The AWS platform agent was retained for its reliability, security and cost lenses but explicitly redirected to assess Docker Swarm + Supabase + VPS rather than recommend AWS services.
- **2026-11-05T07:05:20Z — Open question** — Whether any market-data vendor grants storage, history, redistribution and cross-source derived-valuation rights at retail price; this is the largest launch-blocking commercial dependency and has no budget line.
- **2026-11-05T07:05:20Z — Open question** — Whether MiCA treats a documented, untransmitted order intent as reception/transmission; the answer decides whether the order-intent surface ships at launch or behind a licensed partner.

## Learning candidates for the gate

1. "When a confirmed budget and a bottom-up effort estimate disagree, surface the contradiction as a numbered question before generating artifacts; never silently re-baseline the user's answer." (Project memory)
2. "Treat a market-data licence for storage, history, redistribution and derived valuations as a launch gate, not a procurement detail." (Project memory)
