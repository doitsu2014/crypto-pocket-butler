/**
 * The submit-time validation summary (functional-spec §2.3; NFR25.1).
 *
 * Rendered above the fields in DOM order, with `role="alert"`, and it takes
 * focus when it appears. Each entry anchors to the field it names. It renders
 * only when there is something missing.
 *
 * Requirements: FR19.4, NFR25.1, NFR25.3.
 */

import { useEffect, useRef } from "react";
import { useI18n } from "../../i18n/react";
import type { ClientMissing } from "./gateStatus";

const FIELD_HREF: Record<ClientMissing, string> = {
  country: "#gates-country-input",
  terms: "#gates-terms-checkbox",
  risk: "#gates-risk-checkbox",
  age: "#gates-age-checkbox",
};

const MESSAGE_KEY: Record<ClientMissing, string> = {
  country: "gates.country.required",
  terms: "gates.terms.required",
  risk: "gates.risk.required",
  age: "gates.age.required",
};

export function MissingAcceptanceSummary({ missing }: { missing: ClientMissing[] }) {
  const { t } = useI18n();
  const ref = useRef<HTMLDivElement>(null);
  const signature = missing.join("|");

  useEffect(() => {
    if (signature) {
      ref.current?.focus();
    }
  }, [signature]);

  if (missing.length === 0) {
    return null;
  }

  return (
    <div
      ref={ref}
      tabIndex={-1}
      role="alert"
      data-testid="gates-validation-summary"
      className="rounded-lg border border-danger bg-surface p-3 text-danger"
    >
      <p className="text-small font-medium">{t("gates.summary.title")}</p>
      <ul className="mt-1 list-disc pl-4">
        {missing.map((kind) => (
          <li key={kind}>
            <a href={FIELD_HREF[kind]} className="text-small underline">
              {t(MESSAGE_KEY[kind])}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}
