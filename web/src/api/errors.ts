import { ApiError } from "./client";

/**
 * Flattens a gateway problem into field-level messages for a form. The server
 * detail is used when no structured `errors` array is present.
 */
export function problemMessages(error: unknown): string[] {
  if (error instanceof ApiError) {
    if (error.problem.errors && error.problem.errors.length > 0) {
      return error.problem.errors.map(
        (entry) => `${entry.field}: ${entry.reason}`,
      );
    }
    return [error.problem.detail ?? error.problem.title];
  }
  if (error instanceof Error) {
    return [error.message];
  }
  return ["Unknown error"];
}

/** The first field error for a named field, if the gateway supplied one. */
export function fieldError(error: unknown, field: string): string | null {
  if (error instanceof ApiError) {
    const match = error.problem.errors?.find((entry) => entry.field === field);
    if (match) {
      return match.reason;
    }
  }
  return null;
}
