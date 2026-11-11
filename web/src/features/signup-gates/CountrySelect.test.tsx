import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createTranslator } from "../../i18n";
import { expectNoAxeViolations } from "../../test/axe";
import { renderWithProviders } from "../../test/render";
import { CountrySelect } from "./CountrySelect";

/**
 * The country combobox (plan step 6.1/6.2; performance-design §4/§7;
 * frontend-components §2; WAI-ARIA APG combobox pattern, NFR25).
 */

const t = createTranslator("en");

function renderSelect(options?: { locale?: "en" | "vi"; value?: string }) {
  const onChange = vi.fn();
  const rendered = renderWithProviders(
    <CountrySelect
      value={options?.value ?? ""}
      onChange={onChange}
      labelId="gates-country-label"
    />,
    { locale: options?.locale ?? "en" },
  );
  return { ...rendered, onChange };
}

describe("CountrySelect", () => {
  it("filters by English name and stores the alpha-2 code on selection", async () => {
    const user = userEvent.setup();
    const { onChange } = renderSelect();

    const input = screen.getByTestId("gates-country-input");
    expect(input).toHaveAttribute("role", "combobox");
    expect(input).toHaveAttribute("aria-expanded", "false");

    await user.type(input, "viet");

    const option = screen.getByTestId("gates-country-option-VN");
    expect(option).toHaveTextContent("Vietnam");

    await user.click(option);

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith("VN");
    expect(input).toHaveValue("Vietnam");
    expect(screen.queryByTestId("gates-country-listbox")).not.toBeInTheDocument();
  });

  it("filters diacritic-insensitively and renders Vietnamese names in the VI locale", async () => {
    const user = userEvent.setup();
    renderSelect({ locale: "vi" });

    const input = screen.getByTestId("gates-country-input");
    await user.type(input, "viet");

    const option = screen.getByTestId("gates-country-option-VN");
    expect(option).toHaveTextContent("Việt Nam");
  });

  it("is fully operable from the keyboard (arrows, End, Enter)", async () => {
    const user = userEvent.setup();
    const { onChange } = renderSelect();

    const input = screen.getByTestId("gates-country-input");
    await user.type(input, "viet");
    await user.keyboard("{ArrowDown}");

    expect(screen.getByTestId("gates-country-listbox")).toBeInTheDocument();
    expect(input).toHaveAttribute("aria-expanded", "true");
    expect(input).toHaveAttribute("aria-activedescendant", "gates-country-option-VN");

    await user.keyboard("{End}");
    await user.keyboard("{Enter}");

    expect(onChange).toHaveBeenCalledWith("VN");
    expect(input).toHaveAttribute("aria-expanded", "false");
  });

  it("closes the listbox with Escape without changing the value", async () => {
    const user = userEvent.setup();
    const { onChange } = renderSelect();

    const input = screen.getByTestId("gates-country-input");
    await user.type(input, "viet");
    await user.keyboard("{Escape}");

    expect(screen.queryByTestId("gates-country-listbox")).not.toBeInTheDocument();
    expect(input).toHaveAttribute("aria-expanded", "false");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("anchors its inline error to the input for assistive technology", () => {
    renderWithProviders(
      <CountrySelect
        value=""
        onChange={vi.fn()}
        labelId="gates-country-label"
        error={t("gates.country.required")}
      />,
    );

    const input = screen.getByTestId("gates-country-input");
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAttribute("aria-describedby", "gates-country-error");
    expect(screen.getByText(t("gates.country.required"))).toBeInTheDocument();
  });

  it("is axe clean open and closed", async () => {
    const user = userEvent.setup();
    const { container } = renderSelect();

    await expectNoAxeViolations(container);

    await user.type(screen.getByTestId("gates-country-input"), "viet");
    await expectNoAxeViolations(container);
  });

  it("keeps the combobox label associated with the input", () => {
    renderSelect();

    expect(screen.getByLabelText(t("gates.country.label"))).toBe(
      screen.getByTestId("gates-country-input"),
    );
  });
});
