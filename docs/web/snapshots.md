# Portfolio Snapshots Chart

This directory contains the implementation of the Portfolio Snapshots feature, which displays a time series chart of portfolio value over time.

## Overview

The Portfolio Snapshots page provides a visual representation of how a portfolio's value changes over time through an interactive line chart with customizable date ranges and filters.

## Features Implemented

### ✅ Charting Library
- **Recharts v3.7.0** integrated and fully functional
- Used components:
  - `LineChart` - Main chart container
  - `Line` - Portfolio value line
  - `XAxis` - Date axis
  - `YAxis` - Value axis (in USD)
  - `CartesianGrid` - Background grid
  - `Tooltip` - Interactive tooltip on hover
  - `ResponsiveContainer` - Responsive chart sizing

### ✅ Portfolio Value Time Series
- Displays portfolio value in USD over time
- Line chart with smooth transitions
- Data points clearly marked with dots
- Sorted chronologically for accurate trend visualization
- Active dot highlighting on hover

### ✅ Interactive Tooltip
- Shows on hover over data points
- Displays:
  - Formatted date (e.g., "Jan 15, 2024")
  - Portfolio value in USD currency format (e.g., "$12,345.67")
- Custom styling matching the application's cyberpunk theme
- Positioned dynamically to avoid overflow

### ✅ Date Range Selector
Four preset date ranges available via dropdown:
- **Last 7 days** - Shows snapshots from the past week
- **Last 30 days** (default) - Shows snapshots from the past month
- **Last 90 days** - Shows snapshots from the past quarter
- **All time** - Shows all available snapshots

Date range is passed to the backend API to filter results efficiently.

### ✅ Additional Features
- **Snapshot Type Filter** - Filter by snapshot type:
  - All types
  - EOD (End of Day)
  - Manual
  - Hourly
- **Refresh Button** - Reload snapshots on demand
- **Loading States** - Spinner during data fetch
- **Error Handling** - User-friendly error messages with retry option
- **Empty State** - Informative message when no snapshots exist
- **Snapshot History Table** - Detailed table below chart showing:
  - Date
  - Type (with color-coded badges)
  - Total Value
  - Created At timestamp

## File Structure

```
snapshots/
├── README.md                    # This file
├── page.tsx                     # Server component with auth check
└── components/
    └── SnapshotsClient.tsx      # Main client component with chart logic
```

## Component Architecture

### page.tsx
- Server component that handles authentication
- Redirects to sign-in if not authenticated
- Renders the layout with navigation
- Passes portfolio ID to SnapshotsClient

### SnapshotsClient.tsx
The main component containing all chart functionality:

**State Management:**
- `snapshots` - Array of snapshot data
- `loading` - Loading state flag
- `error` - Error message string
- `dateRange` - Selected date range ("7" | "30" | "90" | "all")
- `selectedType` - Selected snapshot type filter

**Key Functions:**
- `fetchSnapshots()` - Fetches snapshots from API with filters
- `formatCurrency()` - Formats numbers as USD currency
- `formatDate()` - Formats dates for display
- `formatDateTime()` - Formats dates with time

**Chart Data Transformation:**
```typescript
interface ChartDataPoint {
  date: string;              // ISO date string
  value: number;             // Numeric value in USD
  formattedValue: string;    // Pre-formatted currency string
}
```

## API Integration

**Endpoint:** `GET /v1/portfolios/:id/snapshots`

**Query Parameters:**
- `start_date` - Filter start date (YYYY-MM-DD)
- `end_date` - Filter end date (YYYY-MM-DD)
- `snapshot_type` - Filter by type (eod, manual, hourly)

**Response:**
```typescript
interface ListSnapshotsResponse {
  portfolio_id: string;
  snapshots: Snapshot[];
  total_count: number;
}

interface Snapshot {
  id: string;
  portfolio_id: string;
  snapshot_date: string;
  snapshot_type: string;
  total_value_usd: string;
  holdings: any;
  metadata?: any;
  created_at: string;
}
```

## Styling

The chart uses the application's cyberpunk theme:

**Colors:**
- **Line:** Fuchsia (#d946ef)
- **Grid:** Violet with transparency (rgba(139, 92, 246, 0.2))
- **Axis Labels:** Cyan (#67e8f9)
- **Tooltip Background:** Dark slate with fuchsia border
- **Glow Effects:** Applied to interactive elements

**Layout:**
- Chart height: 400px
- Fully responsive width
- Card-style container with glass morphism effect
- Rounded corners and shadow effects

## Usage Example

```tsx
import SnapshotsClient from './components/SnapshotsClient';

export default function SnapshotsPage() {
  return <SnapshotsClient portfolioId="abc123" />;
}
```

## Testing

### Build Verification
```bash
cd web
npm run build
```
Expected: ✅ Build succeeds with no errors

### Manual Testing
1. Start the development server:
   ```bash
   npm run dev
   ```

2. Navigate to: `/portfolios/[id]/snapshots`

3. Verify:
   - Chart renders with data
   - Date range selector changes visible data
   - Tooltip shows on hover
   - Table displays snapshot details
   - Filters work correctly
   - Refresh button reloads data
   - Loading states appear during fetch
   - Error states display when API fails

## Dependencies

- **Recharts 3.7.0** - Chart visualization library
- **React 19.2.3** - UI framework
- **Next.js 16.1.6** - Framework
- **Tailwind CSS 4** - Styling

## Performance Considerations

- Chart data is sorted once after fetch, not on every render
- ResponsiveContainer handles resize efficiently
- Date formatting functions are defined outside component to avoid recreation
- `useCallback` hook used for `fetchSnapshots` to prevent unnecessary re-renders
- API requests include date range filtering to minimize data transfer

## Future Enhancements

Potential improvements:
- [ ] Add zoom/pan functionality for large date ranges
- [ ] Export chart as image (PNG/SVG)
- [ ] Compare multiple portfolios on same chart
- [ ] Add custom date range picker (calendar)
- [ ] Show trend indicators (% change, moving averages)
- [ ] Add annotation support for marking important events
- [ ] Support for different chart types (area, bar, candlestick)
- [ ] Add data granularity selector (daily, weekly, monthly aggregation)

## Related Documentation

- [Portfolio Components](portfolio-components.md)
- [API Documentation](../api/api-overview.md)
- [UI Standardization](UI-STANDARDIZATION.md)

## Notes

- The chart implementation follows Recharts best practices
- Custom components (tooltip, legend) are defined outside render to avoid React warnings
- All currency values are formatted consistently throughout the application
- The feature integrates seamlessly with the existing portfolio management system
