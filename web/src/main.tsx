import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router-dom";
import { createQueryClient } from "./api/queryClient";
import { ErrorBoundary } from "./app/ErrorBoundary";
import { router } from "./app/router";
import { signOut } from "./auth/session";
import { LocaleProvider } from "./i18n/LocaleProvider";
import "./styles.css";

const queryClient = createQueryClient({
  onUnauthorized: () => {
    const returnTo = `${window.location.pathname}${window.location.search}`;
    queryClient.clear();
    void signOut().finally(() => {
      window.location.assign(`/?returnTo=${encodeURIComponent(returnTo)}`);
    });
  },
});

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <ErrorBoundary>
      <LocaleProvider>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </LocaleProvider>
    </ErrorBoundary>
  </StrictMode>,
);
