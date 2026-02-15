# Portfolio Settings Implementation

## Overview
This implementation adds UI and backend support for portfolio targets (target allocation) and guardrails (risk management parameters).

## Backend Changes

### Database Migration
**File**: `api/migration/src/m20240101_000008_add_settings_to_portfolios.rs`

Added two JSONB columns to the `portfolios` table:
- `target_allocation`: Stores target percentages for each asset (e.g., `{"BTC": 40, "ETH": 30, "USDT": 30}`)
- `guardrails`: Stores risk management parameters (e.g., `{"drift_band": 5, "stablecoin_min": 10, "futures_cap": 20, "max_alt_cap": 50}`)

Schema:
```sql
ALTER TABLE portfolios ADD COLUMN target_allocation JSONB NULL;
ALTER TABLE portfolios ADD COLUMN guardrails JSONB NULL;
```

### Entity Model
**File**: `api/src/entities/portfolios.rs`

Updated the Portfolio entity to include:
```rust
pub struct Model {
    // ... existing fields
    pub target_allocation: Option<serde_json::Value>,
    pub guardrails: Option<serde_json::Value>,
}
```

### API DTOs
**File**: `api/src/handlers/portfolios.rs`

Updated request/response DTOs:

1. **CreatePortfolioRequest** - Added optional fields:
   - `target_allocation`: JSON object with asset:percentage pairs
   - `guardrails`: JSON object with risk parameters

2. **UpdatePortfolioRequest** - Added optional fields:
   - `target_allocation`: JSON object with asset:percentage pairs
   - `guardrails`: JSON object with risk parameters

3. **PortfolioResponse** - Added fields to response:
   - `target_allocation`: Returns stored target allocation
   - `guardrails`: Returns stored guardrails

### API Handlers
Updated handlers to persist the new fields:
- `create_portfolio`: Saves target_allocation and guardrails on creation
- `update_portfolio`: Updates target_allocation and guardrails on edit

## Frontend Changes

### New Settings Page
**File**: `web/app/portfolios/[id]/settings/page.tsx`

Server-side page component that:
- Checks authentication
- Renders the settings UI with cyberpunk theme
- Provides navigation breadcrumbs

### Settings Client Component
**File**: `web/app/portfolios/[id]/settings/components/SettingsClient.tsx`

Main interactive component with:

#### Target Allocation Section
- Dynamic form with add/remove asset rows
- Asset name input (auto-uppercase)
- Percentage input (0-100)
- Real-time total calculation display
- Color-coded total (green when = 100%, red otherwise)

#### Guardrails Section
Four configurable parameters:
1. **Drift Band** - Maximum allowed deviation from target allocation (%)
2. **Stablecoin Minimum** - Minimum percentage of stablecoins (%)
3. **Futures Cap** - Maximum percentage allocated to futures (%)
4. **Max Altcoin Cap** - Maximum percentage allocated to altcoins (%)

#### Validation Logic
```typescript
validateForm(): string | null {
  // Check for empty assets
  if (hasEmptyAssets) return "All asset names must be filled in";
  
  // Check for duplicate assets
  if (hasDuplicates) return "Duplicate asset names are not allowed";
  
  // Validate total percentage = 100%
  if (totalPercentage !== 100) return "Target allocation must sum to 100%";
  
  // Check for negative percentages
  if (hasNegative) return "Target percentages cannot be negative";
  
  // Validate guardrail ranges (0-100%)
  if (outOfRange) return "Guardrails must be between 0 and 100";
}
```

### Navigation Update
**File**: `web/app/portfolios/[id]/components/PortfolioDetailClient.tsx`

Added "Settings" button to Quick Actions section:
- Links to `/portfolios/[id]/settings`
- Uses purple gradient styling
- Settings gear icon

## API Endpoints

### Create Portfolio with Settings
```http
POST /v1/portfolios
Content-Type: application/json

{
  "name": "My Portfolio",
  "description": "Description",
  "is_default": false,
  "target_allocation": {
    "BTC": 40,
    "ETH": 30,
    "USDT": 30
  },
  "guardrails": {
    "drift_band": 5,
    "stablecoin_min": 10,
    "futures_cap": 20,
    "max_alt_cap": 50
  }
}
```

### Update Portfolio Settings
```http
PUT /v1/portfolios/{id}
Content-Type: application/json

{
  "target_allocation": {
    "BTC": 50,
    "ETH": 30,
    "USDT": 20
  },
  "guardrails": {
    "drift_band": 3,
    "stablecoin_min": 15,
    "futures_cap": 25,
    "max_alt_cap": 40
  }
}
```

### Get Portfolio (includes settings)
```http
GET /v1/portfolios/{id}

Response:
{
  "id": "uuid",
  "name": "My Portfolio",
  "target_allocation": {
    "BTC": 40,
    "ETH": 30,
    "USDT": 30
  },
  "guardrails": {
    "drift_band": 5,
    "stablecoin_min": 10,
    "futures_cap": 20,
    "max_alt_cap": 50
  }
}
```

## UI Features

### Target Allocation
- ✅ Dynamic add/remove asset rows
- ✅ Real-time total percentage calculation
- ✅ Visual feedback (green/red) for valid/invalid totals
- ✅ Validation: total must equal 100%
- ✅ Validation: no empty asset names
- ✅ Validation: no duplicate assets
- ✅ Validation: no negative percentages

### Guardrails
- ✅ Four configurable parameters with descriptions
- ✅ Range validation (0-100%)
- ✅ Responsive grid layout
- ✅ Helpful explanatory text for each field

### User Experience
- ✅ Error messages displayed clearly
- ✅ Success confirmation on save
- ✅ Loading states during save
- ✅ Breadcrumb navigation
- ✅ Consistent cyberpunk theme
- ✅ Back to Portfolio button

## Design Decisions

### Storage Format: JSONB in portfolios table
**Chosen over separate portfolio_settings table because:**
1. Settings are tightly coupled to portfolio (1:1 relationship)
2. Simpler queries - no joins needed
3. JSONB allows flexible schema evolution
4. Postgres JSONB provides indexing and querying capabilities
5. Reduces database complexity

### Validation Approach: Client + Server
- Frontend validation provides immediate feedback
- Backend can add additional validation in handlers if needed
- Validation is simple (sums, ranges) and doesn't require complex logic

### UI Design
- Follows existing cyberpunk theme (fuchsia/cyan/purple gradients)
- Matches patterns from PortfolioCompositionEditor
- Responsive layout with mobile support
- Clear visual hierarchy (sections, colors, spacing)

## Testing

### Backend Build
```bash
cd api
cargo build
# Success - compiles with new fields
```

### Frontend Build
```bash
cd web
npm install
npm run build
# Success - TypeScript types check, build succeeds
# Route /portfolios/[id]/settings included in build
```

### Database Migration
```bash
cd api/migration
cargo run -- up
# Migration m20240101_000008_add_settings_to_portfolios applied successfully
```

### Schema Verification
```sql
\d portfolios
# Confirmed: target_allocation JSONB, guardrails JSONB columns exist
```

## Future Enhancements

Potential improvements (not in current scope):
1. Asset autocomplete from existing holdings
2. Visual pie chart of target allocation
3. Drift calculation and visualization
4. Historical settings tracking
5. Templates for common allocation strategies
6. Asset category detection (stable/alt/futures)
7. Validation against actual holdings
8. Warning when settings conflict with holdings

## Summary

This implementation provides a complete solution for portfolio settings:
- ✅ Backend storage with JSONB columns
- ✅ API support for create/update/read
- ✅ Full UI with form and validation
- ✅ Navigation integration
- ✅ Consistent design system
- ✅ Type safety (TypeScript + Rust)
- ✅ All builds passing
- ✅ Database migration successful

The settings form validates that:
1. Target allocations sum to exactly 100%
2. No duplicate or empty asset names
3. All percentages are non-negative
4. Guardrails are within valid ranges (0-100%)
