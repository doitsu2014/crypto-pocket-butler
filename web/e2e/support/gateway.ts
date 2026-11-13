import { expect, type Page } from "@playwright/test";
import type {
  AccountStatusView,
  ConsentHistoryView,
  ConsentStateView,
  EffectivePreferences,
  Portfolio,
  PortfolioValuation,
  Principal,
  Profile,
  ProfileView,
  SignupGateState,
} from "../../src/api/types";
import { bearer } from "./session";

/**
 * A fake gateway, answered at the network layer.
 *
 * Every `/v1` call the SPA makes is matched here. A path this table does not
 * answer gets a `404` problem body *and* is recorded, so a journey that starts
 * calling a new endpoint fails loudly instead of quietly rendering an error
 * panel into a screenshot someone might approve.
 */

export const PORTFOLIO_ID = "b1d2c3e4-0000-4000-8000-00000000a001";
export const SECOND_PORTFOLIO_ID = "b1d2c3e4-0000-4000-8000-00000000a002";

export interface GatewayFixtures {
  profile: Profile;
  profileView: ProfileView;
  principal: Principal;
  gateState: SignupGateState | null;
  portfolios: Portfolio[];
  valuations: Record<string, PortfolioValuation>;
  accountStatus: AccountStatusView;
  effectivePreferences: EffectivePreferences;
  consentState: ConsentStateView;
  consentHistory: ConsentHistoryView;
}

export const fixtures: GatewayFixtures = {
  profile: {
    userId: "3f0c2b8e-5b1a-4f2d-9c77-1a2b3c4d5e6f",
    email: "ada@example.com",
    displayName: "Ada Lovelace",
    role: "user",
    status: "active",
    mfaRequired: false,
    mfaEnrolled: true,
    preferences: { baseCurrency: "EUR", locale: "en", timezone: "UTC" },
  },
  profileView: {
    userId: "3f0c2b8e-5b1a-4f2d-9c77-1a2b3c4d5e6f",
    displayName: "Ada Lovelace",
    displayNameOverridden: false,
    providerDisplayName: "Ada Lovelace",
    email: "ada@example.com",
    emailVerified: true,
    status: "active",
  },
  gateState: { missing: [], jurisdictionBlocked: false },
  principal: {
    userId: "3f0c2b8e-5b1a-4f2d-9c77-1a2b3c4d5e6f",
    role: "owner",
    status: "active",
    displayName: "Ada Lovelace",
    emailVerified: true,
    nextStep: "none",
  },
  portfolios: [
    { id: PORTFOLIO_ID, name: "Long term", baseCurrency: "EUR" },
    { id: SECOND_PORTFOLIO_ID, name: "Play", baseCurrency: "EUR" },
  ],
  valuations: {
    [PORTFOLIO_ID]: {
      portfolioId: PORTFOLIO_ID,
      asOf: "2026-11-12T08:00:00Z",
      status: "complete",
      totalValue: "48210.55",
      currency: "EUR",
      pricedCoverage: 1,
      unpricedCount: 0,
      excludedCount: 0,
      lines: [
        {
          assetId: "asset-btc",
          symbol: "BTC",
          quantity: "0.4120",
          value: "35029.80",
          allocation: 72.7,
          targetPercent: 70,
          driftPoints: 2.7,
          guardrailPoints: 5,
          price: "85023.79",
          priceSource: "market-data",
          priceFreshness: "live",
          priceObservedAt: "2026-11-12T07:58:00Z",
          fxRate: "0.9210",
          fxSource: "market-data",
          fxObservedAt: "2026-11-12T07:58:00Z",
          dataStatus: "priced",
        },
        {
          assetId: "asset-eth",
          symbol: "ETH",
          quantity: "4.8000",
          value: "13180.75",
          allocation: 27.3,
          targetPercent: 30,
          driftPoints: -2.7,
          guardrailPoints: 5,
          price: "2982.11",
          priceSource: "market-data",
          priceFreshness: "delayed",
          priceObservedAt: "2026-11-12T07:40:00Z",
          fxRate: "0.9210",
          fxSource: "market-data",
          fxObservedAt: "2026-11-12T07:40:00Z",
          dataStatus: "priced",
        },
      ],
    },
    [SECOND_PORTFOLIO_ID]: {
      portfolioId: SECOND_PORTFOLIO_ID,
      asOf: "2026-11-12T08:00:00Z",
      status: "partial",
      totalValue: "1204.00",
      currency: "EUR",
      pricedCoverage: 0.5,
      unpricedCount: 1,
      lines: [],
    },
  },
  accountStatus: { status: "active" },
  effectivePreferences: {
    userId: "3f0c2b8e-5b1a-4f2d-9c77-1a2b3c4d5e6f",
    locale: { value: "en", source: "user" },
    baseCurrency: { value: "EUR", source: "user" },
    timezone: { value: "UTC", source: "user" },
    notifications: {
      email: { value: true, source: "default" },
      push: { value: false, source: "default" },
    },
  },
  consentState: {
    userId: "3f0c2b8e-5b1a-4f2d-9c77-1a2b3c4d5e6f",
    kinds: { terms: "granted", risk: "granted", age: "granted", analytics: "granted" },
    missing: [],
  },
  consentHistory: {
    events: [
      {
        id: "consent-1",
        seq: 1,
        kind: "analytics",
        action: "granted",
        occurredAt: "2026-11-02T10:15:00Z",
        policyVersion: "2026-11",
      },
    ],
    nextCursor: null,
  },
};

interface Handler {
  method: string;
  test: (path: string) => boolean;
  body: unknown | ((path: string) => unknown);
  status?: number;
}

function buildHandlers(overrides: Partial<GatewayFixtures>): Handler[] {
  const data: GatewayFixtures = { ...fixtures, ...overrides };
  return [
    { method: "GET", test: (p) => p === "/users/me", body: data.profile },
    { method: "GET", test: (p) => p === "/me/profile", body: data.profileView },
    { method: "GET", test: (p) => p === "/me/principal", body: data.principal },
    {
      method: "GET",
      test: (p) => p === "/signup/gate-state",
      body: data.gateState,
    },
    { method: "GET", test: (p) => p === "/portfolios", body: data.portfolios },
    {
      method: "GET",
      test: (p) =>
        /^\/portfolios\/[^/]+$/.test(p) && data.valuations[p.split("/")[2]] !== undefined,
      body: (p) => data.valuations[p.split("/")[2]],
    },
    {
      method: "GET",
      test: (p) => p === "/me/account-status",
      body: data.accountStatus,
    },
    {
      method: "GET",
      test: (p) => p === "/me/preferences",
      body: data.effectivePreferences,
    },
    {
      method: "GET",
      test: (p) => p === "/me/consent/state",
      body: data.consentState,
    },
    {
      method: "GET",
      test: (p) => p === "/me/consent",
      body: data.consentHistory,
    },
    {
      method: "POST",
      test: (p) => p === "/telemetry/client-errors",
      body: null,
      status: 202,
    },
  ];
}

/**
 * Installs the gateway mock. Returns the list of paths nothing answered; assert
 * it is empty at the end of a journey with {@link expectNoUnmatched}.
 */
export async function mockGateway(
  page: Page,
  overrides: Partial<GatewayFixtures> = {},
): Promise<string[]> {
  const handlers = buildHandlers(overrides);
  const unmatched: string[] = [];

  // Matched on the path prefix, not `**/v1/**`: that glob would also match the
  // Supabase auth endpoint at `/auth/v1/...`.
  await page.route(
    (url) => url.pathname.startsWith("/v1/"),
    async (route) => {
      const request = route.request();
      const url = new URL(request.url());
      const path = url.pathname.replace(/^\/v1/, "") || "/";

      // Every gateway call carries the user's token unchanged (NFR-SEC-09).
      const authorization = request.headers()["authorization"];
      if (authorization !== bearer()) {
        unmatched.push(`${request.method()} ${path} (missing bearer token)`);
      }

      const handler = handlers.find(
        (candidate) => candidate.method === request.method() && candidate.test(path),
      );

      if (!handler) {
        unmatched.push(`${request.method()} ${path}`);
        await route.fulfill({
          status: 404,
          contentType: "application/problem+json",
          body: JSON.stringify({
            type: "https://errors.cpb/e2e/not-mocked",
            title: "This journey has no fixture for the call",
            status: 404,
            detail: `${request.method()} ${path}`,
            correlationId: "e2e-not-mocked",
          }),
        });
        return;
      }

      const body =
        typeof handler.body === "function"
          ? (handler.body as (path: string) => unknown)(path)
          : handler.body;

      await route.fulfill({
        status: handler.status ?? 200,
        contentType: "application/json",
        body: JSON.stringify(body),
      });
    },
  );

  return unmatched;
}

/** Fails with the exact list of calls the journey did not anticipate. */
export function expectNoUnmatched(unmatched: string[]): void {
  expect(unmatched, `unexpected gateway calls: ${unmatched.join(", ")}`).toEqual([]);
}
