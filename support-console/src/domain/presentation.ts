export type StatusTone = "success" | "warn" | "danger" | "neutral";

export type StatusShape = "dot" | "diamond" | "square" | "ring";

export interface StatusPresentation {
  label: string;
  tone: StatusTone;
  shape: StatusShape;
}
