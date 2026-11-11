import { lazy, Suspense, type ReactNode } from "react";
import { createBrowserRouter, Navigate } from "react-router-dom";
import { Loading } from "../components";
import SignInPage from "../features/auth/SignInPage";
import VerifyEmailPage from "../features/auth/VerifyEmailPage";
import { GateFullPageRetry } from "../features/signup-gates/GateFullPageRetry";
import { AppShell } from "./AppShell";
import { RequireGatesPassed } from "./RequireGatesPassed";
import { RequireSession } from "./RequireSession";

/**
 * The SPA router (functional spec: journeys and routes).
 *
 * The shell and sign-in stay in the entry chunk; every signed-in journey is a
 * lazily loaded route chunk (performance design: code splitting).
 */

const SignupGatesPage = lazy(() => import("../features/signup-gates/SignupGatesPage"));
const OnboardingPage = lazy(() => import("../features/onboarding/OnboardingPage"));
const ConnectPage = lazy(() => import("../features/connect/ConnectPage"));
const DashboardPage = lazy(() => import("../features/portfolio/DashboardPage"));
const PortfoliosPage = lazy(() => import("../features/portfolio/PortfoliosPage"));
const PortfolioDetailPage = lazy(() => import("../features/portfolio/PortfolioDetailPage"));
const TargetsPage = lazy(() => import("../features/portfolio/TargetsPage"));
const IntentsPage = lazy(() => import("../features/portfolio/IntentsPage"));
const TradesPage = lazy(() => import("../features/portfolio/TradesPage"));
const HistoryPage = lazy(() => import("../features/portfolio/HistoryPage"));
const ActivityPage = lazy(() => import("../features/activity/ActivityPage"));
const BriefsPage = lazy(() => import("../features/briefs/BriefsPage"));
const BriefDetailPage = lazy(() => import("../features/briefs/BriefDetailPage"));
const NotificationsPage = lazy(() => import("../features/notifications/NotificationsPage"));
const SettingsPage = lazy(() => import("../features/settings/SettingsPage"));
const MfaPage = lazy(() => import("../features/settings/MfaPage"));
const PrivacyPage = lazy(() => import("../features/privacy/PrivacyPage"));
const HelpPage = lazy(() => import("../features/help/HelpPage"));

function page(node: ReactNode): ReactNode {
  return <Suspense fallback={<Loading />}>{node}</Suspense>;
}

function protectedPage(node: ReactNode): ReactNode {
  return <RequireSession>{page(node)}</RequireSession>;
}

/**
 * A post-sign-in product route: signed in, and only reachable once every
 * sign-up gate has passed on the server (AC6.2.2, AC6.5.2). `/settings/mfa`
 * stays outside the guard because it is itself an `owned-elsewhere`
 * destination.
 */
function gatedPage(node: ReactNode): ReactNode {
  return <RequireSession>{<RequireGatesPassed>{page(node)}</RequireGatesPassed>}</RequireSession>;
}

export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <SignInPage /> },
      { path: "verify-email", element: <VerifyEmailPage /> },
      {
        path: "gates",
        element: protectedPage(<SignupGatesPage />),
        errorElement: <GateFullPageRetry onRetry={() => window.location.reload()} />,
      },
      { path: "onboarding", element: gatedPage(<OnboardingPage />) },
      { path: "connect", element: gatedPage(<ConnectPage />) },
      { path: "dashboard", element: gatedPage(<DashboardPage />) },
      { path: "portfolios", element: gatedPage(<PortfoliosPage />) },
      {
        path: "portfolios/:portfolioId",
        element: gatedPage(<PortfolioDetailPage />),
      },
      {
        path: "portfolios/:portfolioId/targets",
        element: gatedPage(<TargetsPage />),
      },
      {
        path: "portfolios/:portfolioId/intents",
        element: gatedPage(<IntentsPage />),
      },
      {
        path: "portfolios/:portfolioId/trades",
        element: gatedPage(<TradesPage />),
      },
      {
        path: "portfolios/:portfolioId/history",
        element: gatedPage(<HistoryPage />),
      },
      { path: "activity", element: gatedPage(<ActivityPage />) },
      { path: "briefs", element: gatedPage(<BriefsPage />) },
      {
        path: "briefs/:briefId",
        element: gatedPage(<BriefDetailPage />),
      },
      {
        path: "notifications",
        element: gatedPage(<NotificationsPage />),
      },
      { path: "settings", element: gatedPage(<SettingsPage />) },
      { path: "settings/mfa", element: protectedPage(<MfaPage />) },
      { path: "privacy", element: gatedPage(<PrivacyPage />) },
      { path: "help", element: gatedPage(<HelpPage />) },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);
