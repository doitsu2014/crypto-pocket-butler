# Crypto Pocket Butler - UI/UX Design Language

**Version:** 1.0  
**Last Updated:** 2026-04-21  
**Owner:** Casey (Crypto UI/UX Designer)  
**Status:** Draft

---

## Design Philosophy

> **"Clarity builds trust. Trust enables action."**

Our design language is built on three pillars:

### 1. Clarity First
Financial data must be instantly understandable. Every pixel serves a purpose.

### 2. Trust Through Polish
Professional, refined UI that feels secure and reliable.

### 3. Progressive Disclosure
Show essentials immediately. Details available on demand.

---

## Visual Identity

### Logo Concept

```
┌─────────────────────────────────────────┐
│                                         │
│    ░░▓▓  Crypto Pocket Butler          │
│    ▓▓░░  Simple • Secure • Smart       │
│                                         │
│  Symbol: Stylized wallet + chart       │
│  Colors: Primary blue + gradient       │
│  Style: Modern, geometric, minimal     │
│                                         │
└─────────────────────────────────────────┘
```

### Brand Colors

#### Primary Palette

```
┌─────────────────────────────────────────────────────────────┐
│  PRIMARY BLUE                                                │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │          │ │          │ │          │ │          │       │
│  │  #1A73E8 │ │  #1557B0 │ │  #0D3B7A │ │  #E8F0FE │       │
│  │  Primary │ │  Dark    │ │  Darker  │ │  Light   │       │
│  │  100%    │ │  80%     │ │  60%     │ │  20%     │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
└─────────────────────────────────────────────────────────────┘

Usage:
- Primary: CTAs, links, active states, brand elements
- Dark: Hover states, emphasis
- Darker: Text on light backgrounds
- Light: Backgrounds, subtle highlights
```

#### Semantic Colors

```
┌─────────────────────────────────────────────────────────────┐
│  SUCCESS (Gains, Positive)                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │          │ │          │ │          │ │          │       │
│  │  #10B981 │ │  #059669 │ │  #047857 │ │  #D1FAE5 │       │
│  │  Green   │ │  Dark    │ │  Darker  │ │  Light   │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
│                                                             │
│  DANGER (Losses, Errors, Negative)                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │          │ │          │ │          │ │          │       │
│  │  #EF4444 │ │  #DC2626 │ │  #B91C1C │ │  #FEE2E2 │       │
│  │  Red     │ │  Dark    │ │  Darker  │ │  Light   │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
│                                                             │
│  WARNING (Alerts, Pending)                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │          │ │          │ │          │ │          │       │
│  │  #F59E0B │ │  #D97706 │ │  #B45309 │ │  #FEF3C7 │       │
│  │  Amber   │ │  Dark    │ │  Darker  │ │  Light   │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
│                                                             │
│  INFO (Neutral Info)                                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │          │ │          │ │          │ │          │       │
│  │  #3B82F6 │ │  #2563EB │ │  #1D4ED8 │ │  #DBEAFE │       │
│  │  Blue    │ │  Dark    │ │  Darker  │ │  Light   │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
└─────────────────────────────────────────────────────────────┘
```

#### Neutral Palette

```
┌─────────────────────────────────────────────────────────────┐
│  LIGHT MODE                                                  │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐     │
│  │      │ │      │ │      │ │      │ │      │ │      │     │
│  │#111827│ │#374151│ │#6B7280│ │#9CA3AF│ │#D1D5DB│ │#F3F4F6│ │
│  │Gray900│ │Gray700│ │Gray500│ │Gray400│ │Gray300│ │Gray100│ │
│  │ Text │ │ Text │ │ Text │ │ Border│ │Border│ │ Bg    │     │
│  │Primary│ │Secondary│ │Muted │ │      │ │Light │ │      │     │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘     │
│                                                             │
│  DARK MODE                                                  │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐     │
│  │      │ │      │ │      │ │      │ │      │ │      │     │
│  │#F9FAFB│ │#D1D5DB│ │#9CA3AF│ │#6B7280│ │#374151│ │#1F2937│ │
│  │Gray50 │ │Gray300│ │Gray400│ │Gray500│ │Gray700│ │Gray800│ │
│  │ Text │ │ Text │ │ Text │ │ Border│ │Border│ │ Bg    │     │
│  │Primary│ │Secondary│ │Muted │ │      │ │Dark  │ │      │     │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘     │
└─────────────────────────────────────────────────────────────┘
```

### Colorblind-Safe Alternative

For users with color vision deficiency, provide an alternative theme:

```
┌─────────────────────────────────────────────────────────────┐
│  COLORBLIND MODE (Deuteranopia)                              │
│                                                              │
│  Gains:  #1A73E8 (Blue) + ↗ icon                            │
│  Losses: #F59E0B (Orange) + ↘ icon                          │
│                                                              │
│  Always pair color with:                                    │
│  - Icons (arrows, checkmarks, X)                            │
│  - Patterns (stripes, dots)                                 │
│  - Text labels ("+$1,234" not just green)                   │
└─────────────────────────────────────────────────────────────┘
```

---

## Typography

### Font Families

```yaml
Primary Font: Inter
  - Clean, modern, highly readable
  - Excellent screen rendering
  - Multiple weights available
  - Google Fonts (free)

Monospace Font: JetBrains Mono
  - Code, addresses, numbers
  - Tabular figures for alignment
  - Excellent character distinction

Fallback Stack:
  - Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif
  - JetBrains Mono, "Fira Code", "Source Code Pro", monospace
```

### Type Scale

```
┌─────────────────────────────────────────────────────────────┐
│  DESKTOP TYPE SCALE                                          │
│                                                              │
│  Display    48px / 56px  SemiBold  -2%  Hero sections       │
│  H1         32px / 40px  SemiBold  -1%  Page titles         │
│  H2         24px / 32px  SemiBold   0%  Section headers     │
│  H3         18px / 28px  Medium     0%  Card titles         │
│  H4         16px / 24px  Medium     0%  Subsections         │
│  Body       16px / 24px  Regular    0%  Primary text        │
│  Body Small 14px / 20px  Regular    0%  Secondary text      │
│  Caption    12px / 16px  Regular    0%  Meta, timestamps    │
│  Overline   12px / 16px  SemiBold  +4%  Labels, tags        │
│                                                              │
│  Line Height: 1.5 for body, 1.25 for headings               │
│  Letter Spacing: -0.02em for large text                     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  MOBILE TYPE SCALE                                           │
│                                                              │
│  Display    36px / 44px  SemiBold  -2%                       │
│  H1         28px / 36px  SemiBold  -1%                       │
│  H2         22px / 28px  SemiBold   0%                       │
│  H3         16px / 24px  Medium     0%                       │
│  Body       16px / 24px  Regular    0%                       │
│  Body Small 14px / 20px  Regular    0%                       │
│  Caption    11px / 16px  Regular    0%                       │
│                                                              │
│  Minimum touch target: 44px                                 │
└─────────────────────────────────────────────────────────────┘
```

### Typography Usage Examples

```
┌─────────────────────────────────────────────────────────────┐
│  PAGE TITLE                                                  │
│  Portfolio Overview                         [H1: 32px SB]   │
│                                                              │
│  Welcome back, Duc! Here's your portfolio summary.          │
│  [Body: 16px Reg]                                           │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  ASSET ALLOCATION                   [H3: 18px Med]  │   │
│  │                                                      │   │
│  │  [Pie Chart]                                         │   │
│  │                                                      │   │
│  │  Bitcoin        45.2%    $56,789    [Body Small]    │   │
│  │  Ethereum       32.1%    $40,234    [Body Small]    │   │
│  │  Others         22.7%    $28,407    [Body Small]    │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  Last updated 2 minutes ago                 [Caption: 12px] │
└─────────────────────────────────────────────────────────────┘
```

---

## Spacing System

### 8-Point Grid

```
Base Unit: 8px

Scale:
  4px   - Tight spacing (icon + text)
  8px   - Base unit
  12px  - Comfortable spacing
  16px  - Standard gap
  24px  - Section spacing
  32px  - Large gaps
  48px  - Major sections
  64px  - Page margins (desktop)
  96px  - Hero sections

Usage:
  - Component padding: 16px, 24px
  - Card gaps: 16px
  - Section gaps: 32px, 48px
  - Page margins: 24px (mobile), 64px (desktop)
```

---

## Components

### Buttons

```
┌─────────────────────────────────────────────────────────────┐
│  PRIMARY BUTTON                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Connect Wallet                          │   │
│  │  [Bg: #1A73E8] [Text: White] [Height: 48px]         │   │
│  │  [Radius: 8px] [Padding: 0 24px] [Font: 16px SB]    │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  Hover: Background #1557B0                                  │
│  Active: Background #0D3B7A                                 │
│  Disabled: Background #D1D5DB, Text #9CA3AF                 │
│                                                              │
│  SECONDARY BUTTON                                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              View Details                            │   │
│  │  [Bg: Transparent] [Border: 1px #D1D5DB]            │   │
│  │  [Text: #374151] [Height: 48px]                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  Hover: Background #F3F4F6                                  │
│                                                              │
│  TERTIARY BUTTON (Text Link)                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Learn More →                            │   │
│  │  [Text: #1A73E8] [NoBg] [NoBorder]                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  ICON BUTTON                                                 │
│  ┌────┐  ┌────┐  ┌────┐                                    │
│  │ ⚙️ │  │ ❌ │  │ ➕ │   [Size: 40px] [Radius: 8px]      │
│  └────┘  └────┘  └────┘   [Hover: #F3F4F6]                │
└─────────────────────────────────────────────────────────────┘
```

### Cards

```
┌─────────────────────────────────────────────────────────────┐
│  PORTFOLIO CARD                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  ┌─────────────────────────────────────────────┐    │   │
│  │  │  ░░▓▓  Bitcoin                              │    │   │
│  │  │         BTC                                  │    │   │
│  │  └─────────────────────────────────────────────┘    │   │
│  │                                                      │   │
│  │  2.4532 BTC                           $106,234.56   │   │
│  │  $43,298 / BTC                        +$2,345.67    │   │
│  │                                         +2.34% ↗    │   │
│  │                                                      │   │
│  │  ─────────────────────────────────────────────────  │   │
│  │  [Sparkline chart ────╱╲────]                       │   │
│  │                                                      │   │
│  │  24H: +2.34%  |  7D: +8.12%  |  30D: -3.45%        │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  Specs:                                                      │
│  - Width: 320px (desktop), 100% (mobile)                    │
│  - Height: 180px                                            │
│  - Padding: 24px                                            │
│  - Radius: 12px                                             │
│  - Shadow: 0 4px 6px rgba(0,0,0,0.1)                       │
│  - Background: White (light), #1F2937 (dark)               │
└─────────────────────────────────────────────────────────────┘
```

### Input Fields

```
┌─────────────────────────────────────────────────────────────┐
│  TEXT INPUT                                                  │
│                                                              │
│  Email Address                              [Label: 14px SB]│
│  ┌─────────────────────────────────────────────────────┐   │
│  │  you@example.com                                     │   │
│  │  [Height: 48px] [Padding: 0 16px] [Radius: 8px]     │   │
│  │  [Border: 1px #D1D5DB] [Bg: White]                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  Focus:                                                       │
│  - Border: 2px #1A73E8                                      │
│  - Shadow: 0 0 0 3px rgba(26,115,232,0.2)                  │
│                                                              │
│  Error:                                                       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Invalid email address                               │   │
│  │  [Border: 1px #EF4444] [Text: #EF4444]              │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  🔒  Password                                        │   │
│  │                                      [Show/Hide] 👁 │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Data Tables

```
┌─────────────────────────────────────────────────────────────┐
│  ASSET TABLE                                                 │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Asset        │ Balance    │ Value     │ 24h    │ Alloc │ │
│  ├──────────────┼────────────┼───────────┼────────┼───────┤ │
│  │ ░░▓▓ Bitcoin │ 2.453 BTC  │ $106,234  │ +2.34% │ 45.2% │ │
│  │ ░░▓▓ Ethereum│ 12.89 ETH  │ $40,234   │ +1.89% │ 32.1% │ │
│  │ ░░▓▓ Solana  │ 234.5 SOL  │ $28,407   │ -0.45% │ 22.7% │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  Row Specs:                                                  │
│  - Height: 64px                                             │
│  - Hover: Background #F9FAFB                               │
│  - Border-bottom: 1px #F3F4F6                              │
│  - Padding: 0 16px                                          │
│                                                              │
│  Column Alignment:                                           │
│  - Asset: Left                                             │
│  - Balance: Right (tabular figures)                        │
│  - Value: Right (tabular figures)                          │
│  - 24h: Right (color-coded)                                │
│  - Alloc: Right (with mini bar)                            │
└─────────────────────────────────────────────────────────────┘
```

### Charts & Visualization

```
┌─────────────────────────────────────────────────────────────┐
│  PORTFOLIO ALLOCATION PIE CHART                              │
│                                                              │
│                    ┌─────────────────┐                      │
│                    │     ╱────╲      │   Bitcoin 45.2%     │
│                    │   ╱   BTC  ╲    │   ████████████░░░░  │
│                    │  │    45%   │   │                      │
│                    │   ╲        ╱    │   Ethereum 32.1%    │
│                    │     ╲────╱      │   █████████░░░░░░░  │
│                    │                 │                      │
│                    │   [Interactive] │   Solana 22.7%      │
│                    │   - Hover:      │   ███████░░░░░░░░░  │
│                    │     Highlight   │                      │
│                    │   - Click:      │   Others 0.0%       │
│                    │     Filter      │   ░░░░░░░░░░░░░░░░  │
│                    └─────────────────┘                      │
│                                                              │
│  Colors (Colorblind-Safe):                                   │
│  - Bitcoin: #1A73E8 (Blue)                                  │
│  - Ethereum: #8B5CF6 (Purple)                               │
│  - Solana: #10B981 (Green)                                  │
│  - Others: #9CA3AF (Gray)                                   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  VALUE OVER TIME LINE CHART                                  │
│                                                              │
│  Portfolio Value                           [1D|1W|1M|3M|1Y] │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ $150K ┤                                              │  │
│  │       │                                    ╱────     │  │
│  │ $125K ┤                              ╱────╱          │  │
│  │       │                        ╱────╱               │  │
│  │ $100K ┤                  ╱────╱                     │  │
│  │       │            ╱────╱                           │  │
│  │  $75K ┤      ╱────╱                                 │  │
│  │       │╱────╱                                       │  │
│  │  $50K ┤                                              │  │
│  │       └────┬────┬────┬────┬────┬────┬────┬────┬────  │  │
│  │       Jan  Feb  Mar  Apr  May  Jun  Jul  Aug  Sep    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  Current: $125,430  |  +$25,430 (+25.4%)                    │
│                                                              │
│  Features:                                                   │
│  - Smooth curve (bezier)                                    │
│  - Gradient fill below line                                 │
│  - Interactive: hover for exact value                       │
│  - Markers for key events (buys, sells)                     │
│  - Benchmark comparison toggle (BTC, ETH, S&P500)          │
└─────────────────────────────────────────────────────────────┘
```

### Wallet Connection Modal

```
┌─────────────────────────────────────────────────────────────┐
│                                                              │
│              ┌─────────────────────────────────┐            │
│              │         Connect Wallet          │            │
│              │                                 │            │
│              │  Choose your wallet provider:   │            │
│              │                                 │            │
│              │  ┌───────────────────────────┐ │            │
│              │  │  🦊  MetaMask             │ │            │
│              │  │      Browser wallet       │ │            │
│              │  └───────────────────────────┘ │            │
│              │                                 │            │
│              │  ┌───────────────────────────┐ │            │
│              │  │  🔗  WalletConnect        │ │            │
│              │  │      Mobile wallets       │ │            │
│              │  └───────────────────────────┘ │            │
│              │                                 │            │
│              │  ┌───────────────────────────┐ │            │
│              │  │  📴  Ledger               │ │            │
│              │  │      Hardware wallet      │ │            │
│              │  └───────────────────────────┘ │            │
│              │                                 │            │
│              │  ┌───────────────────────────┐ │            │
│              │  │  🪙  Coinbase Wallet      │ │            │
│              │  │      Mobile wallet        │ │            │
│              │  └───────────────────────────┘ │            │
│              │                                 │            │
│              │  ─────────────────────────────  │            │
│              │                                 │            │
│              │      [Cancel]  [Continue →]    │            │
│              │                                 │            │
│              └─────────────────────────────────┘            │
│                                                              │
│  Modal Specs:                                                │
│  - Width: 480px                                             │
│  - Padding: 32px                                            │
│  - Radius: 16px                                             │
│  - Shadow: 0 20px 40px rgba(0,0,0,0.2)                     │
│  - Backdrop: rgba(0,0,0,0.5) blur(4px)                     │
└─────────────────────────────────────────────────────────────┘
```

---

## Layout System

### Dashboard Grid

```
┌─────────────────────────────────────────────────────────────┐
│  DESKTOP LAYOUT (1440px+)                                    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  HEADER (64px)                                       │   │
│  │  Logo | Nav | Network | Wallet | Profile            │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌────────────┬──────────────────────────────────────────┐ │
│  │            │                                          │ │
│  │  SIDEBAR   │           MAIN CONTENT                   │ │
│  │  (240px)   │           (flex-1)                       │ │
│  │            │                                          │ │
│  │  Dashboard │  ┌─────────────────────────────────┐    │ │
│  │  Portfolio │  │  Portfolio Summary Cards        │    │ │
│  │  Analytics │  │  [3 cards in grid]              │    │ │
│  │  Trans.    │  └─────────────────────────────────┘    │ │
│  │  Settings  │                                          │ │
│  │            │  ┌──────────────┐ ┌──────────────┐     │ │
│  │            │  │  Allocation  │ │  Performance │     │ │
│  │            │  │  Chart       │ │  Chart       │     │ │
│  │            │  └──────────────┘ └──────────────┘     │ │
│  │            │                                          │ │
│  │            │  ┌─────────────────────────────────┐    │ │
│  │            │  │  Asset Table                    │    │ │
│  │            │  └─────────────────────────────────┘    │ │
│  │            │                                          │ │
│  └────────────┴──────────────────────────────────────────┘ │
│                                                              │
│  Grid: 12-column, 24px gutter, 64px margins                 │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  MOBILE LAYOUT (<768px)                                      │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  HEADER (56px)                                       │   │
│  │  ☰ Logo                          👤 🔔              │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                                                      │   │
│  │  Portfolio Summary                                   │   │
│  │  ┌─────────────────────────────────────────────┐    │   │
│  │  │  $125,430.00                                │    │   │
│  │  │  +$3,240.00 (+2.34%) ↗                      │    │   │
│  │  └─────────────────────────────────────────────┘    │   │
│  │                                                      │   │
│  │  [Quick Actions: + Add | Swap | Send]               │   │
│  │                                                      │   │
│  │  Allocation                                          │   │
│  │  ┌─────────────────────────────────────────────┐    │   │
│  │  │  [Pie Chart - Simplified]                   │    │   │
│  │  └─────────────────────────────────────────────┘    │   │
│  │                                                      │   │
│  │  Assets                                              │   │
│  │  ┌─────────────────────────────────────────────┐    │   │
│  │  │  Asset Cards (vertical list)                │    │   │
│  │  └─────────────────────────────────────────────┘    │   │
│  │                                                      │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  BOTTOM NAV (64px)                                   │   │
│  │  🏠 Dashboard  │  📊 Portfolio  │  ⚙️ Settings     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  Touch targets: minimum 44px                                │
└─────────────────────────────────────────────────────────────┘
```

---

## Dark Mode

```
┌─────────────────────────────────────────────────────────────┐
│  DARK MODE COLOR MAPPING                                     │
│                                                              │
│  Element          Light Mode    →    Dark Mode              │
│  ─────────────────────────────────────────────────────      │
│  Background       #FFFFFF       →    #111827                │
│  Surface          #F9FAFB       →    #1F2937                │
│  Card             #FFFFFF       →    #1F2937                │
│  Border           #E5E7EB       →    #374151                │
│  Text Primary     #111827       →    #F9FAFB                │
│  Text Secondary   #374151       →    #D1D5DB                │
│  Text Muted       #6B7280       →    #9CA3AF                │
│  Primary          #1A73E8       →    #3B82F6 (brighter)     │
│  Success          #10B981       →    #34D399 (brighter)     │
│  Danger           #EF4444       →    #F87171 (brighter)     │
│                                                              │
│  Implementation:                                             │
│  - CSS custom properties (--bg, --text, --primary)         │
│  - System preference detection (prefers-color-scheme)      │
│  - Manual toggle in settings                               │
│  - Persist preference in localStorage                      │
└─────────────────────────────────────────────────────────────┘
```

---

## Iconography

### Icon System

```yaml
Library: Lucide Icons (MIT, 1000+ icons)
Style: Outline, 2px stroke, rounded caps
Sizes: 16px, 20px, 24px, 32px
Colors: Inherit from text color

Common Icons:
  - Wallet: wallet
  - Portfolio: pie-chart
  - Analytics: bar-chart-3
  - Transactions: arrow-left-right
  - Settings: settings
  - Profile: user
  - Logout: log-out
  - Add: plus
  - Edit: pencil
  - Delete: trash-2
  - Search: search
  - Filter: filter
  - Download: download
  - Upload: upload
  - Refresh: refresh-cw
  - Check: check
  - X: x
  - Alert: alert-circle
  - Info: info
```

---

## Motion & Animation

### Principles

```yaml
Duration:
  - Fast: 150ms (hover, focus)
  - Normal: 300ms (transitions, modals)
  - Slow: 500ms (page transitions)

Easing:
  - Ease-out: cubic-bezier(0.33, 1, 0.68, 1) - Enter animations
  - Ease-in-out: cubic-bezier(0.65, 0, 0.35, 1) - Smooth transitions
  - Linear: constant - Loading spinners

Types:
  - Fade in/out: opacity 0 ↔ 1
  - Slide up/down: transform translateY
  - Scale: transform scale 0.95 ↔ 1
  - Pulse: for loading states
```

### Micro-interactions

```
Button Hover:
  - Duration: 150ms
  - Effect: Background darken + slight scale (1.02)

Card Hover:
  - Duration: 200ms
  - Effect: Shadow increase + translateY(-2px)

Loading:
  - Skeleton screens for content
  - Spinner for actions
  - Progress bar for uploads

Success:
  - Checkmark animation
  - Green flash
  - Toast notification slide-in
```

---

## Accessibility

### WCAG 2.1 AA Compliance

```yaml
Color Contrast:
  - Normal text: ≥ 4.5:1
  - Large text: ≥ 3:1
  - UI components: ≥ 3:1

Keyboard Navigation:
  - All interactive elements focusable
  - Visible focus indicators
  - Logical tab order
  - Escape closes modals

Screen Readers:
  - Semantic HTML
  - ARIA labels for icons
  - Alt text for images
  - Live regions for dynamic content

Reduced Motion:
  - Respect prefers-reduced-motion
  - Disable non-essential animations
  - Instant transitions
```

---

## Design Tokens

### JSON Format (for Figma → Code sync)

```json
{
  "colors": {
    "primary": {
      "DEFAULT": "#1A73E8",
      "dark": "#1557B0",
      "darker": "#0D3B7A",
      "light": "#E8F0FE"
    },
    "success": "#10B981",
    "danger": "#EF4444",
    "warning": "#F59E0B",
    "background": {
      "light": "#FFFFFF",
      "dark": "#111827"
    }
  },
  "spacing": {
    "xs": "4px",
    "sm": "8px",
    "md": "16px",
    "lg": "24px",
    "xl": "32px",
    "2xl": "48px"
  },
  "typography": {
    "fontFamily": {
      "sans": "Inter",
      "mono": "JetBrains Mono"
    },
    "fontSize": {
      "xs": "12px",
      "sm": "14px",
      "base": "16px",
      "lg": "18px",
      "xl": "24px",
      "2xl": "32px"
    }
  },
  "borderRadius": {
    "sm": "4px",
    "md": "8px",
    "lg": "12px",
    "xl": "16px",
    "full": "9999px"
  }
}
```

---

## Design Handoff

### Figma Organization

```
📁 Crypto Pocket Butler
├── 📄 Cover
├── 📄 Foundations
│   ├── Colors
│   ├── Typography
│   ├── Spacing
│   ├── Icons
│   └── Shadows
├── 📄 Components
│   ├── Buttons
│   ├── Inputs
│   ├── Cards
│   ├── Tables
│   ├── Charts
│   └── Modals
├── 📄 Patterns
│   ├── Wallet Connect Flow
│   ├── Portfolio Dashboard
│   ├── Asset Detail
│   └── Settings
└── 📄 Screens
    ├── Desktop
    │   ├── Dashboard
    │   ├── Portfolio
    │   └── Analytics
    └── Mobile
        ├── Dashboard
        ├── Portfolio
        └── Settings
```

### Developer Handoff Checklist

- [ ] Design tokens exported (JSON)
- [ ] Component specs documented
- [ ] All states designed (hover, active, disabled, error)
- [ ] Responsive breakpoints defined
- [ ] Dark mode variants complete
- [ ] Accessibility audit passed
- [ ] Assets exported (SVG, PNG)
- [ ] Figma Dev Mode links shared
- [ ] Handoff meeting scheduled
