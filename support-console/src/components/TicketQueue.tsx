import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { ApiError, listTickets } from "../api/client";
import { isForbiddenError, isUnauthorizedError } from "../api/queryClient";
import { ticketStatusPresentation } from "../domain/tickets";
import { useClearSupportCacheOnErrors } from "../hooks/useClearSupportCacheOnErrors";
import { StatusBadge } from "./StatusBadge";
import { alertClass, calloutClass, cardClass, overlineClass, sectionTitleClass } from "./ui";

const STATUS_OPTIONS = [
  { value: "", label: "All tickets" },
  { value: "open", label: "Open" },
  { value: "pending", label: "Pending" },
  { value: "resolved", label: "Resolved" },
];

export interface TicketQueueProps {
  grantId: string;
  token: string | null;
}

/**
 * Ticket metadata only, scoped to the active grant's target user. Conversation
 * content and the ticket system's requester label never reach this component
 * (WF5, BR1.4).
 */
export function TicketQueue({ grantId, token }: TicketQueueProps) {
  const [status, setStatus] = useState("");
  const hasGrant = grantId.trim().length > 0;
  const query = useQuery({
    queryKey: ["support", "tickets", grantId, status],
    queryFn: () => listTickets(grantId, status || undefined, token),
    enabled: hasGrant,
  });

  useClearSupportCacheOnErrors(query.error);

  const problem = query.error instanceof ApiError ? query.error.problem : null;

  return (
    <section data-testid="ticket-queue" className={cardClass}>
      <p className={overlineClass}>Support queue</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Tickets</h2>

      {hasGrant ? null : (
        <p data-testid="ticket-queue-grant-required" className={`${calloutClass} mt-4`}>
          An active support grant is required to read the ticket queue.
        </p>
      )}

      <label className="mt-4 flex flex-col gap-1 text-small font-medium text-ink-2" htmlFor="ticket-status-filter">
        Status
        <select
          id="ticket-status-filter"
          data-testid="ticket-status-filter"
          value={status}
          onChange={(event) => setStatus(event.target.value)}
          disabled={!hasGrant}
          className="rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink focus:border-focus focus-visible:shadow-focus focus:outline-none disabled:opacity-45"
        >
          {STATUS_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>

      {query.isLoading ? (
        <p role="status" className="mt-4 text-small text-muted">
          Loading tickets…
        </p>
      ) : null}

      {query.error ? (
        <p role="alert" className={`${alertClass} mt-4`}>
          {isForbiddenError(query.error)
            ? "Access ended. Request a new grant to continue."
            : isUnauthorizedError(query.error)
              ? "Sign in again to continue."
              : (problem?.title ?? query.error.message)}
          {problem ? (
            <span className="ml-2 font-mono text-meta">Correlation ID {problem.correlationId}</span>
          ) : null}
        </p>
      ) : null}

      {query.data && query.data.items.length === 0 ? (
        <p data-testid="ticket-queue-empty" className="mt-4 text-small text-ink-2">
          No tickets match this status.
        </p>
      ) : null}

      {query.data && query.data.items.length > 0 ? (
        <div className="mt-4 overflow-x-auto">
          <table className="w-full border-collapse text-left text-small">
            <caption className="sr-only">Ticket metadata</caption>
            <thead>
              <tr className="text-overline uppercase tracking-overline text-meta">
                <th scope="col" className="py-2 pr-4">
                  Ticket
                </th>
                <th scope="col" className="py-2 pr-4">
                  Subject
                </th>
                <th scope="col" className="py-2 pr-4">
                  Status
                </th>
                <th scope="col" className="py-2">
                  Updated
                </th>
              </tr>
            </thead>
            <tbody>
              {query.data.items.map((ticket) => (
                <tr key={ticket.id} data-testid={`ticket-row-${ticket.id}`} className="border-t border-border-soft">
                  <td className="py-2 pr-4 font-mono text-ink">{ticket.id}</td>
                  <td className="py-2 pr-4 text-ink">{ticket.subject}</td>
                  <td className="py-2 pr-4">
                    <StatusBadge presentation={ticketStatusPresentation(ticket.status)} />
                  </td>
                  <td className="py-2 font-mono text-meta">{ticket.updatedAt}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
    </section>
  );
}
