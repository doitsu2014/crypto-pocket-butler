import type { ConsoleRole } from "../api/types";
import type { StatusPresentation } from "../domain/presentation";
import { StatusBadge } from "./StatusBadge";

const rolePresentation: Record<ConsoleRole, StatusPresentation> = {
  owner: { label: "Owner", tone: "neutral", shape: "ring" },
  admin: { label: "Admin", tone: "success", shape: "dot" },
  support: { label: "Support", tone: "warn", shape: "diamond" },
};

export interface RoleBadgeProps {
  role: ConsoleRole;
}

/**
 * A role is never colour alone: the word is visible and the shape is a second
 * cue (design-system rule, `frontend-components.md` §5).
 */
export function RoleBadge({ role }: RoleBadgeProps) {
  const presentation = rolePresentation[role];
  return (
    <span
      data-testid="role-badge"
      aria-label={`${presentation.label} role`}
      className="inline-flex"
    >
      <StatusBadge presentation={presentation} />
    </span>
  );
}
