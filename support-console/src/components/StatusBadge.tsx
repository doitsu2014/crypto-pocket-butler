import type { StatusPresentation, StatusTone } from "../domain/presentation";

const toneClass: Record<StatusTone, string> = {
  success: "text-success",
  warn: "text-warn",
  danger: "text-danger",
  neutral: "text-muted",
};

export interface StatusBadgeProps {
  presentation: StatusPresentation;
  testId?: string;
}

/**
 * Status is never colour alone: the visible label carries meaning and the
 * shape gives a second, non-colour cue (WCAG 2.2 AA, design-system rule).
 */
export function StatusBadge({ presentation, testId }: StatusBadgeProps) {
  return (
    <span
      data-testid={testId}
      className={`inline-flex items-center gap-2 rounded-full border border-border-soft bg-surface-warm px-2 py-1 text-overline font-medium uppercase tracking-overline ${toneClass[presentation.tone]}`}
    >
      <span
        aria-hidden="true"
        data-shape={presentation.shape}
        className={`status-shape status-shape-${presentation.shape}`}
      />
      <span>{presentation.label}</span>
    </span>
  );
}
