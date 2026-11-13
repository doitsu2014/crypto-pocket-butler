export type StatusTone = "success" | "warn" | "danger" | "neutral";

export type StatusShape = "dot" | "diamond" | "square" | "ring";

export interface StatusPresentation {
  label: string;
  tone: StatusTone;
  shape: StatusShape;
}

/** One home for the copy the grant form, banner, and metadata panel repeat. */
export const READ_AUDIT_STATEMENT = "Every metadata read is recorded by the support API.";

export const GATEWAY_UNREACHABLE_MESSAGE =
  "Could not reach the gateway. Check your connection and try again.";
