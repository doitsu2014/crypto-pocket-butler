import { describe, expect, it } from "vitest";
import { ticketStatusPresentation } from "./tickets";

describe("ticketStatusPresentation", () => {
  it("labels the known ticket states with a shape", () => {
    expect(ticketStatusPresentation("open")).toMatchObject({ label: "Open", shape: "diamond" });
    expect(ticketStatusPresentation("pending")).toMatchObject({ label: "Pending", shape: "ring" });
    expect(ticketStatusPresentation("resolved")).toMatchObject({
      label: "Resolved",
      tone: "success",
      shape: "dot",
    });
  });

  it("falls back to the provider's label without inventing a state", () => {
    expect(ticketStatusPresentation("escalated")).toMatchObject({
      label: "escalated",
      tone: "neutral",
      shape: "dot",
    });
  });
});
