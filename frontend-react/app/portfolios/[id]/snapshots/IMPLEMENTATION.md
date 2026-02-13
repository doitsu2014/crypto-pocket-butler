# Portfolio Snapshots Chart - Visual Implementation Guide

## Feature Overview

The Portfolio Snapshots Chart provides a visual time series representation of portfolio value changes over time.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Portfolio Snapshots                              │
│                  Historical portfolio value over time                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Filters:  [Last 30 days ▼]  [All types ▼]  [Refresh]                │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │ Portfolio Value Over Time                                         │ │
│  │                                                                   │ │
│  │  $100k ┤                                          ●               │ │
│  │        │                                     ●──●/                │ │
│  │   $90k ┤                             ●──●──/                     │ │
│  │        │                      ●──●──/                            │ │
│  │   $80k ┤               ●──●──/                                   │ │
│  │        │        ●──●──/                                          │ │
│  │   $70k ┤  ●──●──/                                                │ │
│  │        └─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────   │ │
│  │          Jan 1  Jan 5  Jan 10 Jan 15 Jan 20 Jan 25 Jan 30       │ │
│  │                                                                   │ │
│  │  [HOVER TOOLTIP]                                                 │ │
│  │  ┌──────────────────┐                                            │ │
│  │  │ Date: Jan 25, 2024│                                           │ │
│  │  │ Value: $95,432.21│                                            │ │
│  │  └──────────────────┘                                            │ │
│  └───────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │ Snapshot History (30)                                            │ │
│  ├─────────┬──────────┬───────────────┬────────────────────────────┤ │
│  │ Date    │ Type     │ Total Value   │ Created At                 │ │
│  ├─────────┼──────────┼───────────────┼────────────────────────────┤ │
│  │ Jan 30  │ [EOD]    │ $95,432.21    │ Jan 30, 2024 5:00 PM      │ │
│  │ Jan 29  │ [EOD]    │ $94,123.45    │ Jan 29, 2024 5:00 PM      │ │
│  │ Jan 28  │ [MANUAL] │ $93,876.54    │ Jan 28, 2024 2:30 PM      │ │
│  │ Jan 27  │ [EOD]    │ $92,345.67    │ Jan 27, 2024 5:00 PM      │ │
│  │ ...     │ ...      │ ...           │ ...                        │ │
│  └─────────┴──────────┴───────────────┴────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

## Component Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         page.tsx                                 │
│                    (Server Component)                            │
│                                                                  │
│  • Handles authentication                                       │
│  • Redirects if not authenticated                               │
│  • Renders navigation layout                                    │
│  • Passes portfolio ID to client                                │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              SnapshotsClient.tsx                           │ │
│  │              (Client Component)                            │ │
│  │                                                            │ │
│  │  State Management:                                         │ │
│  │  • snapshots: Snapshot[]                                   │ │
│  │  • loading: boolean                                        │ │
│  │  • error: string | null                                    │ │
│  │  • dateRange: "7" | "30" | "90" | "all"                   │ │
│  │  • selectedType: string                                    │ │
│  │                                                            │ │
│  │  ┌──────────────────────────────────────────────────────┐ │ │
│  │  │           fetchSnapshots()                           │ │ │
│  │  │                                                      │ │ │
│  │  │  1. Build query parameters                          │ │ │
│  │  │  2. Call API with filters                           │ │ │
│  │  │  3. Update state with results                       │ │ │
│  │  │  4. Handle errors                                   │ │ │
│  │  └──────────────────────────────────────────────────────┘ │ │
│  │                                                            │ │
│  │  ┌──────────────────────────────────────────────────────┐ │ │
│  │  │         Chart Data Transformation                    │ │ │
│  │  │                                                      │ │ │
│  │  │  snapshots                                           │ │ │
│  │  │    .map(snapshot => ({                              │ │ │
│  │  │      date: snapshot.snapshot_date,                  │ │ │
│  │  │      value: parseFloat(snapshot.total_value_usd),   │ │ │
│  │  │      formattedValue: formatCurrency(...)            │ │ │
│  │  │    }))                                              │ │ │
│  │  │    .sort((a, b) => date comparison)                 │ │ │
│  │  └──────────────────────────────────────────────────────┘ │ │
│  │                                                            │ │
│  │  Recharts Components:                                      │ │
│  │  • ResponsiveContainer (400px height, 100% width)         │ │
│  │  • LineChart (main chart container)                       │ │
│  │  • CartesianGrid (background grid)                        │ │
│  │  • XAxis (dates, formatted)                               │ │
│  │  • YAxis (USD values, abbreviated)                        │ │
│  │  • Tooltip (interactive hover info)                       │ │
│  │  • Line (portfolio value line with dots)                  │ │
│  └────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

## Data Flow

```
┌─────────┐      ┌────────────┐      ┌──────────┐      ┌─────────┐
│  User   │─────▶│  UI Filter │─────▶│   API    │─────▶│ Backend │
│ Action  │      │  Changes   │      │  Request │      │ Database│
└─────────┘      └────────────┘      └──────────┘      └─────────┘
                                           │
                        ┌──────────────────┘
                        │
                        ▼
                  ┌──────────┐      ┌──────────┐      ┌─────────┐
                  │   API    │─────▶│Transform │─────▶│ Render  │
                  │ Response │      │   Data   │      │  Chart  │
                  └──────────┘      └──────────┘      └─────────┘
```

### Example Data Flow: Changing Date Range

```
1. User selects "Last 7 days"
   ↓
2. setDateRange("7") updates state
   ↓
3. useEffect triggers fetchSnapshots()
   ↓
4. Calculate start/end dates:
   startDate = today - 7 days
   endDate = today
   ↓
5. API call: GET /v1/portfolios/123/snapshots?start_date=2024-01-23&end_date=2024-01-30
   ↓
6. Backend filters and returns snapshots
   ↓
7. Transform response:
   - Parse total_value_usd to number
   - Format date strings
   - Sort chronologically
   ↓
8. Update snapshots state
   ↓
9. React re-renders chart with new data
   ↓
10. Chart displays 7 days of data
```

## Key Features Breakdown

### 1. Date Range Selector

```typescript
// State
const [dateRange, setDateRange] = useState<"7" | "30" | "90" | "all">("30");

// Calculate date range
if (dateRange !== "all") {
  const endDate = new Date();
  const startDate = new Date();
  startDate.setDate(endDate.getDate() - parseInt(dateRange));
  params.append("start_date", startDate.toISOString().split("T")[0]);
  params.append("end_date", endDate.toISOString().split("T")[0]);
}

// UI
<select value={dateRange} onChange={(e) => setDateRange(e.target.value)}>
  <option value="7">Last 7 days</option>
  <option value="30">Last 30 days</option>
  <option value="90">Last 90 days</option>
  <option value="all">All time</option>
</select>
```

### 2. Interactive Tooltip

```typescript
<Tooltip
  contentStyle={{
    backgroundColor: "rgba(15, 23, 42, 0.95)",
    border: "2px solid rgba(217, 70, 239, 0.5)",
    borderRadius: "8px",
    boxShadow: "0 0 20px rgba(217, 70, 239, 0.3)",
  }}
  labelStyle={{ color: "#67e8f9" }}
  itemStyle={{ color: "#d946ef" }}
  formatter={(value: number) => [formatCurrency(value), "Value"]}
  labelFormatter={(label) => `Date: ${formatDate(label)}`}
/>
```

**Tooltip Display:**
```
┌──────────────────────┐
│ Date: Jan 25, 2024   │  ← Label (formatted date)
│ Value: $95,432.21    │  ← Formatted currency
└──────────────────────┘
```

### 3. Chart Configuration

```typescript
<LineChart data={chartData}>
  <CartesianGrid
    strokeDasharray="3 3"
    stroke="rgba(139, 92, 246, 0.2)"
  />
  <XAxis
    dataKey="date"
    stroke="#67e8f9"
    tick={{ fill: "#67e8f9" }}
    tickFormatter={formatDate}
  />
  <YAxis
    stroke="#67e8f9"
    tick={{ fill: "#67e8f9" }}
    tickFormatter={(value) => `$${(value / 1000).toFixed(0)}k`}
  />
  <Line
    type="monotone"
    dataKey="value"
    stroke="#d946ef"
    strokeWidth={3}
    dot={{ fill: "#d946ef", strokeWidth: 2, r: 4 }}
    activeDot={{ r: 6, fill: "#d946ef" }}
  />
</LineChart>
```

## Styling Theme

```
Color Palette (Cyberpunk):
├── Primary: Fuchsia (#d946ef, #e879f9)
├── Secondary: Violet (#8b5cf6, #a78bfa)
├── Accent: Cyan (#22d3ee, #67e8f9)
└── Background: Dark Slate (#0f172a, #1e293b)

Effects:
├── Glow Shadows: 0 0 20px rgba(217, 70, 239, 0.5)
├── Backdrop Blur: backdrop-blur-md
├── Border Glow: border-2 border-violet-500/30
└── Gradient Text: from-fuchsia-300 via-violet-300 to-cyan-300
```

## API Specification

### Request

```http
GET /v1/portfolios/{portfolio_id}/snapshots?start_date=2024-01-01&end_date=2024-01-31&snapshot_type=eod
```

### Response

```json
{
  "portfolio_id": "abc123",
  "total_count": 30,
  "snapshots": [
    {
      "id": "snap-001",
      "portfolio_id": "abc123",
      "snapshot_date": "2024-01-30",
      "snapshot_type": "eod",
      "total_value_usd": "95432.21",
      "holdings": { ... },
      "metadata": { ... },
      "created_at": "2024-01-30T17:00:00Z"
    },
    ...
  ]
}
```

## Performance Optimization

1. **Data Fetching**
   - Date range filtering on backend reduces data transfer
   - Type filtering reduces unnecessary snapshots
   - Pagination ready (not yet implemented)

2. **Chart Rendering**
   - ResponsiveContainer handles resize efficiently
   - Data sorted once, not on every render
   - Memoized formatting functions (defined outside component)

3. **State Management**
   - useCallback for fetchSnapshots prevents recreation
   - Minimal re-renders with proper dependency arrays
   - Loading states prevent jarring UX

## Browser Compatibility

✅ Modern browsers (Chrome, Firefox, Safari, Edge)
✅ Responsive design (mobile, tablet, desktop)
✅ SVG rendering (Recharts uses SVG)
⚠️ Requires JavaScript enabled

## Accessibility

- Keyboard navigable filters
- Screen reader friendly labels
- High contrast color scheme
- Hover states for interactive elements
- Error messages clearly communicated

## Related Features

```
Portfolio Management System
│
├── Portfolio Detail (/portfolios/[id])
│   ├── Holdings Table
│   ├── Allocation Pie Chart
│   ├── Allocation Bar Chart
│   └── Drift Badges
│
├── Snapshots (/portfolios/[id]/snapshots) ← THIS FEATURE
│   ├── Time Series Chart
│   └── Snapshot History Table
│
├── Settings (/portfolios/[id]/settings)
│   └── Target Allocation Configuration
│
└── Recommendations (/portfolios/[id]/recommendations)
    └── Rebalancing Suggestions
```

## Summary

The Portfolio Snapshots Chart is a **complete, production-ready feature** that:

✅ Uses Recharts v3.7.0 for visualization
✅ Displays portfolio value over time
✅ Provides interactive tooltips
✅ Includes date range selector
✅ Follows application styling conventions
✅ Integrates with existing API
✅ Handles loading and error states
✅ Works responsively across devices
✅ Requires no additional code changes

**Status:** READY FOR PRODUCTION ✅
