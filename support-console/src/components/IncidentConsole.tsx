import { useState } from "react";
import type { Incident } from "../api/types";
import type { SupportRole } from "../auth/session";
import { IncidentForm } from "./IncidentForm";
import { IncidentList } from "./IncidentList";
import { cardClass, overlineClass, sectionTitleClass } from "./ui";

export interface IncidentConsoleProps {
  role: SupportRole;
  token: string | null;
  initialIncidents?: Incident[];
}

/** BR3.1: only admin declares or updates an incident. */
export function IncidentConsole({ role, token, initialIncidents = [] }: IncidentConsoleProps) {
  const [incidents, setIncidents] = useState<Incident[]>(initialIncidents);

  if (role !== "admin") {
    return (
      <section data-testid="incident-console-admin-only" className={cardClass}>
        <p className={overlineClass}>Restricted</p>
        <h2 className={`${sectionTitleClass} mt-1`}>Admins only</h2>
        <p className="mt-2 text-small text-ink-2">
          Only an admin can declare or update an incident.
        </p>
      </section>
    );
  }

  function onDeclared(incident: Incident) {
    setIncidents((current) =>
      current.some((existing) => existing.id === incident.id)
        ? current.map((existing) => (existing.id === incident.id ? incident : existing))
        : [incident, ...current],
    );
  }

  return (
    <div data-testid="incident-console" className="flex flex-col gap-4">
      <IncidentForm token={token} onDeclared={onDeclared} />
      <IncidentList incidents={incidents} />
    </div>
  );
}

export default IncidentConsole;
