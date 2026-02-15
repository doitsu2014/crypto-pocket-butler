# Portfolio UI Components

This directory contains reusable React components for displaying portfolio data in the Crypto Pocket Butler application.

## Components

### 1. HoldingsTable

A sortable table component that displays asset holdings with detailed information.

**Props:**
- `holdings: AssetHolding[]` - Array of asset holdings to display
- `allocation: AllocationItem[]` - Array of allocation percentages for each asset

**Features:**
- Sortable by asset name, quantity, or value
- Displays price, quantity, and total value for each holding
- Shows allocation percentage
- Highlights top 3 holdings with badges
- Responsive table with horizontal scrolling on mobile
- Cyberpunk-themed styling with cyan/fuchsia gradients

**Example:**
```tsx
import { HoldingsTable } from '@/components/portfolio';

<HoldingsTable 
  holdings={portfolioData.holdings} 
  allocation={portfolioData.allocation} 
/>
```

---

### 2. AllocationBar

A horizontal bar chart component showing portfolio allocation as stacked bars.

**Props:**
- `allocation: AllocationItem[]` - Array of allocation items
- `maxItems?: number` - Maximum number of items to display (default: 10)

**Features:**
- Shows percentage and USD value for each asset
- Animated gradient progress bars
- Responsive layout
- Violet/purple color scheme

**Example:**
```tsx
import { AllocationBar } from '@/components/portfolio';

<AllocationBar 
  allocation={portfolioData.allocation} 
  maxItems={10} 
/>
```

---

### 3. AllocationPie

A pie chart component visualizing portfolio allocation distribution.

**Props:**
- `allocation: AllocationItem[]` - Array of allocation items
- `maxItems?: number` - Maximum number of items to display (default: 10)

**Features:**
- Interactive pie chart using Recharts library
- Custom tooltip showing asset name, value, and percentage
- Legend with color-coded asset names
- Cyberpunk color palette
- Responsive container

**Example:**
```tsx
import { AllocationPie } from '@/components/portfolio';

<AllocationPie 
  allocation={portfolioData.allocation} 
  maxItems={10} 
/>
```

---

### 4. DriftBadge

A badge component showing how much an asset's current allocation differs from its target allocation.

**Props:**
- `currentPercentage: number` - Current allocation percentage
- `targetPercentage: number` - Target allocation percentage
- `asset: string` - Asset name (for identification)

**Features:**
- Color-coded severity levels:
  - **Green** (<5% drift): Allocation is on target
  - **Yellow** (5-10% drift): Minor drift detected
  - **Red** (>10% drift): Significant drift from target
- Shows drift amount with +/- sign
- Displays target percentage
- Compact design suitable for inline use

**Example:**
```tsx
import { DriftBadge } from '@/components/portfolio';

<DriftBadge 
  currentPercentage={45.5} 
  targetPercentage={40.0} 
  asset="BTC" 
/>
```

---

## Data Types

### AssetHolding
```typescript
interface AssetHolding {
  asset: string;                    // Asset symbol (e.g., "BTC", "ETH")
  total_quantity: string;           // Total quantity held
  total_available: string;          // Available quantity
  total_frozen: string;             // Frozen/locked quantity
  price_usd: number;                // Current price in USD
  value_usd: number;                // Total value in USD
  accounts: AccountHoldingDetail[]; // Account breakdown
}
```

### AllocationItem
```typescript
interface AllocationItem {
  asset: string;        // Asset symbol
  value_usd: number;    // Value in USD
  percentage: number;   // Allocation percentage (0-100)
}
```

### AccountHoldingDetail
```typescript
interface AccountHoldingDetail {
  account_id: string;   // Account identifier
  account_name: string; // Account display name
  quantity: string;     // Quantity in this account
  available: string;    // Available quantity
  frozen: string;       // Frozen quantity
}
```

---

## Styling

All components use the application's cyberpunk theme with:
- **Primary colors:** Fuchsia (#e879f9), Violet (#a78bfa), Cyan (#22d3ee)
- **Backgrounds:** Slate with opacity and backdrop blur
- **Effects:** Glow shadows, gradients, transitions
- **Typography:** Gradient text for headings, monospace for numbers

Components are fully responsive and work on mobile, tablet, and desktop screens.

---

## Usage in Portfolio Detail Page

The portfolio detail page at `/portfolios/[id]` uses all four components:

```tsx
import { 
  HoldingsTable, 
  AllocationBar, 
  AllocationPie, 
  DriftBadge 
} from '@/components/portfolio';

// In component:
<AllocationBar allocation={holdings.allocation} />
<AllocationPie allocation={holdings.allocation} />
<HoldingsTable holdings={holdings.holdings} allocation={holdings.allocation} />

{/* Drift indicators section */}
{portfolio.target_allocation && (
  <div>
    {holdings.allocation.map((item) => {
      const targetPercentage = portfolio.target_allocation?.[item.asset];
      if (!targetPercentage) return null;
      
      return (
        <DriftBadge
          key={item.asset}
          currentPercentage={item.percentage}
          targetPercentage={targetPercentage}
          asset={item.asset}
        />
      );
    })}
  </div>
)}
```

---

## Dependencies

- **React 19.2.3** - UI framework
- **Recharts 3.7.0** - Charting library (used by AllocationPie)
- **Tailwind CSS 4** - Styling framework

---

## Testing

To test these components:

1. **Build the project:**
   ```bash
   cd web
   npm run build
   ```

2. **Run the dev server:**
   ```bash
   npm run dev
   ```

3. **Navigate to a portfolio detail page:**
   - Go to `/portfolios/[id]` where `[id]` is a valid portfolio ID
   - The page will display all components with live data

4. **Test drift indicators:**
   - Configure target allocation in portfolio settings
   - Return to portfolio detail page to see drift badges

---

## Future Enhancements

Potential improvements:
- Add expandable rows in HoldingsTable to show per-account breakdown
- Support custom color schemes for charts
- Add export functionality (CSV, PDF)
- Create comparison view for multiple portfolios
- Add historical drift tracking
- Support for custom drift thresholds
