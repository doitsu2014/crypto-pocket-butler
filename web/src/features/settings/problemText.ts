/**
 * Status-only rendering of an RFC 7807 problem for the settings surfaces
 * (security-design §5, NFR22.1). The server's `detail` and `title` are never
 * rendered: a mapped, pre-written sentence is shown instead, plus the
 * correlation id so support can find the request. Field errors are the one
 * exception, because they name a field and a reason and carry no payload.
 */

import { ApiError } from "../../api/client";
import type { ProblemDetails } from "../../api/types";
import type { Translate } from "../../i18n";

export function problemText(problem: ProblemDetails, t: Translate): string {
  switch (problem.status) {
    case 401:
      return t("problem.unauthorized");
    case 403:
      return t("problem.forbidden");
    case 404:
      return t("problem.notFound");
    case 409:
      return t("problem.conflict");
    case 410:
      return t("problem.gone");
    case 422:
      return t("problem.validation");
    case 429:
      return t("problem.rateLimited");
    default:
      return t("problem.server");
  }
}

export function problemMessage(error: unknown, t: Translate): string {
  return error instanceof ApiError ? problemText(error.problem, t) : t("problem.network");
}

/** Field-level messages keyed by the server's field name. */
export function fieldErrorsOf(error: unknown): Record<string, string> {
  if (!(error instanceof ApiError)) {
    return {};
  }
  const out: Record<string, string> = {};
  for (const field of error.problem.errors ?? []) {
    out[field.field] = field.reason;
  }
  return out;
}

export function correlationOf(error: unknown): string | null {
  return error instanceof ApiError ? error.problem.correlationId : null;
}

export function isConflict(error: unknown): boolean {
  return error instanceof ApiError && error.problem.status === 409;
}

export function isGone(error: unknown): boolean {
  return error instanceof ApiError && error.problem.status === 410;
}
