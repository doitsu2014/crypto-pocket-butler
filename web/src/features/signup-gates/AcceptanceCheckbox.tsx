/**
 * One acceptance checkbox (functional-spec §2.2; frontend-components §2).
 *
 * Terms and risk carry a document link that opens in a new tab with
 * `rel="noopener noreferrer"`; the age variant has no link. A recorded
 * acceptance (arrived from G2, never from local state) renders checked and
 * read-only.
 *
 * Requirements: FR19.4, NFR25.1, NFR25.2, NFR26.1.
 */

import { useI18n } from "../../i18n/react";
import type { ServerMissing } from "./gateStatus";

export interface AcceptanceCheckboxProps {
  acceptance: ServerMissing;
  checked: boolean;
  onChange: (checked: boolean) => void;
  documentHref?: string;
  error?: string;
  readOnly?: boolean;
}

export function AcceptanceCheckbox({
  acceptance,
  checked,
  onChange,
  documentHref,
  error,
  readOnly = false,
}: AcceptanceCheckboxProps) {
  const { t } = useI18n();
  const inputId = `gates-${acceptance}-checkbox`;
  const errorId = `gates-${acceptance}-error`;

  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={inputId} className="flex items-start gap-2 text-small text-ink-2">
        <input
          id={inputId}
          data-testid={inputId}
          type="checkbox"
          className="mt-1"
          checked={readOnly ? true : checked}
          disabled={readOnly}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? errorId : undefined}
          onChange={(event) => onChange(event.target.checked)}
        />
        <span>
          {t(`gates.${acceptance}.label`)}
          {documentHref ? (
            <>
              {" "}
              <a
                href={documentHref}
                target="_blank"
                rel="noopener noreferrer"
                className="underline"
              >
                {t(`gates.${acceptance}.document`)}
              </a>
            </>
          ) : null}
        </span>
      </label>
      {readOnly ? <p className="text-small text-muted">{t("gates.accepted")}</p> : null}
      {error ? (
        <p id={errorId} className="text-small text-danger">
          {error}
        </p>
      ) : null}
    </div>
  );
}
