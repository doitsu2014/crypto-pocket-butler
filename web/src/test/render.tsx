import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement, ReactNode } from "react";
import { I18nProvider } from "../i18n/react";

export function renderWithProviders(
  ui: ReactElement,
  options: {
    locale?: "en" | "vi";
    queryClient?: QueryClient;
    route?: string;
  } = {},
) {
  const queryClient =
    options.queryClient ??
    new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <I18nProvider locale={options.locale ?? "en"} timezone="UTC">
        <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
      </I18nProvider>
    );
  }

  const tree = options.route ? (
    <MemoryRouter initialEntries={[options.route]}>{ui}</MemoryRouter>
  ) : (
    ui
  );

  return { ...render(tree, { wrapper: Wrapper }), queryClient };
}
