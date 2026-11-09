import { useMutation } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { ApiError, declareIncident, NetworkError } from "../api/client";
import type { Incident } from "../api/types";
import {
  hasIncidentErrors,
  incidentStatusPresentation,
  MAX_SUMMARY_LENGTH,
  notificationPresentation,
  parseComponents,
  requiresImmediateNotice,
  validateIncident,
  type IncidentErrors,
} from "../domain/incidents";
import { StatusBadge } from "./StatusBadge";
import {
  alertClass,
  cardClass,
  calloutClass,
  fieldErrorClass,
  inputClass,
  labelClass,
  overlineClass,
  primaryButtonClass,
  sectionTitleClass,
} from "./ui";

export interface IncidentFormProps {
  token: string | null;
  onDeclared: (incident: Incident) => void;
}

export function IncidentForm({ token, onDeclared }: IncidentFormProps) {
  const [severity, setSeverity] = useState("sev2");
  const [summary, setSummary] = useState("");
  const [componentText, setComponentText] = useState("");
  const [errors, setErrors] = useState<IncidentErrors>({});

  const mutation = useMutation<Incident, Error>({
    mutationFn: () =>
      declareIncident(
        {
          severity: severity as Incident["severity"],
          summary: summary.trim(),
          components: parseComponents(componentText),
        },
        token,
      ),
    onSuccess: (incident) => {
      setErrors({});
      onDeclared(incident);
    },
  });

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (mutation.isPending) {
      return;
    }
    const nextErrors = validateIncident({
      severity,
      summary,
      components: parseComponents(componentText),
    });
    setErrors(nextErrors);
    if (hasIncidentErrors(nextErrors)) {
      return;
    }
    mutation.mutate();
  }

  const problem = mutation.error instanceof ApiError ? mutation.error.problem : null;

  return (
    <form data-testid="incident-form" onSubmit={submit} className={cardClass} noValidate>
      <p className={overlineClass}>Admin</p>
      <h2 className={`${sectionTitleClass} mt-1`}>Declare an incident</h2>

      <div className="mt-4 grid gap-4 sm:grid-cols-2">
        <label className={labelClass} htmlFor="incident-form-severity">
          Severity
          <select
            id="incident-form-severity"
            data-testid="incident-form-severity"
            value={severity}
            onChange={(event) => setSeverity(event.target.value)}
            className={inputClass}
          >
            <option value="sev1">SEV-1</option>
            <option value="sev2">SEV-2</option>
            <option value="sev3">SEV-3</option>
          </select>
          {errors.severity ? <span className={fieldErrorClass}>{errors.severity}</span> : null}
        </label>

        <label className={labelClass} htmlFor="incident-form-components">
          Affected components
          <textarea
            id="incident-form-components"
            data-testid="incident-form-components"
            value={componentText}
            onChange={(event) => setComponentText(event.target.value)}
            rows={2}
            aria-invalid={Boolean(errors.components)}
            aria-describedby={errors.components ? "incident-form-components-error" : undefined}
            className={inputClass}
          />
          {errors.components ? (
            <span id="incident-form-components-error" className={fieldErrorClass}>
              {errors.components}
            </span>
          ) : null}
        </label>

        <label className={`${labelClass} sm:col-span-2`} htmlFor="incident-form-summary">
          Summary
          <textarea
            id="incident-form-summary"
            data-testid="incident-form-summary"
            value={summary}
            onChange={(event) => setSummary(event.target.value)}
            rows={3}
            aria-invalid={Boolean(errors.summary)}
            aria-describedby={errors.summary ? "incident-form-summary-error" : undefined}
            className={inputClass}
          />
          {errors.summary ? (
            <span id="incident-form-summary-error" className={fieldErrorClass}>
              {errors.summary}
            </span>
          ) : (
            <span className="text-small text-meta">{MAX_SUMMARY_LENGTH} characters maximum.</span>
          )}
        </label>
      </div>

      {requiresImmediateNotice(severity) ? (
        <p data-testid="incident-form-sev1-notice" className={`${calloutClass} mt-4`}>
          A SEV-1 notice is sent to subscribers within 15 minutes. A failed send is retried and
          surfaced here.
        </p>
      ) : null}

      {mutation.error ? (
        <p role="alert" className={`${alertClass} mt-4`}>
          {mutation.error instanceof NetworkError
            ? "Could not reach the gateway. Check your connection and try again."
            : (problem?.title ?? mutation.error.message)}
          {problem ? (
            <span className="ml-2 font-mono text-meta">Correlation ID {problem.correlationId}</span>
          ) : null}
        </p>
      ) : null}

      {mutation.data ? (
        <div className={`${calloutClass} mt-4`} aria-live="polite">
          <StatusBadge presentation={notificationPresentation(mutation.data.notification?.state ?? "pending")} />
          <StatusBadge presentation={incidentStatusPresentation(mutation.data.status)} />
          {mutation.data.notification?.detail ? (
            <p className="mt-2 text-small text-ink-2">{mutation.data.notification.detail}</p>
          ) : null}
        </div>
      ) : null}

      <button
        type="submit"
        data-testid="incident-form-submit"
        disabled={mutation.isPending}
        className={`${primaryButtonClass} mt-6`}
      >
        {mutation.isPending ? "Declaring…" : "Declare incident"}
      </button>
    </form>
  );
}
