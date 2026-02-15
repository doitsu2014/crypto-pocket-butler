# Domain Models - JSON Schema Documentation

This document describes the JSON schemas for holdings, allocations, and snapshots in the crypto-pocket-butler backend.

## Table of Contents
1. [Account Holdings](#account-holdings)
2. [Portfolio Allocations](#portfolio-allocations)
3. [Portfolio Snapshots](#portfolio-snapshots)

---

## Account Holdings

Account holdings represent raw quantity data from crypto accounts before price enrichment.

### Schema

```json
{
  "asset": "BTC",
  "quantity": "1.5",
  "available": "1.5",
  "frozen": "0",
  "price_usd": 50000.0,  // Optional: usually not present
  "value_usd": 75000.0   // Optional: usually not present
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `asset` | String | Yes | Asset symbol (e.g., "BTC", "ETH") |
| `quantity` | String | Yes | Total quantity as decimal string |
| `available` | String | Yes | Available (unfrozen) quantity as decimal string |
| `frozen` | String | Yes | Frozen (locked) quantity as decimal string |
| `price_usd` | Number | No | Optional price from account data (usually absent) |
| `value_usd` | Number | No | Optional value from account data (usually absent) |

### Storage Location

- **Database Table**: `accounts`
- **Column**: `holdings` (JSONB array)

### Example (Multiple Holdings)

```json
[
  {
    "asset": "BTC",
    "quantity": "1.5",
    "available": "1.5",
    "frozen": "0"
  },
  {
    "asset": "ETH",
    "quantity": "10.25",
    "available": "8.0",
    "frozen": "2.25"
  },
  {
    "asset": "USDT",
    "quantity": "5000.0",
    "available": "5000.0",
    "frozen": "0"
  }
]
```

---

## Portfolio Allocations

Portfolio allocations represent aggregated holdings with price enrichment, value calculations, and portfolio weights.

### Schema

```json
{
  "asset": "BTC",
  "quantity": "1.5",
  "price_usd": 50000.0,
  "value_usd": 75000.0,
  "weight": 45.5,
  "unpriced": false
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `asset` | String | Yes | Asset symbol |
| `quantity` | String | Yes | Total quantity across all accounts (decimal string) |
| `price_usd` | Number | No | Current price per unit in USD (null if unpriced) |
| `value_usd` | Number | Yes | Total value in USD (quantity × price) |
| `weight` | Number | Yes | Percentage of total portfolio value (0-100) |
| `unpriced` | Boolean | No | Flag indicating if asset has no price data (default: false) |

### Storage Location

- **Database Table**: `portfolio_allocations`
- **Column**: `holdings` (JSONB array)

### Example (Complete Allocation)

```json
[
  {
    "asset": "BTC",
    "quantity": "1.5",
    "price_usd": 50000.0,
    "value_usd": 75000.0,
    "weight": 60.0,
    "unpriced": false
  },
  {
    "asset": "ETH",
    "quantity": "20.5",
    "price_usd": 2500.0,
    "value_usd": 51250.0,
    "weight": 40.0,
    "unpriced": false
  },
  {
    "asset": "UNKNOWN_TOKEN",
    "quantity": "1000.0",
    "value_usd": 0.0,
    "weight": 0.0,
    "unpriced": true
  }
]
```

### Weight Calculation

- Weights are computed only for **priced assets**
- Formula: `weight = (value_usd / total_portfolio_value) × 100`
- Unpriced assets have `weight = 0.0`
- Weights of priced assets sum to 100%

---

## Portfolio Snapshots

Portfolio snapshots are immutable point-in-time records of portfolio allocations.

### Holdings Schema

Snapshot holdings use the same structure as allocation items:

```json
{
  "asset": "BTC",
  "quantity": "1.5",
  "price_usd": 50000.0,
  "value_usd": 75000.0,
  "weight": 45.5,
  "unpriced": false
}
```

### Metadata Schema

```json
{
  "portfolio_name": "My Portfolio",
  "allocation_as_of": "2024-01-01T12:00:00Z",
  "snapshot_time": "2024-01-01T16:00:00Z",
  "created_at": "2024-01-01T16:00:00Z"
}
```

### Metadata Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `portfolio_name` | String | Yes | Name of the portfolio at snapshot time |
| `allocation_as_of` | String | Yes | ISO 8601 timestamp when underlying allocation was computed |
| `snapshot_time` | String | Yes | ISO 8601 timestamp when snapshot was taken |
| `created_at` | String | Yes | ISO 8601 timestamp when snapshot record was created |

### Storage Location

- **Database Table**: `snapshots`
- **Columns**: 
  - `holdings` (JSONB array) - Holdings data
  - `metadata` (JSONB object) - Metadata

### Snapshot Types

- `"eod"` - End-of-day snapshot (automated, daily)
- `"manual"` - User-triggered snapshot
- `"hourly"` - Hourly snapshot (if configured)

### Example (Complete Snapshot Record)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "portfolio_id": "123e4567-e89b-12d3-a456-426614174000",
  "snapshot_date": "2024-01-01",
  "snapshot_type": "eod",
  "total_value_usd": "125000.00",
  "holdings": [
    {
      "asset": "BTC",
      "quantity": "1.5",
      "price_usd": 50000.0,
      "value_usd": 75000.0,
      "weight": 60.0,
      "unpriced": false
    },
    {
      "asset": "ETH",
      "quantity": "20.0",
      "price_usd": 2500.0,
      "value_usd": 50000.0,
      "weight": 40.0,
      "unpriced": false
    }
  ],
  "metadata": {
    "portfolio_name": "My Crypto Portfolio",
    "allocation_as_of": "2024-01-01T12:00:00Z",
    "snapshot_time": "2024-01-01T23:00:00Z",
    "created_at": "2024-01-01T23:00:05Z"
  },
  "allocation_id": "789e0123-e45b-67c8-d901-234567890123",
  "created_at": "2024-01-01T23:00:05Z"
}
```

---

## Implementation Notes

### Type Safety

All JSON structures have corresponding Rust structs in `api/src/domain/`:

- `AccountHolding` - for account holdings
- `AllocationItem` - for allocation holdings
- `AllocationData` - complete allocation with metadata
- `SnapshotHolding` - for snapshot holdings
- `SnapshotMetadata` - for snapshot metadata
- `SnapshotData` - complete snapshot with metadata

### Database Boundary

- **Serialization**: Typed structs are serialized to JSON only when writing to the database
- **Deserialization**: JSON is deserialized to typed structs immediately when reading from the database
- **Internal Logic**: All business logic operates on typed structs, not loose JSON

### Validation

- Asset symbols are normalized to uppercase
- Decimal values use strings for precision (e.g., quantities)
- Float values (f64) are used for prices and computed values
- Empty asset names are rejected during processing
