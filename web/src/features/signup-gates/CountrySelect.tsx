/**
 * Country combobox (functional-spec §2.2; performance-design §4/§7;
 * frontend-components §2; WAI-ARIA APG combobox, NFR25).
 *
 * `React.memo`-wrapped and owning its own query string, so a keystroke
 * re-renders only this component's listbox — never the three checkboxes or the
 * submit control. Filtering is a substring match over the precomputed `search`
 * key from the generated module: no per-keystroke normalisation, no network.
 *
 * Requirements: FR19.3, NFR2.4, NFR25.1, NFR25.2, NFR26.1.
 */

import { memo, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useI18n } from "../../i18n/react";
import { COUNTRIES, type Country } from "./countries";

export interface CountrySelectProps {
  value: string;
  onChange: (code: string) => void;
  error?: string;
  labelId: string;
}

const MAX_VISIBLE = 50;

function fold(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/đ/g, "d")
    .replace(/Đ/g, "d")
    .toLowerCase();
}

function displayName(country: Country | undefined, locale: "en" | "vi"): string {
  if (!country) {
    return "";
  }
  return locale === "vi" ? country.vi : country.en;
}

function CountrySelectImpl({ value, onChange, error, labelId }: CountrySelectProps) {
  const { t, locale } = useI18n();
  const selected = COUNTRIES.find((country) => country.code === value);
  const [query, setQuery] = useState(() => displayName(selected, locale));
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const blurTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const matches = useMemo(() => {
    const needle = fold(query);
    const pool = needle
      ? COUNTRIES.filter((country) => country.search.includes(needle))
      : COUNTRIES;
    return pool.slice(0, MAX_VISIBLE);
  }, [query]);

  function close(): void {
    setOpen(false);
    setActive(-1);
  }

  function select(country: Country): void {
    setQuery(displayName(country, locale));
    close();
    onChange(country.code);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>): void {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setOpen(true);
      setActive((current) => {
        const next = current < 0 ? 0 : Math.min(current + 1, matches.length - 1);
        return next;
      });
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setOpen(true);
      setActive((current) => Math.max(current - 1, 0));
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      setOpen(true);
      setActive(0);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      setOpen(true);
      setActive(matches.length - 1);
      return;
    }
    if (event.key === "Enter") {
      if (open && active >= 0 && active < matches.length) {
        event.preventDefault();
        select(matches[active]);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key === "Tab") {
      close();
    }
  }

  const activeOption = open && active >= 0 ? matches[active] : undefined;
  const listboxId = "gates-country-listbox";
  const errorId = "gates-country-error";

  return (
    <div className="relative flex flex-col gap-1">
      <label
        id={labelId}
        htmlFor="gates-country-input"
        className="text-small font-medium text-ink-2"
      >
        {t("gates.country.label")}
      </label>
      <input
        id="gates-country-input"
        data-testid="gates-country-input"
        type="text"
        role="combobox"
        autoComplete="off"
        aria-autocomplete="list"
        aria-expanded={open}
        aria-controls={listboxId}
        aria-activedescendant={
          activeOption ? `gates-country-option-${activeOption.code}` : undefined
        }
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        placeholder={t("gates.country.placeholder")}
        className="rounded-xl border border-border bg-surface px-3 py-2 text-small text-ink"
        value={query}
        onChange={(event) => {
          setQuery(event.target.value);
          setOpen(true);
          setActive(0);
        }}
        onFocus={() => {
          if (blurTimer.current) {
            clearTimeout(blurTimer.current);
            blurTimer.current = null;
          }
          setOpen(true);
        }}
        onBlur={() => {
          blurTimer.current = setTimeout(() => setOpen(false), 0);
        }}
        onKeyDown={handleKeyDown}
      />
      {error ? (
        <p id={errorId} className="text-small text-danger">
          {error}
        </p>
      ) : null}
      {open ? (
        <ul
          id={listboxId}
          data-testid={listboxId}
          role="listbox"
          aria-labelledby={labelId}
          className="absolute top-full z-10 mt-1 max-h-64 w-full overflow-auto rounded-xl border border-border bg-surface py-1 shadow-[rgba(0,0,0,0.05)_0_4px_24px]"
        >
          {matches.map((country) => (
            <li
              key={country.code}
              id={`gates-country-option-${country.code}`}
              data-testid={`gates-country-option-${country.code}`}
              role="option"
              aria-selected={country.code === value}
              className="cursor-pointer px-3 py-2 text-small text-ink hover:bg-parchment"
              onMouseDown={(event) => {
                event.preventDefault();
                select(country);
              }}
            >
              {displayName(country, locale)}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

export const CountrySelect = memo(CountrySelectImpl);
