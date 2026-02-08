# 05 — OpenClaw Agent Design

# OpenClaw Agent Design
## Agent loop
1. Fetch holdings + prices.
1. Compute portfolio + drift vs targets.
1. Generate recommendations + explain.
1. Write to Notion + send Telegram summary.
## Tools/skills needed
- OKX connector (read-only first).
- Wallet indexer connector (EVM first).
- Notion writer (daily brief + tables).
## Scheduling
- Cron daily (e.g., 08:00) + on-demand run.
## Human-in-the-loop
- Agent proposes; you approve trades until trust is built.
