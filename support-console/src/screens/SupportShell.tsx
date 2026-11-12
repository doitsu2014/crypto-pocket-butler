import { useQueryClient } from "@tanstack/react-query";
import { lazy, Suspense, useEffect, useState } from "react";
import { clearSupportCache } from "../api/queryClient";
import type { SupportAccessGrant } from "../api/types";
import { clearSupportSession, type SupportRole, type SupportSession } from "../auth/session";
import { GrantBanner } from "../components/GrantBanner";
import { GrantRequestForm } from "../components/GrantRequestForm";
import { HelpPanel } from "../components/HelpPanel";
import { StatusFooter } from "../components/StatusFooter";
import { SupportMetadataPanel } from "../components/SupportMetadataPanel";
import { TicketQueue } from "../components/TicketQueue";
import { calloutClass, linkClass, overlineClass, secondaryButtonClass } from "../components/ui";
import { canReadWithGrant, grantStatusAt } from "../domain/grants";
import { useGrantClock } from "../hooks/useGrantClock";
import { useMyGrants } from "../hooks/useMyGrants";

const IncidentConsole = lazy(() => import("../components/IncidentConsole"));
const AdminUsersSection = lazy(() => import("./admin/AdminUsersSection"));

export type SectionId = "grants" | "metadata" | "tickets" | "incidents" | "admin-users" | "help";

interface SectionDefinition {
  id: SectionId;
  label: string;
  adminOnly?: boolean;
}

const SECTIONS: SectionDefinition[] = [
  { id: "grants", label: "Grants" },
  { id: "metadata", label: "Metadata" },
  { id: "tickets", label: "Tickets" },
  { id: "incidents", label: "Incidents", adminOnly: true },
  { id: "admin-users", label: "Admin users", adminOnly: true },
  { id: "help", label: "Help" },
];

/** `search` was the old name of the metadata section; old links still land. */
function normalizeSection(value: string): SectionId | null {
  if (value === "search") {
    return "metadata";
  }
  return SECTIONS.some((section) => section.id === value) ? (value as SectionId) : null;
}

function readSectionFromHash(): SectionId {
  const raw = window.location.hash.replace(/^#\/?/, "");
  return normalizeSection(raw) ?? "grants";
}

export interface SupportShellProps {
  session: SupportSession;
  role: SupportRole;
  onSignOut: () => void;
  initialGrant?: SupportAccessGrant | null;
  initialSection?: SectionId;
}

export function SupportShell({
  session,
  role,
  onSignOut,
  initialGrant = null,
  initialSection,
}: SupportShellProps) {
  const queryClient = useQueryClient();
  const [grant, setGrant] = useState<SupportAccessGrant | null>(initialGrant);
  const [section, setSection] = useState<SectionId>(
    initialSection ? (normalizeSection(initialSection) ?? "grants") : readSectionFromHash(),
  );

  const mine = useMyGrants(session.accessToken, role);

  const clock = useGrantClock(grant?.expiresAt ?? null);
  const status = grant ? grantStatusAt(grant, clock) : null;
  const hasActiveGrant = Boolean(grant && canReadWithGrant(grant, clock));

  // The gateway already scopes `mine` to the caller, so the newest entry is the
  // session's grant. It is adopted unconditionally: `requestedBy` is Identity's
  // internal id, never the Supabase auth id, so comparing it would reject every
  // grant. A grant adopted from a 201 is never replaced by a stale list.
  useEffect(() => {
    if (grant === null && mine.data && mine.data.length > 0) {
      setGrant(mine.data[0]);
    }
  }, [grant, mine.data]);

  useEffect(() => {
    const onHashChange = () => setSection(readSectionFromHash());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  useEffect(() => {
    if (grant && grant.status === "active" && !hasActiveGrant) {
      clearSupportCache(queryClient);
    }
  }, [grant, hasActiveGrant, queryClient]);

  const visibleSection: SectionId =
    SECTIONS.find((item) => item.id === section)?.adminOnly && role !== "admin"
      ? "grants"
      : section;

  function signOut() {
    clearSupportCache(queryClient);
    clearSupportSession();
    onSignOut();
  }

  /** Only a grant already on screen is updated; another agent's approval never adopts. */
  function onGrantChanged(updated: SupportAccessGrant) {
    setGrant((current) => (current && current.id === updated.id ? updated : current));
  }

  function onAccessEnded() {
    setGrant((current) => (current ? { ...current, status: "expired" } : current));
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
              {session.user.email ? (
                <span className="text-meta"> · {session.user.email}</span>
              ) : null}
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
          <GrantBanner
            grant={grant}
            now={clock}
            onCheckStatus={() => setSection("grants")}
            onRequestNew={() => setSection("grants")}
          />
        ) : (
          <p data-testid="support-shell-no-grant" className={calloutClass}>
            No support grant is active. Request one, or approve a pending request, before reading
            account metadata.
          </p>
        )}

        {grant && status !== "active" ? (
          <p className="text-small text-meta">
            Reads are disabled while the grant is{" "}
            {status === "pending_approval" ? "pending approval" : status}.
          </p>
        ) : null}

        {visibleSection === "grants" ? (
          role === "admin" ? (
            <p data-testid="admin-link" className={calloutClass}>
              Administrators approve pending grants and manage users from the{" "}
              <a
                href="#admin-users"
                className={linkClass}
                onClick={() => setSection("admin-users")}
              >
                Admin users
              </a>{" "}
              section.
            </p>
          ) : (
            <GrantRequestForm
              token={session.accessToken}
              onCreated={(created) => setGrant(created)}
              onCheckStatus={() => setSection("grants")}
            />
          )
        ) : null}

        {visibleSection === "metadata" ? (
          grant ? (
            <SupportMetadataPanel
              grant={grant}
              token={session.accessToken}
              status={status}
              onAccessEnded={onAccessEnded}
            />
          ) : (
            <p data-testid="metadata-grant-required" className={calloutClass}>
              A support grant is required to read account and portfolio metadata.
            </p>
          )
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

        {visibleSection === "admin-users" && role === "admin" ? (
          <Suspense fallback={<p role="status">Loading admin users…</p>}>
            <AdminUsersSection
              token={session.accessToken}
              role={role}
              currentUserId={session.user.id}
              signedInEmail={session.user.email}
              onGrantChanged={onGrantChanged}
            />
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
