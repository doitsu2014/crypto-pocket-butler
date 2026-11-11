/**
 * The signup gate form: country + three acceptances (functional-spec §2.2/§2.3;
 * frontend-components §2; NFR25).
 *
 * Validation runs on submit only, and Continue is never disabled by validation
 * — only by an in-flight submission. Missing items are merged from the local
 * fields and `serverMissing` in field order, deduplicated with the server's
 * answer winning, and the summary is always the first thing in the DOM.
 *
 * Requirements: FR19.3, FR19.4, NFR25.1, NFR25.2, NFR25.3, NFR26.1.
 */

import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useI18n } from "../../i18n/react";
import { AcceptanceCheckbox } from "./AcceptanceCheckbox";
import { CountrySelect } from "./CountrySelect";
import { MissingAcceptanceSummary } from "./MissingAcceptanceSummary";
import {
  CONSENT_KINDS,
  FIELD_ORDER,
  type ClientMissing,
  type GateFormValues,
  type ServerMissing,
} from "./gateStatus";

export interface GateFormProps {
  onSubmit: (values: GateFormValues) => void;
  isSubmitting: boolean;
  serverMissing?: ServerMissing[];
  recorded?: ServerMissing[];
}

export function GateForm({
  onSubmit,
  isSubmitting,
  serverMissing = [],
  recorded = [],
}: GateFormProps) {
  const { t } = useI18n();
  const [countryCode, setCountryCode] = useState("");
  const [checked, setChecked] = useState<Record<ServerMissing, boolean>>({
    terms: false,
    risk: false,
    age: false,
  });
  const [submitted, setSubmitted] = useState(false);

  const recordedSet = useMemo(() => new Set<ServerMissing>(recorded), [recorded]);
  const required = useMemo(
    () => CONSENT_KINDS.filter((kind) => !recordedSet.has(kind)),
    [recordedSet],
  );

  // A changed server answer (a 422, or a returned visitor) is authoritative:
  // a box it still names as outstanding is cleared locally so the summary and
  // the field agree. An unchanged answer leaves local input untouched, which is
  // what keeps the values on a 503.
  const serverSignature = serverMissing.join("|");
  useEffect(() => {
    if (!serverSignature) {
      return;
    }
    const outstanding = new Set(serverSignature.split("|") as ServerMissing[]);
    setChecked((current) => {
      const next = { ...current };
      let changed = false;
      for (const kind of CONSENT_KINDS) {
        if (outstanding.has(kind) && next[kind]) {
          next[kind] = false;
          changed = true;
        }
      }
      return changed ? next : current;
    });
  }, [serverSignature]);

  const missing = useMemo<ClientMissing[]>(() => {
    const found = new Set<ClientMissing>();
    serverMissing.forEach((kind) => {
      // The server's snapshot names what it had not recorded at read time; a
      // box the person has since ticked is no longer outstanding locally.
      if (!recordedSet.has(kind) && !checked[kind]) {
        found.add(kind);
      }
    });
    if (submitted) {
      if (!countryCode) {
        found.add("country");
      }
      required.forEach((kind) => {
        if (!checked[kind]) {
          found.add(kind);
        }
      });
    }
    return FIELD_ORDER.filter((kind) => found.has(kind));
  }, [serverMissing, recordedSet, submitted, countryCode, required, checked]);

  function handleSubmit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    setSubmitted(true);

    const stillMissing = FIELD_ORDER.filter((kind) => {
      if (kind === "country") {
        return !countryCode;
      }
      return required.includes(kind) && !checked[kind];
    });
    if (stillMissing.length > 0) {
      return;
    }
    // A recorded acceptance is displayed as given, so the payload says true for
    // it even though the visitor never ticked the (read-only) box.
    const accepted = (kind: ServerMissing) => (recordedSet.has(kind) ? true : checked[kind]);
    onSubmit({
      countryCode,
      terms: accepted("terms"),
      risk: accepted("risk"),
      age: accepted("age"),
    });
  }

  function errorFor(kind: ClientMissing): string | undefined {
    return missing.includes(kind) ? t(`gates.${kind}.required`) : undefined;
  }

  return (
    <form data-testid="gates-form" noValidate aria-busy={isSubmitting} onSubmit={handleSubmit}>
      <MissingAcceptanceSummary missing={missing} />
      <fieldset className="mt-4 flex flex-col gap-4">
        <legend className="text-overline text-meta">{t("gates.form.legend")}</legend>
        <CountrySelect
          value={countryCode}
          onChange={setCountryCode}
          error={errorFor("country")}
          labelId="gates-country-label"
        />
        {CONSENT_KINDS.map((kind) => {
          const isRecorded = recordedSet.has(kind);
          return (
            <AcceptanceCheckbox
              key={kind}
              acceptance={kind}
              checked={checked[kind]}
              readOnly={isRecorded}
              onChange={(next) => setChecked((current) => ({ ...current, [kind]: next }))}
              documentHref={kind === "age" ? undefined : t(`gates.${kind}.documentHref`)}
              error={isRecorded ? undefined : errorFor(kind)}
            />
          );
        })}
      </fieldset>
      <div className="mt-6 flex flex-col gap-2">
        <button
          type="submit"
          data-testid="gates-continue"
          disabled={isSubmitting}
          className="rounded-xl bg-accent px-4 py-2 text-small font-medium text-accent-on"
        >
          {t("gates.continue")}
        </button>
        {isSubmitting ? (
          <p role="status" className="text-small text-muted">
            {t("gates.submitting")}
          </p>
        ) : null}
      </div>
    </form>
  );
}
