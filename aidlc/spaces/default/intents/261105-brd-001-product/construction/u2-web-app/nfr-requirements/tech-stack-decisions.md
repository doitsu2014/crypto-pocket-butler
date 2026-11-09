# Tech Stack Decisions — u2-web-app

| Concern | Decision | Alternative rejected |
|---|---|---|
| UI | React 18.3 + TypeScript 5.7 on Vite 6 (project mandate) | Next.js (server rendering not needed) |
| Routing | React Router `createBrowserRouter` | Hand-rolled switch (no deep-link or scroll handling) |
| Server state | TanStack Query 5 | Redux Toolkit Query (heavier, not in the stack) |
| Styling | Tailwind CSS v4 mapped to Parchment Ledger tokens in `design-system/tokens.css` | CSS-in-JS (raw values, not the stack) |
| Auth | `@supabase/supabase-js` with in-memory storage and PKCE | Storing a session in localStorage (forbidden) |
| Charts | Recharts (project mandate) | Chart.js |
| i18n | One JSON resource per locale loaded by a small module | `react-i18next` (extra dependency not needed for two locales) |
| Tests | Vitest + Testing Library; Playwright for core-flow smoke and axe | Jest (not the Vite default) |
| Wallet | WalletConnect read-only session, no signing | Ethers signer (forbidden) |
| Package manager | Yarn pinned through corepack at 4.18.1 (`web/package.json` `packageManager`). The team mandate's "Yarn ≥ 6" is read as "Yarn (modern, corepack-pinned), never npm/pnpm"; this row records 4.18.1 as the project's actual version | npm/pnpm (forbidden) |
