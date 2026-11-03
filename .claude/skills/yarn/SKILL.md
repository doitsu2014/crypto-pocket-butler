---
name: yarn
description: >
  Use and enforce Yarn (v6.x or newer) as the only package manager for Crypto
  Pocket Butler JavaScript/TypeScript work (the React SPA and any tooling).
  Covers pinned-version setup with corepack, allowed commands, and rejecting
  npm/pnpm usage and foreign lockfiles. Use whenever installing dependencies,
  adding packages, running scripts, or creating/updating package.json,
  yarn.lock, or CI steps.
argument-hint: "[setup | add <package> | check]"
---

# Package Manager — Yarn ≥ 6 (enforced)

Yarn 6 or newer is the **only** package manager in this repo. `npm`, `npx`, `pnpm`, and their
lockfiles are forbidden.

## Commands

| Do | Don't |
| --- | --- |
| `yarn install` | `npm install` / `pnpm install` |
| `yarn add <pkg>` / `yarn add -D <pkg>` | `npm install <pkg>` |
| `yarn dlx <tool>` | `npx <tool>` |
| `yarn <script>` / `yarn run <script>` | `npm run <script>` |
| commit `yarn.lock` | commit `package-lock.json` / `pnpm-lock.yaml` / `npm-shrinkwrap.json` |

## Enforcement (set up when package.json is created)

1. Pin one version with corepack — it writes the `packageManager` field, and corepack then
   refuses other managers and other versions:

   ```bash
   corepack enable && corepack use yarn@6
   ```

   The resulting `"packageManager": "yarn@<major>.x.y+sha512-…"` field in `package.json` is
   the enforcement. The pinned major must be **6 or newer** — upgrades to a newer major are a
   normal reviewed change (`corepack use yarn@<latest>` + lockfile regen); downgrading below
   6 is not. Never hand-edit the field to another manager or an older version.
2. CI installs with `yarn install --immutable` so a stale or missing `yarn.lock` fails the
   build.
3. CI and the local check reject foreign lockfiles: the build fails if
   `package-lock.json`, `pnpm-lock.yaml`, or `npm-shrinkwrap.json` exists anywhere.

## Rules for agents

- Every install/add/dlx/script command you write or run is `yarn …` — including in docs,
  CI configs, Dockerfiles, and the Swarm stack definitions.
- Never delete or hand-edit `yarn.lock`; add packages via `yarn add` so the lock updates
  correctly.
- Dependency changes are recorded and reviewed (org standard): name what was added and why
  in the PR description.
