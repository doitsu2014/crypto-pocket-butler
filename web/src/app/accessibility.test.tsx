import { describe, it } from "vitest";
import { cleanup } from "@testing-library/react";
import type { ReactElement } from "react";
import ActivityPage from "../features/activity/ActivityPage";
import SignInPage from "../features/auth/SignInPage";
import VerifyEmailPage from "../features/auth/VerifyEmailPage";
import BriefDetailPage from "../features/briefs/BriefDetailPage";
import BriefsPage from "../features/briefs/BriefsPage";
import ConnectPage from "../features/connect/ConnectPage";
import HelpPage from "../features/help/HelpPage";
import NotificationsPage from "../features/notifications/NotificationsPage";
import OnboardingPage from "../features/onboarding/OnboardingPage";
import DashboardPage from "../features/portfolio/DashboardPage";
import HistoryPage from "../features/portfolio/HistoryPage";
import IntentsPage from "../features/portfolio/IntentsPage";
import PortfolioDetailPage from "../features/portfolio/PortfolioDetailPage";
import PortfoliosPage from "../features/portfolio/PortfoliosPage";
import TargetsPage from "../features/portfolio/TargetsPage";
import TradesPage from "../features/portfolio/TradesPage";
import PrivacyPage from "../features/privacy/PrivacyPage";
import MfaPage from "../features/settings/MfaPage";
import SettingsPage from "../features/settings/SettingsPage";
import type { Brief, Pnl, PortfolioValuation, Profile, SessionInfo } from "../api/types";
import { problem, stubGateway } from "../test/gateway";
import { renderWithProviders } from "../test/render";
import { expectNoAxeViolations } from "../test/axe";

const valuation: PortfolioValuation = {
  portfolioId: "p1",
  asOf: "2026-11-08T12:00:00Z",
  status: "partial",
  totalValue: "1000.00",
  currency: "USD",
  pricedCoverage: 0.9,
  unpricedCount: 1,
  excludedCount: 1,
  lines: [
    {
      assetId: "btc",
      symbol: "BTC",
      quantity: "0.5",
      value: "900.00",
      allocation: 90,
      targetPercent: 60,
      driftPoints: 30,
      price: "1800.00",
      priceSource: "Binance",
      priceFreshness: "live",
      priceObservedAt: "2026-11-08T11:55:00Z",
      fxRate: null,
      fxSource: null,
      fxObservedAt: null,
      dataStatus: "priced",
    },
    {
      assetId: "doge",
      symbol: "DOGE",
      quantity: "1000",
      value: null,
      allocation: null,
      targetPercent: 40,
      driftPoints: null,
      price: null,
      priceSource: null,
      priceFreshness: "unpriced",
      priceObservedAt: null,
      fxRate: null,
      fxSource: null,
      fxObservedAt: null,
      dataStatus: "unpriced",
    },
  ],
};

const pnl: Pnl = {
  asOf: "2026-11-08T12:00:00Z",
  netInvested: null,
  realized: null,
  unrealized: null,
  coverage: 0.5,
  currency: "USD",
  fxAsOf: null,
  fxEstimated: true,
  marksUnavailable: true,
  holdingsUnavailable: false,
  fxUnavailable: false,
  coverageGap: ["DOGE"],
};

const profile: Profile = {
  userId: "user-1",
  email: "mai@example.com",
  displayName: "Mai",
  mfaRequired: true,
  mfaEnrolled: false,
};

const sessions: SessionInfo[] = [];

const brief: Brief = {
  id: "brief-1",
  period: "daily",
  periodStart: "2026-11-08T00:00:00Z",
  periodEnd: "2026-11-09T00:00:00Z",
  status: "generated",
  failureReason: null,
  asOf: "2026-11-08T12:00:00Z",
  dataStatus: "complete",
  sections: [{ title: "Valuation", kind: "valuation", lines: valuation.lines }],
  generatedAt: "2026-11-08T12:05:00Z",
};

async function check(element: ReactElement, route: string) {
  const { container } = renderWithProviders(<main>{element}</main>, { route });
  await expectNoAxeViolations(container);
  // Several surfaces are checked in one test; clear each render so a later
  // scan does not see two main landmarks from two pages.
  cleanup();
}

describe("core flows meet WCAG 2.2 AA automated checks", () => {
  it("sign-in and verification", async () => {
    await check(<SignInPage sessionOverride={{ status: "signed-out", session: null }} />, "/");
    await check(<VerifyEmailPage />, "/verify-email");
  });

  it("onboarding and connect", async () => {
    await check(<OnboardingPage />, "/onboarding");
    stubGateway({
      "GET /v1/users/user-1/accounts": () => ({ body: [] }),
    });
    await check(<ConnectPage userId="user-1" accountsOverride={[]} />, "/connect");
  });

  it("dashboard, portfolios and detail", async () => {
    stubGateway({ "GET /v1/portfolios": () => ({ body: [] }) });
    await check(<DashboardPage />, "/dashboard");
    await check(<PortfoliosPage />, "/portfolios");

    stubGateway({
      "GET /v1/portfolios/p1": () => ({ body: valuation }),
      "GET /v1/portfolios": () => ({
        body: [{ id: "p1", name: "Main", baseCurrency: "USD" }],
      }),
      "GET /v1/users/user-1/accounts": () => ({ body: [] }),
    });
    await check(<PortfolioDetailPage portfolioId="p1" userId="user-1" />, "/portfolios/p1");
  });

  it("targets, intents, trades and history", async () => {
    await check(
      <TargetsPage portfolioId="p1" valuationOverride={valuation} />,
      "/portfolios/p1/targets",
    );
    await check(
      <IntentsPage portfolioId="p1" valuationOverride={valuation} />,
      "/portfolios/p1/intents",
    );
    await check(
      <TradesPage portfolioId="p1" valuationOverride={valuation} />,
      "/portfolios/p1/trades",
    );
    await check(<HistoryPage snapshotsOverride={[]} />, "/portfolios/p1/history");
  });

  it("activity, briefs and notifications", async () => {
    await check(
      <ActivityPage userId="user-1" transactionsOverride={[]} pnlOverride={pnl} />,
      "/activity",
    );
    await check(<BriefsPage userId="user-1" briefsOverride={[]} />, "/briefs");
    await check(
      <BriefDetailPage userId="user-1" briefId="brief-1" briefOverride={brief} />,
      "/briefs/brief-1",
    );
    await check(
      <NotificationsPage notificationsOverride={[]} preferencesOverride={{ types: [] }} />,
      "/notifications",
    );
  });

  it("settings, MFA, privacy and help", async () => {
    await check(
      <SettingsPage profileOverride={profile} sessionsOverride={sessions} />,
      "/settings",
    );
    await check(<MfaPage profileOverride={profile} />, "/settings/mfa");
    await check(<PrivacyPage consentOverride={[]} />, "/privacy");
    await check(<HelpPage articlesOverride={[]} incidentsOverride={[]} />, "/help");
  });

  it("a failed source degrades its panel without losing the page structure", async () => {
    stubGateway({
      "GET /v1/portfolios": () => ({ status: 500, body: problem(500, "outage") }),
    });
    const { container } = renderWithProviders(
      <main>
        <DashboardPage />
      </main>,
      { route: "/dashboard" },
    );
    await expectNoAxeViolations(container);
    cleanup();
  });
});
