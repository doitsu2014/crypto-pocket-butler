// core/data/native-phases.mjs — my-aidlc's projection metadata for the five
// native AI-DLC phases.
//
// The pinned AI-DLC release ships stages, agents, scopes, and protocols, but no
// per-phase `phase.md`: phase membership comes from each stage's `phase:` field
// and ordering comes from the compiled prefix (initialization 0, ideation 1,
// inception 2, construction 3, operation 4). The engine still needs a display
// name, focus, and review flag per phase, so this table authors exactly that
// projection layer — engine metadata only, never methodology.
//
// `order` follows the native phase prefix. `review` is the my-aidlc gate policy
// for leaving the phase and defaults to `auto`.

export const NATIVE_PHASES = [
  {
    slug: "initialization",
    name: "Initialization",
    order: 1,
    focus: "Setup",
    ai_role: "Workspace orchestrator",
    output: "Intent and record dir",
    description:
      "Create the intent and its record dir, classify the workspace, and route the plan the scope selects.",
    key_activities: [
      "Scaffold the workspace shell",
      "Classify the project and its build system",
      "Create the intent record and initialise state",
    ],
    example_prompts: ["Set up this workspace for AI-DLC"],
    review: "auto",
  },
  {
    slug: "ideation",
    name: "Ideation",
    order: 2,
    focus: "Intent",
    ai_role: "Product partner",
    output: "Brief, scope, approval",
    description:
      "Capture intent, research the market and prior art, test feasibility, define scope, form the team, sketch the experience, and secure approval before technical work begins.",
    key_activities: [
      "Intent capture and framing",
      "Market and build-versus-buy research",
      "Feasibility and constraint analysis",
      "Scope definition and prioritised backlog",
      "Rough mockups and team formation",
      "Approval handoff into Inception",
    ],
    example_prompts: [
      "What problem are we solving and for whom?",
      "What should this first version leave out?",
      "Is this technically feasible with what we have?",
    ],
    review: "auto",
  },
  {
    slug: "inception",
    name: "Inception",
    order: 3,
    focus: "Design",
    ai_role: "Architect partner",
    output: "Requirements, stories, architecture, units",
    description:
      "Turn approved intent into reverse-engineered understanding, formal requirements, user stories, refined mockups, application and domain design, units of work, and a delivery plan.",
    key_activities: [
      "Reverse engineering for brownfield systems",
      "Requirements analysis",
      "User stories and personas",
      "Refined mockups and interaction spec",
      "Domain, contract, and application design",
      "Units generation and delivery planning",
      "Practices discovery",
    ],
    example_prompts: [
      "Write testable requirements from this intent",
      "Break this into independently verifiable units",
      "How should these components fit together?",
    ],
    review: "auto",
  },
  {
    slug: "construction",
    name: "Construction",
    order: 4,
    focus: "Code",
    ai_role: "Coding agent",
    output: "Working code",
    description:
      "Design the functional and non-functional detail, specify infrastructure, generate code per unit, test the build, and wire the CI pipeline.",
    key_activities: [
      "Functional design",
      "Non-functional requirements and design",
      "Infrastructure design",
      "Code generation per unit",
      "Build and test",
      "CI pipeline configuration",
    ],
    example_prompts: [
      "Implement this unit against its acceptance criteria",
      "What should we test before this ships?",
      "Write the CI configuration for this project",
    ],
    review: "auto",
  },
  {
    slug: "operation",
    name: "Operation",
    order: 5,
    focus: "Run",
    ai_role: "Operations assistant",
    output: "Deployments, observability",
    description:
      "Deploy, provision environments, observe the running system, prepare incident response, validate performance against the non-functional targets, and feed improvements back.",
    key_activities: [
      "Deployment pipeline and strategy",
      "Environment provisioning",
      "Deployment execution",
      "Observability setup",
      "Incident response readiness",
      "Performance validation",
      "Feedback and optimisation",
    ],
    example_prompts: [
      "Write a rollback runbook for this deployment",
      "Which signals should alert us?",
      "Did we meet the performance targets?",
    ],
    review: "auto",
  },
];

/** Render one native phase as a `phase.md` with the engine's frontmatter. */
export function renderPhaseFile(phase) {
  const lines = ["---"];
  lines.push(`slug: ${phase.slug}`);
  lines.push(`name: ${phase.name}`);
  lines.push(`order: ${phase.order}`);
  lines.push(`focus: ${phase.focus}`);
  lines.push(`ai_role: ${phase.ai_role}`);
  lines.push(`output: ${phase.output}`);
  lines.push(`review: ${phase.review || "auto"}`);
  lines.push(`description: ${JSON.stringify(phase.description)}`);
  lines.push("key_activities:");
  for (const activity of phase.key_activities) lines.push(`  - ${JSON.stringify(activity)}`);
  lines.push("example_prompts:");
  for (const prompt of phase.example_prompts) lines.push(`  - ${JSON.stringify(prompt)}`);
  lines.push("---");
  lines.push("");
  lines.push(`# ${phase.name}`);
  lines.push("");
  lines.push(phase.description);
  lines.push("");
  lines.push(
    "> Native phase metadata. Membership and order come from the pinned AI-DLC release; " +
      "this file only supplies my-aidlc's display and gate fields. Do not edit by hand — " +
      "regenerate with `my-aidlc update`.",
  );
  lines.push("");
  return `${lines.join("\n")}`;
}
