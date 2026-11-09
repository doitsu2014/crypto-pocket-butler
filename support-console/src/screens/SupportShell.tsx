import { useQueryClient } from "@tanstack/react-query";
import { lazy, Suspense, useEffect, useState } from "react";
import type { SupportAccessGrant } from "../api/types";
import { clearSupportCache } from "../api/queryClient";
import { clearSupportSession, type SupportRole, type SupportSession } from "../auth/session";
import { AccountMetadataPanel } from "../components/AccountMetadataPanel";
import { AccountSearch } from "../components/AccountSearch";
import { GrantApprovalQueue } from "../components/GrantApprovalQueue";
import { GrantBanner } from "../components/GrantBanner";
import { GrantRequestForm } from "../components/GrantRequestForm";
import { HelpPanel } from "../components/HelpPanel";
import { StatusFooter } from "../components/StatusFooter";
import { TicketQueue } from "../components/TicketQueue";
import { canReadWithGrant, grantStatusAt } from "../domain/grants";
import { useGrantClock } from "../hooks/useGrantClock";
import { calloutClass, linkClass, overlineClass, secondaryButtonClass } from "../components/ui";

const IncidentConsole = lazy(() => import("../components/IncidentConsole"));

export type SectionId = "grants" | "search" | "tickets" | "incidents" | "help";

interface SectionDefinition {
  id: SectionId;
  label: string;
  adminOnly?: boolean;
}

const SECTIONS: SectionDefinition[] = [
  { id: "grants", label: "Grants" },
  { id: "search", label: "Account search" },
  { id: "tickets", label: "Tickets" },
  { id: "incidents", label: "Incidents", adminOnly: true },
  { id: "help", label: "Help" },
];

function isSectionId(value: string): value is SectionId {
  return SECTIONS.some((section) => section.id === value);
}

function readSectionFromHash(): SectionId {
  const raw = window.location.hash.replace(/^#\/?/, "");
  return isSectionId(raw) ? raw : "grants";
}

export interface SupportShellProps {
  session: SupportSession;
  role: SupportRole;
  onSignOut: () => void;
  initialGrant?: SupportAccessGrant | null;
  initialPendingGrants?: SupportAccessGrant[];
  initialSection?: SectionId;
}

export function SupportShell({
  session,
  role,
  onSignOut,
  initialGrant = null,
  initialPendingGrants = [],
  initialSection,
}: SupportShellProps) {
  const queryClient = useQueryClient();
  const [grant, setGrant] = useState<SupportAccessGrant | null>(initialGrant);
  const [pendingGrants, setPendingGrants] = useState<SupportAccessGrant[]>(initialPendingGrants);
  const [section, setSection] = useState<SectionId>(initialSection ?? readSectionFromHash);
  const [selectedAccountId, setSelectedAccountId] = useState<string | null>(null);

  const clock = useGrantClock(grant?.expiresAt ?? null);
  const status = grant ? grantStatusAt(grant, clock) : null;
  const hasActiveGrant = Boolean(grant && canReadWithGrant(grant, clock));
  const grantExpired = Boolean(grant && grant.status === "active" && !hasActiveGrant);

  useEffect(() => {
    const onHashChange = () => setSection(readSectionFromHash());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  useEffect(() => {
    if (grantExpired) {
      clearSupportCache(queryClient);
      setSelectedAccountId(null);
    }
  }, [grantExpired, queryClient]);

  const visibleSection: SectionId =
    section === "incidents" && role !== "admin" ? "grants" : section;

  function signOut() {
    clearSupportCache(queryClient);
    clearSupportSession();
    onSignOut();
  }

  function onGrantChanged(updated: SupportAccessGrant) {
    setPendingGrants((current) => current.map((item) => (item.id === updated.id ? updated : item)));
    if (updated.status === "active") {
      setGrant(updated);
    }
  }

  function onAccessEnded() {
    setGrant((current) => (current ? { ...current, status: "expired" } : current));
    setSelectedAccountId(null);
  }

  return (
    <div className="min-h-screen bg-parchment text-ink">
      <header className="border-b border-border-soft bg-surface">
        <div className="mx-auto flex w-full max-w-5xl flex-wrap items-center justify-between gap-3 px-6 py-4">
          <div>
            <p className={overlineClass}>Crypto Pocket Butler</p>
            <h1 className="font-display text-kpi font-medium leading-display text-ink">
              Support console
            </h1>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <p className="text-small text-ink-2">
              Signed in as <span className="font-medium text-ink">{role}</span>
              {session.user.email ? <span className="text-meta"> · {session.user.email}</span> : null}
            </p>
            <button
              type="button"
              data-testid="support-shell-sign-out"
              onClick={signOut}
              className={secondaryButtonClass}
            >
              Sign out
            </button>
          </div>
        </div>
        <nav aria-label="Support console sections" className="mx-auto w-full max-w-5xl px-6">
          <ul className="flex flex-wrap gap-4">
            {SECTIONS.filter((item) => !item.adminOnly || role === "admin").map((item) => (
              <li key={item.id}>
                <a
                  href={`#${item.id}`}
                  data-testid={`nav-${item.id}`}
                  aria-current={visibleSection === item.id ? "page" : undefined}
                  onClick={() => setSection(item.id)}
                  className={`inline-block border-b-2 py-3 text-small font-medium ${
                    visibleSection === item.id
                      ? "border-accent text-ink"
                      : "border-transparent text-muted"
                  }`}
                >
                  {item.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>
      </header>

      <main className="mx-auto flex w-full max-w-5xl flex-col gap-4 px-6 py-8">
        {grant ? (
          <GrantBanner grant={grant} now={clock} onRequestNew={() => setSection("grants")} />
        ) : (
          <p data-testid="support-shell-no-grant" className={calloutClass}>
            No support grant is active. Request one, or approve a pending request, before reading
            account metadata.
          </p>
        )}

        {grant && status !== "active" ? (
          <p className="text-small text-meta">
            Reads are disabled while the grant is {status === "pending_approval" ? "pending approval" : status}.
          </p>
        ) : null}

        {visibleSection === "grants" ? (
          <div className="flex flex-col gap-4">
            <GrantRequestForm
              requesterId={session.user.id}
              token={session.accessToken}
              onCreated={(created) => setPendingGrants((current) => [created, ...current])}
            />
            <GrantApprovalQueue
              grants={pendingGrants}
              currentUserId={session.user.id}
              role={role}
              token={session.accessToken}
              onChanged={onGrantChanged}
            />
          </div>
        ) : null}

        {visibleSection === "search" ? (
          <div className="flex flex-col gap-4">
            <AccountSearch
              grantId={hasActiveGrant && grant ? grant.id : ""}
              token={session.accessToken}
              onSelect={setSelectedAccountId}
            />
            {selectedAccountId && grant ? (
              <AccountMetadataPanel
                accountId={selectedAccountId}
                grantId={grant.id}
                token={session.accessToken}
                defaultReason={grant.reason}
                expiresAt={grant.expiresAt}
                onAccessEnded={onAccessEnded}
              />
            ) : null}
          </div>
        ) : null}

        {visibleSection === "tickets" ? (
          <TicketQueue
            grantId={hasActiveGrant && grant ? grant.id : ""}
            token={session.accessToken}
          />
        ) : null}

        {visibleSection === "incidents" && role === "admin" ? (
          <Suspense fallback={<p role="status">Loading incident console…</p>}>
            <IncidentConsole role={role} token={session.accessToken} />
          </Suspense>
        ) : null}

        {visibleSection === "help" ? <HelpPanel /> : null}

        <p className="text-small text-muted">
          <a href="#help" className={linkClass} onClick={() => setSection("help")}>
            Read the help and contact guidance
          </a>
        </p>
      </main>

      <StatusFooter />
    </div>
  );
}
