# Crypto Pocket Butler - UI/UX Style Guide

This document defines the visual design system for the Crypto Pocket Butler frontend application. Use this guide to maintain consistency when adding new features or making UI changes.

## Design Philosophy

**Theme**: Dark, Mysterious, Neon Cyberpunk
- **Core Concept**: High-tech cryptocurrency management with a secretive, futuristic aesthetic
- **Mood**: Professional yet mysterious, secure yet vibrant
- **Visual Style**: Intense neon glows, holographic effects, deep blacks with vibrant accent colors

---

## Color Palette

### Primary Colors
```css
/* Fuchsia/Pink (Primary Accent) */
--fuchsia-300: rgb(232 121 249)  /* Text highlights */
--fuchsia-400: rgb(232 121 249)  /* Gradient starts */
--fuchsia-500: rgb(217 70 239)   /* Borders, buttons */
--fuchsia-600: rgb(192 38 211)   /* Button backgrounds */
--fuchsia-950: rgb(74 4 78)      /* Dark backgrounds */

/* Violet/Purple (Secondary) */
--violet-300: rgb(196 181 253)   /* Light text */
--violet-400: rgb(167 139 250)   /* Gradient mid */
--violet-500: rgb(139 92 246)    /* Borders, icons */
--violet-600: rgb(124 58 237)    /* Buttons */
--violet-900: rgb(76 29 149)     /* Backgrounds */
--violet-950: rgb(46 16 101)     /* Deep backgrounds */

/* Cyan (Tertiary Accent) */
--cyan-300: rgb(103 232 249)     /* Light text */
--cyan-400: rgb(34 211 238)      /* Highlights */
--cyan-500: rgb(6 182 212)       /* Borders */
--cyan-600: rgb(8 145 178)       /* Buttons */

/* Base Colors */
--black: rgb(0 0 0)              /* Pure black background */
--slate-900: rgb(15 23 42)       /* Card backgrounds */
--slate-950: rgb(2 6 23)         /* Dark card backgrounds */
--slate-200: rgb(226 232 240)    /* Primary text */
--slate-300: rgb(203 213 225)    /* Secondary text */
```

### Usage Guidelines
- **Fuchsia**: Primary CTAs, important icons, main interactive elements
- **Violet**: Secondary elements, supporting UI components  
- **Cyan**: Information displays, portfolio/data visualizations
- **Red**: Danger actions (sign out, delete), error states
- **Green**: Success states, verification indicators

---

## Typography

### Font Families
- **Sans-serif** (System default): Primary UI font
- **Monospace** (for data/numbers): User for IDs, addresses, numeric values

### Text Styles

#### Headings
```tsx
/* Main Hero Heading */
className="text-5xl sm:text-6xl md:text-7xl font-extrabold 
           bg-gradient-to-r from-fuchsia-400 via-purple-400 to-cyan-400 
           bg-clip-text text-transparent 
           drop-shadow-[0_0_30px_rgba(168,85,247,0.8)] 
           animate-pulse"

/* Section Heading */
className="text-3xl font-extrabold 
           bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300 
           bg-clip-text text-transparent 
           drop-shadow-[0_0_20px_rgba(232,121,249,0.6)]"

/* Card Heading */
className="text-lg font-bold text-fuchsia-300 
           drop-shadow-[0_0_10px_rgba(232,121,249,0.5)]"
```

#### Body Text
```tsx
/* Primary Text */
className="text-slate-200 drop-shadow-[0_0_10px_rgba(226,232,240,0.3)]"

/* Secondary Text */
className="text-slate-300"

/* Small Text */
className="text-sm text-slate-300"
```

---

## Components

### Buttons

#### Primary CTA Button (Fuchsia Neon)
```tsx
<button className="
  inline-flex items-center px-8 py-3 
  border-2 border-fuchsia-500 
  text-base font-bold rounded-lg 
  text-white 
  bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600 
  hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500 
  shadow-[0_0_30px_rgba(217,70,239,0.6)] 
  hover:shadow-[0_0_50px_rgba(217,70,239,0.9)] 
  transition-all duration-300 
  transform hover:scale-110 
  animate-pulse
">
  Button Text
</button>
```

#### Secondary Button (Cyan)
```tsx
<button className="
  inline-flex items-center px-8 py-3 
  border-2 border-cyan-500 
  text-base font-medium rounded-lg 
  text-cyan-200 
  bg-slate-900/50 hover:bg-slate-800/70 
  hover:border-cyan-400 
  backdrop-blur-sm 
  shadow-[0_0_20px_rgba(34,211,238,0.4)] 
  hover:shadow-[0_0_40px_rgba(34,211,238,0.7)] 
  transition-all duration-300 
  transform hover:scale-105
">
  Button Text
</button>
```

#### Danger Button (Red)
```tsx
<button className="
  inline-flex items-center px-4 py-2 
  border-2 border-red-500 
  text-sm font-bold rounded-lg 
  text-red-300 
  bg-red-950/30 hover:bg-red-900/50 
  hover:border-red-400 
  shadow-[0_0_20px_rgba(239,68,68,0.4)] 
  hover:shadow-[0_0_30px_rgba(239,68,68,0.7)] 
  transition-all duration-300 
  transform hover:scale-105
">
  Button Text
</button>
```

### Cards

#### Primary Card (Fuchsia Border)
```tsx
<div className="
  bg-slate-950/70 backdrop-blur-sm 
  border-2 border-fuchsia-500/40 
  shadow-[0_0_40px_rgba(217,70,239,0.4)] 
  rounded-2xl p-6
">
  Card Content
</div>
```

#### Secondary Card (Cyan Border)
```tsx
<div className="
  bg-slate-950/70 backdrop-blur-sm 
  border-2 border-cyan-500/40 
  shadow-[0_0_40px_rgba(34,211,238,0.4)] 
  rounded-2xl p-6
">
  Card Content
</div>
```

#### Hover Card (Feature Card)
```tsx
<div className="
  group 
  bg-slate-900/60 backdrop-blur-sm 
  rounded-xl 
  border-2 border-fuchsia-500/50 hover:border-fuchsia-400 
  p-6 
  transition-all duration-300 
  shadow-[0_0_30px_rgba(217,70,239,0.3)] 
  hover:shadow-[0_0_50px_rgba(217,70,239,0.6)] 
  hover:transform hover:scale-105
">
  Card Content
</div>
```

### Icons

#### Glowing Icon Box (Fuchsia)
```tsx
<div className="
  w-12 h-12 rounded-xl 
  bg-gradient-to-br from-fuchsia-500 to-violet-600 
  flex items-center justify-center 
  shadow-[0_0_30px_rgba(217,70,239,0.8)] 
  animate-pulse
">
  <svg className="w-6 h-6 text-white drop-shadow-[0_0_10px_rgba(255,255,255,0.8)]">
    {/* SVG path */}
  </svg>
</div>
```

#### Glowing Icon Box (Cyan)
```tsx
<div className="
  w-12 h-12 rounded-xl 
  bg-gradient-to-br from-cyan-500 to-blue-600 
  flex items-center justify-center 
  shadow-[0_0_30px_rgba(34,211,238,0.8)] 
  animate-pulse
">
  <svg className="w-7 h-7 text-white drop-shadow-[0_0_10px_rgba(255,255,255,0.8)]">
    {/* SVG path */}
  </svg>
</div>
```

---

## Visual Effects

### Neon Glow (Box Shadows)

**Intensity Levels:**
```css
/* Subtle Glow */
shadow-[0_0_20px_rgba(217,70,239,0.3)]

/* Medium Glow */
shadow-[0_0_30px_rgba(217,70,239,0.6)]

/* Intense Glow */
shadow-[0_0_50px_rgba(217,70,239,0.9)]

/* Ultra Intense Glow */
shadow-[0_0_80px_rgba(217,70,239,0.6)]
```

### Text Glow (Drop Shadow)
```css
/* Subtle Text Glow */
drop-shadow-[0_0_10px_rgba(232,121,249,0.5)]

/* Medium Text Glow */
drop-shadow-[0_0_20px_rgba(168,85,247,0.6)]

/* Intense Text Glow */
drop-shadow-[0_0_30px_rgba(168,85,247,0.8)]
```

### Background Effects

#### Glowing Orbs
```tsx
{/* Fuchsia Orb */}
<div className="
  absolute top-0 left-0 
  w-[600px] h-[600px] 
  bg-fuchsia-500/30 
  rounded-full blur-[120px] 
  animate-pulse
"></div>

{/* Cyan Orb */}
<div className="
  absolute bottom-0 right-0 
  w-[500px] h-[500px] 
  bg-cyan-500/30 
  rounded-full blur-[120px] 
  animate-pulse" 
  style={{ animationDelay: '1s' }}
></div>
```

#### Grid Pattern Overlay
```tsx
<div className="
  absolute inset-0 
  bg-[url('data:image/svg+xml;base64,...')] 
  opacity-30
"></div>
```

### Backdrop Effects
```css
/* Frosted Glass */
backdrop-blur-sm   /* Light blur (4px) */
backdrop-blur-xl   /* Heavy blur (24px) */
```

---

## Animations

### Pulse Animation
```tsx
/* Apply to icons, indicators, primary CTAs */
className="animate-pulse"
```

### Hover Animations
```css
/* Scale Up */
hover:scale-105   /* 5% scale */
hover:scale-110   /* 10% scale */

/* Translate */
group-hover:translate-x-1  /* Move right on parent hover */

/* Shadow Intensity */
hover:shadow-[0_0_50px_rgba(217,70,239,0.9)]
```

### Animation Delays (Stagger Effect)
```tsx
style={{ animationDelay: '0.5s' }}
style={{ animationDelay: '1s' }}
style={{ animationDelay: '2s' }}
```

---

## Layout Patterns

### Full-Screen Container
```tsx
<div className="min-h-screen bg-black relative overflow-hidden">
  {/* Background effects */}
  {/* Content */}
</div>
```

### Centered Content
```tsx
<div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
  {/* Content */}
</div>
```

### Grid Layout (Feature Cards)
```tsx
<div className="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3">
  {/* Cards */}
</div>
```

---

## Responsive Design

### Breakpoints (Tailwind Default)
- **sm**: 640px
- **md**: 768px
- **lg**: 1024px
- **xl**: 1280px
- **2xl**: 1536px

### Text Scaling
```tsx
className="text-5xl sm:text-6xl md:text-7xl"  /* Heading */
className="text-4xl sm:text-5xl"              /* Subheading */
```

---

## Accessibility

### Focus States
```css
focus:outline-none
focus:ring-4 focus:ring-fuchsia-500/50
```

### Interactive Indicators
```tsx
{/* Pulsing Green Dot (Online/Secure) */}
<span className="
  w-2 h-2 
  bg-green-400 
  rounded-full 
  animate-pulse 
  shadow-[0_0_10px_rgba(74,222,128,0.8)]
"></span>
```

---

## State Variations

### Loading State
```tsx
<div className="animate-pulse">
  <div className="h-4 bg-violet-900/50 rounded w-3/4 mb-2 
                  shadow-[0_0_10px_rgba(139,92,246,0.3)]"></div>
  <div className="h-4 bg-violet-900/50 rounded w-1/2 
                  shadow-[0_0_10px_rgba(139,92,246,0.3)]"></div>
</div>
```

### Error State
```tsx
<div className="
  bg-red-950/30 
  border-2 border-red-500/50 
  rounded-xl p-4 
  shadow-[0_0_30px_rgba(239,68,68,0.3)]
">
  <p className="text-red-300 text-sm 
                drop-shadow-[0_0_5px_rgba(252,165,165,0.5)]">
    Error message
  </p>
</div>
```

### Success Indicator
```tsx
<span className="
  inline-flex items-center gap-1 
  text-green-400 
  drop-shadow-[0_0_10px_rgba(74,222,128,0.6)]
">
  <svg className="w-4 h-4" /* checkmark icon */>
  Success Text
</span>
```

---

## Best Practices

1. **Always use neon glows on interactive elements** - Buttons, links, cards should have shadow glows
2. **Maintain dark backgrounds** - Use `bg-black` or `bg-slate-950` for base layers
3. **Use gradients for important text** - Hero headings and CTA text should use gradient + glow
4. **Apply backdrop blur to overlays** - Cards and modals should have `backdrop-blur-sm` or `backdrop-blur-xl`
5. **Animate pulsing effects on key elements** - Icons, indicators, primary buttons
6. **Use border thickness consistently** - `border-2` for prominent elements, `border` for subtle ones
7. **Layer multiple glowing orbs** - Create depth with staggered animation delays
8. **Maintain high contrast** - Light text on dark backgrounds with sufficient glow for readability
9. **Use transform animations sparingly** - Scale effects on hover for interactive elements only
10. **Test glow intensity** - Ensure glows are visible but not overwhelming

---

## Do's and Don'ts

### ✅ Do's
- Use intense neon colors (fuchsia, cyan, violet)
- Apply glowing shadows to all interactive elements
- Use pure black (#000) for main background
- Add subtle animations (pulse, scale, translate)
- Layer multiple visual effects (gradient + glow + backdrop blur)
- Use frosted glass effect on cards
- Animate orbs in background with staggered delays
- Use bold/extrabold fonts for headings

### ❌ Don'ts
- Don't use light backgrounds
- Don't use subtle or pastel colors without glow
- Don't remove border glows from interactive elements
- Don't use static flat colors for buttons
- Don't skip drop shadows on important text
- Don't use small borders (less than 2px) on prominent elements
- Don't apply too many animations to single element
- Don't use low contrast text

---

## Quick Reference: Common Patterns

### Neon Button
```
border-2 + gradient background + shadow glow + hover scale + animate-pulse
```

### Glowing Card
```
backdrop-blur + border-2 + shadow glow + hover effects
```

### Hero Heading
```
gradient text + drop-shadow + animate-pulse
```

### Icon Badge
```
gradient background + shadow glow + animate-pulse + rounded-xl
```

### Background Layer
```
black base + glowing orbs + grid pattern overlay + high opacity
```

---

## Version History

- **v1.0** (Current) - Initial neon cyberpunk theme with intense glows and holographic effects
  - Pure black backgrounds
  - Fuchsia/violet/cyan neon palette
  - High-intensity box shadows and glows
  - Animated pulsing elements
  - Multi-layer background effects

---

## Future Considerations

- **Animation library integration** - Consider Framer Motion for more complex animations
- **Theme variants** - Potential for user-selectable intensity levels (subtle/normal/intense)
- **3D effects** - Explore CSS 3D transforms for depth
- **Particle effects** - Consider adding subtle particle animations for enhanced mystery
- **Color pulsing** - Animated color shifts in gradients for dynamic feel

---

*This style guide should be referenced for all UI/UX work on the Crypto Pocket Butler frontend to maintain visual consistency and the mysterious neon aesthetic.*
