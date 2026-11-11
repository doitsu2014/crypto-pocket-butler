import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createTranslator } from "../../i18n";
import { expectNoAxeViolations } from "../../test/axe";
import { renderWithProviders } from "../../test/render";
import { GateForm } from "./GateForm";

/**
 * The gate form (plan step 6.3/6.4; functional-spec §2.2/§2.3; NFR25).
 * Validation runs on submit only; Continue is never disabled by validation.
 */

const t = createTranslator("en");

function renderForm(props?: {
  serverMissing?: Array<"terms" | "risk" | "age">;
  recorded?: Array<"terms" | "risk" | "age">;
}) {
  const onSubmit = vi.fn();
  const rendered = renderWithProviders(
    <GateForm
      onSubmit={onSubmit}
      isSubmitting={false}
      serverMissing={props?.serverMissing ?? []}
      recorded={props?.recorded ?? []}
    />,
  );
  return { ...rendered, onSubmit };
}

async function completeAll(user: ReturnType<typeof userEvent.setup>) {
  await user.type(screen.getByTestId("gates-country-input"), "viet");
  await user.click(screen.getByTestId("gates-country-option-VN"));
  await user.click(screen.getByTestId("gates-terms-checkbox"));
  await user.click(screen.getByTestId("gates-risk-checkbox"));
  await user.click(screen.getByTestId("gates-age-checkbox"));
}

describe("GateForm", () => {
  it("submits exactly the four values when every field is present", async () => {
    const user = userEvent.setup();
    const { onSubmit } = renderForm();

    await completeAll(user);
    await user.click(screen.getByTestId("gates-continue"));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit).toHaveBeenCalledWith({
      countryCode: "VN",
      terms: true,
      risk: true,
      age: true,
    });
  });

  it("names every missing item in summary order and moves focus, without submitting", async () => {
    const user = userEvent.setup();
    const { onSubmit } = renderForm();

    await user.click(screen.getByTestId("gates-continue"));

    const summary = screen.getByTestId("gates-validation-summary");
    expect(summary).toHaveAttribute("role", "alert");
    expect(summary).toHaveFocus();

    const expected = [
      t("gates.country.required"),
      t("gates.terms.required"),
      t("gates.risk.required"),
      t("gates.age.required"),
    ];
    for (const message of expected) {
      expect(within(summary).getByText(message)).toBeInTheDocument();
    }
    const links = within(summary).getAllByRole("link");
    expect(links.map((link) => link.getAttribute("href"))).toEqual([
      "#gates-country-input",
      "#gates-terms-checkbox",
      "#gates-risk-checkbox",
      "#gates-age-checkbox",
    ]);

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByTestId("gates-continue")).not.toBeDisabled();
  });

  it("renders a per-field inline error beside each missing field", async () => {
    const user = userEvent.setup();
    renderForm();

    await user.click(screen.getByTestId("gates-continue"));

    expect(screen.getByTestId("gates-country-input")).toHaveAttribute("aria-invalid", "true");
    for (const kind of ["terms", "risk", "age"] as const) {
      expect(screen.getByTestId(`gates-${kind}-checkbox`)).toHaveAttribute(
        "aria-describedby",
        `gates-${kind}-error`,
      );
    }
  });

  it("merges server-named missing items into the summary once, server wins", async () => {
    const user = userEvent.setup();
    renderForm({ serverMissing: ["terms"] });

    const summary = screen.getByTestId("gates-validation-summary");
    expect(summary).toHaveTextContent(t("gates.terms.required"));

    await user.click(screen.getByTestId("gates-continue"));

    expect(within(summary).getAllByText(t("gates.terms.required"))).toHaveLength(1);
  });

  it("renders already-recorded acceptances as given and read-only", () => {
    renderForm({ recorded: ["terms"] });

    const terms = screen.getByTestId("gates-terms-checkbox") as HTMLInputElement;
    expect(terms).toBeChecked();
    expect(terms).toBeDisabled();
    expect(screen.getByText(t("gates.accepted"))).toBeInTheDocument();
    expect(screen.getByTestId("gates-risk-checkbox")).not.toBeChecked();
    expect(screen.getByTestId("gates-age-checkbox")).not.toBeChecked();
  });

  it("submits true for an already-recorded acceptance the visitor never ticked", async () => {
    const user = userEvent.setup();
    const { onSubmit } = renderForm({ recorded: ["terms"] });

    await user.type(screen.getByTestId("gates-country-input"), "viet");
    await user.click(screen.getByTestId("gates-country-option-VN"));
    await user.click(screen.getByTestId("gates-risk-checkbox"));
    await user.click(screen.getByTestId("gates-age-checkbox"));
    await user.click(screen.getByTestId("gates-continue"));

    expect(onSubmit).toHaveBeenCalledWith({
      countryCode: "VN",
      terms: true,
      risk: true,
      age: true,
    });
  });

  it("starts with every checkbox unchecked, never pre-checked from local state", () => {
    renderForm();

    for (const kind of ["terms", "risk", "age"] as const) {
      expect(screen.getByTestId(`gates-${kind}-checkbox`)).not.toBeChecked();
    }
  });

  it("keeps DOM order summary → country → terms → risk → age → continue", async () => {
    const user = userEvent.setup();
    renderForm();

    await user.click(screen.getByTestId("gates-continue"));

    const summary = screen.getByTestId("gates-validation-summary");
    const country = screen.getByTestId("gates-country-input");
    const terms = screen.getByTestId("gates-terms-checkbox");
    const risk = screen.getByTestId("gates-risk-checkbox");
    const age = screen.getByTestId("gates-age-checkbox");
    const submit = screen.getByTestId("gates-continue");

    const ordered = [summary, country, terms, risk, age, submit];
    for (let index = 0; index < ordered.length - 1; index += 1) {
      expect(
        ordered[index].compareDocumentPosition(ordered[index + 1]) &
          Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    }
  });

  it("is axe clean empty, with a validation summary, and with server-named items", async () => {
    const user = userEvent.setup();
    const { container } = renderForm({ serverMissing: ["terms"] });

    await expectNoAxeViolations(container);

    await user.click(screen.getByTestId("gates-continue"));
    await expectNoAxeViolations(container);
  });
});
