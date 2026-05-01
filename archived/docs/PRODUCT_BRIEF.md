# Crypto Pocket Butler - Product Brief

## Vision

A unified dashboard to connect multiple crypto wallets, aggregate portfolio data, and provide intelligent portfolio management with compliance built-in.

## Target Users

| Persona | Description | Key Needs |
|---------|-------------|-----------|
| **Retail Investor** | Individual with 3-10 wallets across chains | Simple view, P&L tracking, tax reports |
| **Active Trader** | Frequent trader, multiple strategies | Real-time data, alerts, fast refresh |
| **Family Office** | Managing wealth for families | Multi-user, reporting, compliance, audit trails |
| **Fund Manager** | Running crypto fund, client reporting | Performance analytics, white-label, client portals |

## Core Features (MVP)

### 1. Wallet Connection
- [ ] MetaMask, WalletConnect v2 support
- [ ] Ethereum, BSC, Polygon, Arbitrum, Optimism
- [ ] Read-only address import (watch mode)
- [ ] Multi-wallet aggregation

### 2. Portfolio Tracking
- [ ] Real-time valuation (CoinGecko + on-chain fallbacks)
- [ ] Cost basis tracking (FIFO method first)
- [ ] P&L calculation (realized/unrealized)
- [ ] Asset allocation visualization

### 3. Multi-User Access
- [ ] Role-based access (Owner, Admin, Analyst, Viewer)
- [ ] Shared portfolios
- [ ] Activity audit logs

### 4. Configuration
- [ ] User preferences (currency, refresh rate)
- [ ] Custom price sources
- [ ] Notification settings

## Advanced Features (Post-MVP)

### 5. Investment Insights
- [ ] Performance metrics (IRR, CAGR, Sharpe ratio)
- [ ] Attribution analysis (by asset, chain, strategy)
- [ ] Risk analytics (concentration, VaR, correlation)
- [ ] Benchmark comparison (BTC, ETH, S&P500)

### 6. Rebalancing
- [ ] Target allocation setup
- [ ] Drift alerts (>5% deviation)
- [ ] Rebalance recommendations
- [ ] Tax-loss harvesting suggestions
- [ ] One-click execution (via DEX aggregator)

### 7. Compliance Check
- [ ] Transaction history export (CSV, PDF)
- [ ] Capital gains reports (FIFO, LIFO, HIFO)
- [ ] Staking/income tracking
- [ ] Sanctioned address screening
- [ ] Geographic restrictions

## Technical Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (React)                      │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐   │
│  │   Dashboard │ │  Portfolio  │ │    Analytics    │   │
│  └─────────────┘ └─────────────┘ └─────────────────┘   │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                      API Layer                           │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐   │
│  │   Wallet    │ │  Portfolio  │ │    Compliance   │   │
│  │   Service   │ │   Service   │ │     Service     │   │
│  └─────────────┘ └─────────────┘ └─────────────────┘   │
└─────────────────────────────────────────────────────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
     ┌─────────────┐ ┌──────────┐ ┌──────────┐
     │Price Feeds  │ │ On-Chain │ │  Users   │
     │CoinGecko    │ │  Indexer │ │   DB     │
     │Chainlink    │ │  TheGraph│ │ Postgres │
     └─────────────┘ └──────────┘ └──────────┘
```

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Portfolio Accuracy | >99% | Vs. manual calculation |
| Data Refresh Latency | <30 seconds | Price updates |
| Supported Wallets | 5+ | MetaMask, WC, Ledger, etc. |
| Supported Chains | 10+ | EVM + Solana |
| Time to First Value | <2 minutes | Connect wallet → see portfolio |

## Competitive Landscape

| Competitor | Strengths | Weaknesses | Our Differentiation |
|------------|-----------|------------|---------------------|
| DeBank | Great UX, multi-chain | No rebalancing, limited analytics | Rebalancing + compliance |
| Zapper | DeFi focus, good UI | Limited institutional features | Multi-user, audit trails |
| Nansen | Powerful analytics | Expensive, complex | Simpler, affordable |
| Koinly | Tax focus | Not real-time portfolio | Real-time + tax |

## Pricing Strategy

| Tier | Price | Features |
|------|-------|----------|
| **Free** | $0 | 3 wallets, basic tracking, 24h refresh |
| **Pro** | $15/mo | Unlimited wallets, real-time, insights |
| **Team** | $50/mo | Multi-user, audit logs, reports |
| **Enterprise** | Custom | White-label, API access, SLA |

## Roadmap

### Q2 2026 — Foundation
- Wallet connection (MetaMask, WC)
- Basic portfolio tracking
- 5 chain support

### Q3 2026 — Growth
- Multi-user access
- Mobile app (iOS/Android)
- 10+ chain support

### Q4 2026 — Intelligence
- Investment insights
- Rebalancing recommendations
- Tax reports

### Q1 2027 — Scale
- Institutional features
- API for developers
- White-label options

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Price feed failures | High | Multiple sources, caching |
| API rate limits | Medium | Tiered refresh rates, paid tiers |
| Security concerns | Critical | Read-only access, encryption, audits |
| Regulatory changes | High | Flexible compliance engine, legal review |
| Competition | Medium | Focus on underserved segments (family offices) |

---

**Last Updated:** 2026-04-21  
**Owner:** Pat (Crypto Portfolio PM)  
**Status:** Draft
