import { useI18n } from "../../i18n/react";
import type { DeletionView } from "../../api/types";

/**
 * The open-request notice, rendered identically by the delete page and the
 * restore screen (functional-spec §3.5/§3.6). It names the request date and the
 * exact `recoveryEndsAt`, lists the retained categories only when the server
 * supplied some, and otherwise shows the placeholder policy sentence.
 */
export function OpenRequestNotice({
  request,
  onRestore,
  pending = false,
  error,
}: {
  request: DeletionView;
  onRestore: () => void;
  pending?: boolean;
  error?: string | null;
}) {
  const { t, format } = useI18n();
  return (
    <div data-testid="deletion-open-request" className="grid gap-3">
      <p className="text-small text-ink-2">
        {t("settings.delete.requestedOn", { date: format(request.requestedAt) })}
      </p>
      <p data-testid="deletion-recovery-date" className="text-small text-ink">
        {t("settings.delete.recoveryOn", { date: format(request.recoveryEndsAt) })}
      </p>

      <div className="grid gap-1">
        <h2 className="text-small font-medium text-ink">{t("settings.delete.retainedTitle")}</h2>
        {request.retainedCategories.length > 0 ? (
          <ul data-testid="deletion-retained" className="grid gap-1 text-small text-ink-2">
            {request.retainedCategories.map((category) => (
              <li key={category.category}>
                {category.category} · {category.basis}
              </li>
            ))}
          </ul>
        ) : (
          <p data-testid="deletion-retained-policy" className="max-w-prose text-small text-muted">
            {t("settings.delete.retained")}
          </p>
        )}
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          data-testid="deletion-restore"
          disabled={pending}
          onClick={onRestore}
          className="w-fit rounded-lg bg-accent px-4 py-2 text-small font-medium text-accent-on disabled:opacity-45"
        >
          {t("settings.restore.action")}
        </button>
      </div>

      {error ? (
        <p role="alert" data-testid="deletion-restore-error" className="text-small text-danger">
          {error}
        </p>
      ) : null}
    </div>
  );
}
