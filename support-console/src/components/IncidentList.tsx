import type { Incident } from "../api/types";
import {
  incidentSeverityPresentation,
  incidentStatusPresentation,
  notificationPresentation,
} from "../domain/incidents";
import { StatusBadge } from "./StatusBadge";
import { cardClass, calloutClass, overlineClass, sectionTitleClass } from "./ui";

export function IncidentList({ incidents }: { incidents: Incident[] }) {
  return (
    <section data-testid="incident-list" className={cardClass}>
      <p className={overlineClass}>Incident store</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Incidents</h2>

      {incidents.length === 0 ? (
        <p data-testid="incident-list-empty" className="mt-4 text-small text-ink-2">
          No incidents have been declared.
        </p>
      ) : (
        <ul className="mt-4 flex flex-col gap-3">
          {incidents.map((incident) => (
            <li
              key={incident.id}
              data-testid={`incident-row-${incident.id}`}
              className={`${calloutClass} flex flex-col gap-2`}
            >
              <div className="flex flex-wrap items-center gap-2">
                <StatusBadge presentation={incidentSeverityPresentation(incident.severity)} />
                <StatusBadge presentation={incidentStatusPresentation(incident.status)} />
                {incident.notification ? (
                  <StatusBadge presentation={notificationPresentation(incident.notification.state)} />
                ) : null}
              </div>
              <p className="text-small font-medium text-ink">{incident.summary}</p>
              <p className="text-small text-ink-2">
                Components: {incident.components.join(", ")}
              </p>
              <p className="text-small text-meta">Updated {incident.updatedAt}</p>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
