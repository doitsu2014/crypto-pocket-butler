/**
 * The sign-up gate screen and its submit state machine (functional-spec
 * §2.2–§2.6; frontend-components §2/§3; NFR25, NFR26).
 *
 * The screen owns no routing decision of its own: it renders the entry status
 * it is handed, the form while gates are outstanding, and the outcome of the
 * one submit. `ready → submitting → {passed, blocked, error}` — a 422 stays on
 * the form and re-renders from G2's `missing` list, because G2 is the only
 * source of server-named items.
 *
 * Requirements: FR19.2, FR19.3, FR19.4, FR19.5, NFR2.2, NFR25.1, NFR25.2, NFR26.1.
 */

import { useState } from "react";
import { Navigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { signOut, useSession } from "../../auth/session";
import { useI18n } from "../../i18n/react";
import { BlockedJurisdictionNotice } from "./BlockedJurisdictionNotice";
import { CONSENT_KINDS, GATE_DESTINATIONS, type GateFormValues } from "./gateStatus";
import { GateErrorNotice } from "./GateErrorNotice";
import { GateForm } from "./GateForm";
import { GateFullPageRetry } from "./GateFullPageRetry";
import { gateSessionKey, useGateStatus, type GateReader } from "./useGateStatus";
import { useSubmitGates, type GateSubmitter } from "./useSubmitGates";

/** The structural client the screen needs; a real `GatewayClient` satisfies it. */
export type GateClient = GateReader & GateSubmitter;

type Notice =
  | { kind: "none" }
  | { kind: "blocked" }
  | { kind: "error"; variant: "nothing-saved" | "incomplete"; correlationId?: string };

export interface SignupGatesScreenProps {
  /** Overrides the shared gateway client (tests); production uses the default. */
  client?: GateClient;
}

export function SignupGatesScreen({ client }: SignupGatesScreenProps) {
  const { t } = useI18n();
  const session = useSession();
  const sessionUserId = session.session?.user?.id ?? "";
  const status = useGateStatus({ client });
  const { submit, isPending } = useSubmitGates({ sessionUserId, client });
  const queryClient = useQueryClient();
  const [notice, setNotice] = useState<Notice>({ kind: "none" });
  const [lastValues, setLastValues] = useState<GateFormValues | null>(null);

  async function submitValues(values: GateFormValues): Promise<void> {
    setLastValues(values);
    setNotice({ kind: "none" });
    const outcome = await submit(values);
    switch (outcome.kind) {
      case "blocked":
        setNotice({ kind: "blocked" });
        break;
      case "nothing-saved":
        setNotice({
          kind: "error",
          variant: "nothing-saved",
          correlationId: outcome.correlationId,
        });
        break;
      case "incomplete":
        setNotice({ kind: "error", variant: "incomplete", correlationId: outcome.correlationId });
        break;
      case "unasserted":
        // G2 has been re-read; the form re-renders from its `missing` list.
        setNotice({ kind: "none" });
        break;
      case "passed":
        break;
    }
  }

  if (status.kind === "loading") {
    return (
      <p
        data-testid="gates-loading"
        role="status"
        aria-live="polite"
        className="animate-pulse text-small text-muted"
      >
        {t("gates.loading")}
      </p>
    );
  }

  if (status.kind === "owned-elsewhere") {
    return <Navigate to={GATE_DESTINATIONS[status.destination]} replace />;
  }

  if (status.kind === "error") {
    return (
      <GateFullPageRetry
        onRetry={() => {
          void queryClient.invalidateQueries({ queryKey: gateSessionKey(sessionUserId) });
        }}
      />
    );
  }

  if (status.kind === "jurisdiction-blocked") {
    return (
      <BlockedJurisdictionNotice
        continuedUse={status.continuedUse}
        onSignOut={() => void signOut()}
      />
    );
  }

  if (status.kind === "passed") {
    return <Navigate to="/dashboard" replace />;
  }

  if (notice.kind === "blocked") {
    return <BlockedJurisdictionNotice continuedUse={false} onSignOut={() => void signOut()} />;
  }

  const serverMissing = status.serverMissing;
  // "No record" also answers with an empty `serverMissing`, but nothing has been
  // recorded yet: every acceptance starts unchecked and enabled (spec §2.1).
  const recorded = status.noRecord
    ? []
    : CONSENT_KINDS.filter((kind) => !serverMissing.includes(kind));

  return (
    <section className="mx-auto max-w-xl rounded-lg border border-border bg-surface p-8">
      <h1 className="font-display text-section font-medium text-ink">{t("gates.title")}</h1>
      <p className="mt-2 text-body text-muted">{t("gates.intro")}</p>

      {notice.kind === "error" ? (
        <div className="mt-4">
          <GateErrorNotice
            variant={notice.variant}
            correlationId={notice.correlationId}
            onRetry={() => {
              if (lastValues) {
                void submitValues(lastValues);
              }
            }}
          />
        </div>
      ) : null}

      <GateForm
        onSubmit={(values) => void submitValues(values)}
        isSubmitting={isPending}
        serverMissing={serverMissing}
        recorded={recorded}
      />
    </section>
  );
}
