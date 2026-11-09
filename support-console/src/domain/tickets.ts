import type { StatusPresentation } from "./presentation";

/** Ticket states are a small closed set in the C2 metadata contract. */
export function ticketStatusPresentation(status: string): StatusPresentation {
  switch (status.toLowerCase()) {
    case "open":
      return { label: "Open", tone: "warn", shape: "diamond" };
    case "pending":
      return { label: "Pending", tone: "warn", shape: "ring" };
    case "resolved":
      return { label: "Resolved", tone: "success", shape: "dot" };
    default:
      return { label: status || "Unknown", tone: "neutral", shape: "dot" };
  }
}
