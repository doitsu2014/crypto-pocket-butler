import { lazy, Suspense, type ReactNode } from "react";
import { createBrowserRouter, Navigate } from "react-router-dom";
import { Loading } from "../components";
import SignInPage from "../features/auth/SignInPage";
import VerifyEmailPage from "../features/auth/VerifyEmailPage";
import { AppShell } from "./AppShell";
import { RequireSession } from "./RequireSession";

/**
 * The SPA router (functional spec: journeys and routes).
 *
 * The shell and sign-in stay in the entry chunk; every signed-in journey is a
 * lazily loaded route chunk (performance design: code splitting).
 */

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

export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <SignInPage /> },
      { path: "verify-email", element: <VerifyEmailPage /> },
      { path: "onboarding", element: protectedPage(<OnboardingPage />) },
      { path: "connect", element: protectedPage(<ConnectPage />) },
      { path: "dashboard", element: protectedPage(<DashboardPage />) },
      { path: "portfolios", element: protectedPage(<PortfoliosPage />) },
      {
        path: "portfolios/:portfolioId",
        element: protectedPage(<PortfolioDetailPage />),
      },
      {
        path: "portfolios/:portfolioId/targets",
        element: protectedPage(<TargetsPage />),
      },
      {
        path: "portfolios/:portfolioId/intents",
        element: protectedPage(<IntentsPage />),
      },
      {
        path: "portfolios/:portfolioId/trades",
        element: protectedPage(<TradesPage />),
      },
      {
        path: "portfolios/:portfolioId/history",
        element: protectedPage(<HistoryPage />),
      },
      { path: "activity", element: protectedPage(<ActivityPage />) },
      { path: "briefs", element: protectedPage(<BriefsPage />) },
      {
        path: "briefs/:briefId",
        element: protectedPage(<BriefDetailPage />),
      },
      {
        path: "notifications",
        element: protectedPage(<NotificationsPage />),
      },
      { path: "settings", element: protectedPage(<SettingsPage />) },
      { path: "settings/mfa", element: protectedPage(<MfaPage />) },
      { path: "privacy", element: protectedPage(<PrivacyPage />) },
      { path: "help", element: protectedPage(<HelpPage />) },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);
