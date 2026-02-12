# UI Standardization - Visual Reference

This document provides a visual reference for the standardized UI components.

## Toast Notifications

### Success Toast (Green Theme)
```
┌────────────────────────────────────────┐
│ ✓  Portfolio created successfully!  ✕ │
└────────────────────────────────────────┘
- Background: Green-950/90 with blur
- Border: Green-500/70 with glow
- Icon: Check circle (green-400)
- Auto-dismisses in 5 seconds
```

### Error Toast (Red Theme)
```
┌────────────────────────────────────────┐
│ ⊗  Failed to load data              ✕ │
└────────────────────────────────────────┘
- Background: Red-950/90 with blur
- Border: Red-500/70 with glow
- Icon: X circle (red-400)
- Auto-dismisses in 5 seconds
```

### Info Toast (Cyan Theme)
```
┌────────────────────────────────────────┐
│ ℹ  Processing your request...       ✕ │
└────────────────────────────────────────┘
- Background: Cyan-950/90 with blur
- Border: Cyan-500/70 with glow
- Icon: Info circle (cyan-400)
```

### Warning Toast (Yellow Theme)
```
┌────────────────────────────────────────┐
│ ⚠  This action cannot be undone     ✕ │
└────────────────────────────────────────┘
- Background: Yellow-950/90 with blur
- Border: Yellow-500/70 with glow
- Icon: Warning triangle (yellow-400)
```

## Loading Skeletons

### Card Skeleton (for grid layouts)
```
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ ████████     ■  │  │ ███████      ■  │  │ ████████     ■  │
│ ████            │  │ ████             │  │ ███              │
│ ███████         │  │ ████████         │  │ ████████         │
└─────────────────┘  └─────────────────┘  └─────────────────┘
- Animated pulse effect
- Violet/purple gradient bars
- Card container with border
```

### List Skeleton (for list views)
```
┌────────────────────────────────────────┐
│ ████████                               │
│ ████████████████████████████████       │
└────────────────────────────────────────┘
┌────────────────────────────────────────┐
│ ██████                                 │
│ ████████████████████████████           │
└────────────────────────────────────────┘
- Animated pulse effect
- Horizontal bars of varying width
- List item containers
```

### Table Skeleton (for data tables)
```
┌─────────────────────────────────────────────────────────┐
│ [Header Row - darker background]                        │
├─────────────────────────────────────────────────────────┤
│ ███████     ██████████     ████████                     │
│ ████        ████████████   ████████                     │
│ ██████      ████████       ██████                       │
└─────────────────────────────────────────────────────────┘
- Header row with separator
- Multiple content rows
- Column-like structure
```

## Loading Spinner

### Small (sm)
```
    ⟳
(6x6 pixels)
- Rotating border animation
- Violet/fuchsia colored
- For inline loading
```

### Medium (md) - Default
```
      ⟳
  (10x10 pixels)
- With optional message text
- Centered in container
- For section loading
```

### Large (lg)
```
        ⟳
   (16x16 pixels)
- Prominent spinner
- With message text below
- For full-page loading
```

## Empty States

### Portfolio Empty State
```
┌─────────────────────────────────────────┐
│                                         │
│              📁 (icon)                  │
│                                         │
│         No portfolios yet               │
│   Create your first portfolio to        │
│           get started                   │
│                                         │
│      [Create Portfolio Button]          │
│                                         │
└─────────────────────────────────────────┘
- Cyan theme with border glow
- Icon (portfolio briefcase)
- Title and description text
- Optional action button
```

### Account Empty State
```
┌─────────────────────────────────────────┐
│                                         │
│              💳 (icon)                  │
│                                         │
│          No accounts yet                │
│  Connect your wallets and exchanges     │
│         to start tracking               │
│                                         │
│       [Add Account Button]              │
│                                         │
└─────────────────────────────────────────┘
- Cyan theme
- Icon (credit card)
- Descriptive text
```

### Recommendation Empty State
```
┌─────────────────────────────────────────┐
│                                         │
│              💡 (icon)                  │
│                                         │
│   No recommendations available yet      │
│  Generate AI-powered suggestions for    │
│       your portfolio                    │
│                                         │
│    [Generate Recommendations]           │
│                                         │
└─────────────────────────────────────────┘
- Cyan theme
- Icon (light bulb)
- Action-oriented text
```

### Snapshot Empty State
```
┌─────────────────────────────────────────┐
│                                         │
│              📊 (icon)                  │
│                                         │
│         No snapshots yet                │
│    Historical data will appear here     │
│         once synced                     │
│                                         │
└─────────────────────────────────────────┘
- Cyan theme
- Icon (bar chart)
- Informational text
```

## Error Alerts

### Banner Error (with retry)
```
┌─────────────────────────────────────────────────────┐
│ ⊗  Failed to load portfolios    [Retry]  [✕]       │
└─────────────────────────────────────────────────────┘
- Red theme with border
- Error icon on left
- Message text in center
- Retry button (underlined)
- Dismiss button (X) on right
- Full width banner
```

### Inline Error (with dismiss)
```
┌──────────────────────────────────┐
│ ⊗  Invalid input data        ✕  │
└──────────────────────────────────┘
- Red theme
- Error icon
- Shorter message
- Dismiss button only
- Fits within section
```

## Loading Button States

### Default State
```
┌─────────────────┐
│  Create         │
└─────────────────┘
- Normal button appearance
- Full opacity
- Clickable
```

### Loading State
```
┌─────────────────┐
│  ⟳ Creating...  │
└─────────────────┘
- Spinner icon (rotating)
- Loading text
- Disabled state
- Reduced opacity
```

## Component Interactions

### Form Submission Flow
```
1. User fills form
   ┌────────────────────────────┐
   │ Name: [My Portfolio____]   │
   │                            │
   │ [Create]                   │
   └────────────────────────────┘

2. Click Create → Button shows loading
   ┌────────────────────────────┐
   │ Name: [My Portfolio____]   │
   │                            │
   │ [⟳ Creating...]            │
   └────────────────────────────┘

3. Success → Toast notification appears
   ┌────────────────────────────────┐ (top-right)
   │ ✓ Portfolio created!        ✕ │
   └────────────────────────────────┘
   
   ┌────────────────────────────┐
   │ (Form hidden/reset)        │
   │                            │
   │ (Portfolio list refreshed) │
   └────────────────────────────┘

4. Error → Toast notification appears
   ┌────────────────────────────────┐ (top-right)
   │ ⊗ Failed to create          ✕ │
   └────────────────────────────────┘
   
   ┌────────────────────────────┐
   │ Name: [My Portfolio____]   │
   │                            │
   │ [Create] (enabled again)   │
   └────────────────────────────┘
```

### Data Loading Flow
```
1. Page loads → Skeleton appears
   ┌─────────────────┐  ┌─────────────────┐
   │ ████████     ■  │  │ ███████      ■  │
   │ ████            │  │ ████             │
   └─────────────────┘  └─────────────────┘

2. Success → Data displays
   ┌─────────────────┐  ┌─────────────────┐
   │ Portfolio 1  📁 │  │ Portfolio 2  📁 │
   │ Main holdings   │  │ Alt investments │
   └─────────────────┘  └─────────────────┘

3. Error → Error alert with retry
   ┌─────────────────────────────────────────┐
   │ ⊗ Failed to load    [Retry]  [✕]       │
   └─────────────────────────────────────────┘

4. No data → Empty state
   ┌─────────────────────────────────────────┐
   │              📁                          │
   │       No portfolios yet                  │
   │  Create your first portfolio             │
   │      [Create Portfolio]                  │
   └─────────────────────────────────────────┘
```

## Color Palette

### Success (Green)
- Background: `bg-green-950/30`
- Border: `border-green-500/50`
- Text: `text-green-300`
- Glow: `shadow-[0_0_20px_rgba(34,197,94,0.4)]`

### Error (Red)
- Background: `bg-red-950/30`
- Border: `border-red-500/50`
- Text: `text-red-300`
- Glow: `shadow-[0_0_20px_rgba(239,68,68,0.25)]`

### Info (Cyan)
- Background: `bg-cyan-950/30`
- Border: `border-cyan-500/40`
- Text: `text-cyan-400`
- Glow: `shadow-[0_0_25px_rgba(34,211,238,0.3)]`

### Warning (Yellow)
- Background: `bg-yellow-950/30`
- Border: `border-yellow-500/50`
- Text: `text-yellow-300`
- Glow: `shadow-[0_0_20px_rgba(234,179,8,0.4)]`

### Loading (Violet/Purple)
- Background: `bg-slate-950/70`
- Border: `border-violet-500/40`
- Skeleton: `bg-violet-900/50`
- Glow: `shadow-[0_0_25px_rgba(139,92,246,0.3)]`

## Animations

### Slide-in (Toast)
```css
@keyframes slide-in {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}
Duration: 0.3s ease-out
```

### Pulse (Skeleton)
```css
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
Duration: 2s infinite
```

### Spin (Loading Spinner)
```css
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
Duration: 1s linear infinite
```

## Responsive Behavior

### Desktop (lg: 1024px+)
- Toast: Fixed top-right, max-width 384px
- Skeleton cards: 3 columns
- Empty state: Centered, max-width 512px

### Tablet (md: 768px+)
- Toast: Fixed top-right, max-width 384px
- Skeleton cards: 2 columns
- Empty state: Centered, max-width 448px

### Mobile (sm: 640px+)
- Toast: Fixed top-right, max-width 320px
- Skeleton cards: 1 column
- Empty state: Centered, full width padding

## Accessibility

### Screen Reader Support
- Toast: `role="alert"` for immediate announcement
- Loading: `aria-label="Loading"` on spinners
- Buttons: `aria-busy="true"` during loading
- Empty states: Semantic heading structure

### Keyboard Navigation
- Toast dismiss: Focusable X button
- Error alert retry: Focusable button
- Empty state action: Focusable button
- All interactive elements: Tab order preserved

### Color Contrast
- All text meets WCAG AA standards (4.5:1 minimum)
- Error text: Red-300 on Red-950 background
- Success text: Green-300 on Green-950 background
- Loading text: Slate-400 on dark background

## Z-Index Layers
```
Layer 5 (z-50): Toast notifications (top-most)
Layer 4 (z-40): Modal dialogs
Layer 3 (z-30): Dropdown menus
Layer 2 (z-20): Sticky headers
Layer 1 (z-10): Overlays
Layer 0 (z-0):  Base content
```

## Performance Considerations

### Toast System
- Maximum 5 simultaneous toasts
- Auto-cleanup after dismiss/timeout
- No memory leaks (proper cleanup on unmount)

### Loading Skeletons
- Pure CSS animations (no JavaScript)
- Hardware-accelerated transforms
- Minimal DOM nodes

### Error Handling
- Debounced retry button (prevent spam)
- Error messages cached
- API errors categorized for smart retry
