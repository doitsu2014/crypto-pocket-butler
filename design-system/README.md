# Gilded Ledger — Design System Package

Luxury dark design system for **Crypto Pocket Butler**: black-lacquer
surfaces, ivory Didone display type, one champagne-gold accent, tabular mono
figures.

## Contents

| File | What it is |
|---|---|
| `DESIGN.md` | The canonical written system — palette roles, typography, components, data conventions, do's and don'ts |
| `tokens.css` | The paste-ready `:root` token block every artifact should use |
| `index.html` | Visual style guide — open in a browser |
| `splash.html` | Loading splash screen — open in a browser |
| `logo.svg` | Primary lockup: seal + "Gilded Ledger" wordmark |
| `logo-mark.svg` | Emblem alone; doubles as a favicon |
| `manifest.json` | Package metadata and file map |

## Using it

1. Paste the `:root` block from `tokens.css` into the first `<style>` of any
   new artifact (or link the file).
2. Reference every color, size, and duration with `var(--name)` — never raw
   hex values.
3. Build from the components documented in `DESIGN.md` and shown in
   `index.html`.
4. Fonts load from Google Fonts: Playfair Display (display), Jost (interface),
   IBM Plex Mono (figures).

Sample balances, prices, and drift figures throughout are labelled sample
data — illustrative only.
