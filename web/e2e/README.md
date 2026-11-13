# Browser journeys

Playwright drives a real Chromium against the SPA in `web/`. These are the only tests in the
project that render the app in a browser; every other frontend test runs in jsdom against mocks.

They exist because a crash can be invisible to a unit test: the first run of this suite found that
`sessionStore` was handed the Supabase client instead of `client.auth`, so `getSession` was
undefined and the very first render threw `Unexpected Application Error`. Every unit test injected
a fake, and every one of them passed.

## Run

```bash
yarn --cwd web e2e           # all journeys, then the report is written
yarn --cwd web e2e:report    # open the HTML report
yarn --cwd web e2e --ui      # interactive runner
yarn --cwd web e2e -g "sign-in"   # one journey by title
```

The first run needs the browser once: `yarn --cwd web exec playwright install chromium`.

A run needs **no Docker, no database, no credentials**: the gateway and Supabase Auth are answered
at the network layer (`e2e/support/`). Vite is started for you on port 4173 (override with
`E2E_PORT`).

## Where the evidence is

| What                                             | Where                                                    |
| ------------------------------------------------ | -------------------------------------------------------- |
| Screenshots of every step that calls `capture()` | `e2e/screenshots/<journey>/NN-name.png`                  |
| The same screenshots, inline in context          | the HTML report — open the test, look at its attachments |
| Failure screenshots, traces, `error-context.md`  | `e2e/.artifacts/test-results/`                           |
| The report itself                                | `e2e/.artifacts/report/index.html`                       |

All three folders are gitignored; they are evidence for a run, not source.

A screenshot is taken only after the journey's assertions pass, so a picture in that folder is a
state the test proved — not a snapshot of whatever happened to be on screen.

## Layout

```
playwright.config.ts       the runner: dev server, reporter, viewport, trace policy
e2e/journeys/*.spec.ts     one file per journey, named for the surface it covers
e2e/support/session.ts     fake Supabase Auth + the sign-in helper (fills the real form)
e2e/support/gateway.ts     the `/v1` fixtures, plus the list of calls nothing answered
e2e/support/capture.ts     screenshot → report attachment + `e2e/screenshots/`
```

## Ground rules

- **Navigate by clicking, not `page.goto`.** The access token lives in memory only (NFR9.1), so a
  full page load ends the session by design. Sign in once through the form, then click.
- **Sign-in goes through the real form.** Nothing injects a session; the journey proves the path a
  person takes.
- **The gateway mock is exhaustive.** A `/v1` path with no fixture returns a `404` problem _and_ is
  recorded; every journey calls `expectNoUnmatched(unmatched)` at the end, so a newly-introduced
  call fails the journey instead of quietly painting an error panel into a screenshot.
- **Every `/v1` call must carry the session token**, and the mock fails the request if it does not —
  the browser side of "the JWT is forwarded unchanged" (NFR-SEC-09).
- **Screenshots are for review, not pixel diffing.** No `toHaveScreenshot` baseline exists; content
  that is genuinely unstable should be masked (`capture(page, testInfo, name, { mask })`) rather
  than allowed to make the picture meaningless.

## What these tests do not prove

There is no live gateway, no database and no real Supabase behind them, so they say nothing about
a service's behaviour. Service-level behaviour is proven by the Rust suites, including the
`#[ignore]` adapter and migration suites that run against a disposable Postgres.

To point the same specs at a running stack later, start the app with real `VITE_SUPABASE_URL` /
`VITE_GATEWAY_URL` and drop the two `mock*` calls from the journey; the selectors and assertions
stay as they are.
