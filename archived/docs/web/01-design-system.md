# Design System

> Visual design system for Crypto Pocket Butler - Dark Neon Cyberpunk theme.

---

## Design Philosophy

**Theme**: Dark, Mysterious, Neon Cyberpunk

- **Core Concept**: High-tech cryptocurrency management with a secretive, futuristic aesthetic
- **Mood**: Professional yet mysterious, secure yet vibrant
- **Visual Style**: Intense neon glows, holographic effects, deep blacks with vibrant accent colors

---

## Color Palette

### Primary Colors

| Color | Shade | RGB | Usage |
|-------|-------|-----|-------|
| **Fuchsia** | 300 | `rgb(232 121 249)` | Text highlights |
| | 400 | `rgb(232 121 249)` | Gradient starts |
| | 500 | `rgb(217 70 239)` | Borders, buttons |
| | 600 | `rgb(192 38 211)` | Button backgrounds |
| | 950 | `rgb(74 4 78)` | Dark backgrounds |
| **Violet** | 300 | `rgb(196 181 253)` | Light text |
| | 400 | `rgb(167 139 250)` | Gradient mid |
| | 500 | `rgb(139 92 246)` | Borders, icons |
| | 600 | `rgb(124 58 237)` | Buttons |
| | 900 | `rgb(76 29 149)` | Backgrounds |
| | 950 | `rgb(46 16 101)` | Deep backgrounds |
| **Cyan** | 300 | `rgb(103 232 249)` | Light text |
| | 400 | `rgb(34 211 238)` | Highlights |
| | 500 | `rgb(6 182 212)` | Borders |
| | 600 | `rgb(8 145 178)` | Buttons |
| **Slate** | 900 | `rgb(15 23 42)` | Card backgrounds |
| | 950 | `rgb(2 6 23)` | Dark card backgrounds |
| | 200 | `rgb(226 232 240)` | Primary text |
| | 300 | `rgb(203 213 225)` | Secondary text |

### Usage Guidelines

- **Fuchsia** → Primary CTAs, important icons, main interactive elements
- **Violet** → Secondary elements, supporting UI components
- **Cyan** → Information displays, portfolio/data visualizations
- **Red** → Danger actions (sign out, delete), error states
- **Green** → Success states, verification indicators

---

## Typography

### Font Families

- **Sans-serif** (System default) → Primary UI font
- **Monospace** → IDs, addresses, numeric values

### Text Styles

#### Headings

```tsx
// Main Hero Heading
className="text-5xl sm:text-6xl md:text-7xl font-extrabold 
           bg-gradient-to-r from-fuchsia-400 via-purple-400 to-cyan-400 
           bg-clip-text text-transparent 
           drop-shadow-[0_0_30px_rgba(168,85,247,0.8)]"

// Section Heading
className="text-3xl font-extrabold 
           bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 
           bg-clip-text text-transparent 
           drop-shadow-[0_0_20px_rgba(232,121,249,0.6)]"

// Card Heading
className="text-lg font-bold text-fuchsia-300 
           drop-shadow-[0_0_10px_rgba(232,121,249,0.5)]"
```

#### Body Text

```tsx
// Primary Text
className="text-slate-200 drop-shadow-[0_0_10px_rgba(226,232,240,0.3)]"

// Secondary Text
className="text-slate-300"

// Small Text
className="text-sm text-slate-300"
```

---

## Components

### Buttons

#### Primary Button (Fuchsia)

```tsx
<button className="px-6 py-3 bg-gradient-to-r from-fuchsia-600 to-violet-600 
                   rounded-lg font-bold text-white 
                   border-2 border-fuchsia-500/50
                   hover:from-fuchsia-500 hover:to-violet-500
                   hover:shadow-[0_0_20px_rgba(217,70,239,0.5)]
                   transition-all duration-300">
  Create Portfolio
</button>
```

#### Secondary Button (Violet)

```tsx
<button className="px-6 py-3 bg-violet-950/50 
                   rounded-lg font-bold text-violet-300 
                   border-2 border-violet-500/50
                   hover:bg-violet-900/50
                   hover:shadow-[0_0_15px_rgba(139,92,246,0.4)]
                   transition-all duration-300">
  Cancel
</button>
```

#### Danger Button (Red)

```tsx
<button className="px-4 py-2 bg-red-950/50 
                   rounded-lg font-medium text-red-300 
                   border border-red-500/50
                   hover:bg-red-900/50
                   hover:shadow-[0_0_10px_rgba(239,68,68,0.4)]
                   transition-all duration-300">
  Delete
</button>
```

---

### Cards

```tsx
<div className="bg-slate-950/70 backdrop-blur-sm 
                border-2 border-violet-500/40 
                rounded-2xl p-6
                hover:border-fuchsia-500/60
                hover:shadow-[0_0_30px_rgba(139,92,246,0.3)]
                transition-all duration-300">
  {/* Card content */}
</div>
```

---

### Input Fields

```tsx
<input className="w-full px-4 py-3 
                  bg-slate-950/50 border-2 border-violet-500/40
                  rounded-lg text-slate-200 placeholder-slate-400
                  focus:border-fuchsia-500/60
                  focus:shadow-[0_0_15px_rgba(217,70,239,0.3)]
                  focus:outline-none
                  transition-all duration-300"
       placeholder="Enter portfolio name" />
```

---

## UI Patterns

### Toast Notifications

```
┌────────────────────────────────────────┐
│ ✓  Portfolio created successfully!  ✕ │
└────────────────────────────────────────┘
```

| Type | Background | Border | Icon |
|------|------------|--------|------|
| Success | Green-950/90 | Green-500/70 | ✓ Check circle |
| Error | Red-950/90 | Red-500/70 | ✕ X circle |
| Info | Cyan-950/90 | Cyan-500/70 | ℹ Info circle |
| Warning | Yellow-950/90 | Yellow-500/70 | ⚠ Warning triangle |

**Usage:**

```tsx
const toast = useToast();

toast.success("Portfolio created successfully!");
toast.error("Failed to load data");
toast.info("Processing your request...");
toast.warning("This action cannot be undone");
```

### Loading Skeletons

```
Card Skeleton:              List Skeleton:
┌─────────────────┐        ┌────────────────────────────────────────┐
│ ████████     ■  │        │ ████████                               │
│ ████            │        │ ████████████████████████████████       │
│ ███████         │        └────────────────────────────────────────┘
└─────────────────┘
```

**Usage:**

```tsx
<LoadingSkeleton count={3} type="card" />
<LoadingSkeleton count={5} type="list" />
<LoadingSkeleton count={4} type="table" />
```

### Empty States

```tsx
<EmptyState
  icon="portfolio"
  title="No portfolios yet"
  description="Create your first portfolio to get started"
  actionLabel="Create Portfolio"
  onAction={() => setShowCreate(true)}
/>
```

### Error Alerts

```tsx
<ErrorAlert 
  message="Failed to load portfolios" 
  onRetry={loadPortfolios}
  onDismiss={() => setError(null)}
  type="banner"
/>
```

---

## Gradients

### Primary Gradient (Hero)

```css
bg-gradient-to-r from-fuchsia-400 via-purple-400 to-cyan-400
```

### Card Gradient

```css
bg-gradient-to-br from-violet-950/50 to-slate-950/50
```

### Button Gradient

```css
bg-gradient-to-r from-fuchsia-600 to-violet-600
```

---

## Animation Effects

### Neon Glow

```tsx
// Button glow on hover
hover:shadow-[0_0_20px_rgba(217,70,239,0.5)]

// Card glow on hover
hover:shadow-[0_0_30px_rgba(139,92,246,0.3)]

// Text glow
drop-shadow-[0_0_10px_rgba(232,121,249,0.5)]
```

### Pulse Animation

```tsx
// Animated pulse (hero heading)
animate-pulse

// Skeleton loading
animate-pulse (built into LoadingSkeleton)
```

---

## Responsive Design

### Breakpoints

| Breakpoint | Min Width | Usage |
|------------|-----------|-------|
| `sm:` | 640px | Small devices |
| `md:` | 768px | Tablets |
| `lg:` | 1024px | Laptops |
| `xl:` | 1280px | Desktops |
| `2xl:` | 1536px | Large screens |

### Grid Layouts

```tsx
// Card grid
<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">

// Responsive text
<h1 className="text-3xl sm:text-4xl md:text-5xl lg:text-6xl">
```