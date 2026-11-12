import { defineConfig, devices } from "@playwright/test";

/**
 * Browser journeys for the SPA.
 *
 * These are the only tests in the project that drive a real browser. They run
 * against the Vite dev server with the gateway and Supabase Auth mocked at the
 * network layer (`e2e/support/`), so a run needs no Docker, no database and no
 * credentials — and it fails if the UI cannot reach a state a person could.
 *
 * Every step that matters takes a screenshot, attached to the HTML report and
 * also written to `e2e/screenshots/` so the run can be reviewed without
 * re-running it.
 */
const PORT = Number(process.env.E2E_PORT ?? 4173);
const BASE_URL = `http://127.0.0.1:${PORT}`;

export default defineConfig({
  testDir: "./e2e/journeys",
  outputDir: "./e2e/.artifacts/test-results",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  workers: process.env.CI ? 2 : undefined,
  timeout: 30_000,
  expect: { timeout: 7_000 },
  reporter: [
    ["list"],
    [
      "html",
      {
        outputFolder: "e2e/.artifacts/report",
        open: "never",
        title: "Crypto Pocket Butler — browser journeys",
      },
    ],
  ],
  use: {
    baseURL: BASE_URL,
    // A failure keeps its own evidence even when a step forgot to capture.
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "off",
    viewport: { width: 1440, height: 900 },
    locale: "en-GB",
    timezoneId: "UTC",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    // The Supabase client only exists when both values are non-empty; the
    // network layer is mocked, so the values themselves are never dialled.
    command: `yarn vite --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: BASE_URL,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    env: {
      VITE_SUPABASE_URL: "http://127.0.0.1:1",
      VITE_SUPABASE_ANON_KEY: "e2e-anon-key",
    },
  },
});
