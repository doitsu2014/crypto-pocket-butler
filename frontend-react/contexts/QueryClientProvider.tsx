"use client";

import { QueryClientProvider as TanStackQueryClientProvider } from "@tanstack/react-query";
import { getQueryClient } from "@/lib/query-client";
import { useState } from "react";

/**
 * Query Client Provider wrapper for the application
 * Provides TanStack Query context to all components
 */
export function QueryClientProvider({ children }: { children: React.ReactNode }) {
  // Create query client once per component lifecycle
  const [queryClient] = useState(() => getQueryClient());

  return (
    <TanStackQueryClientProvider client={queryClient}>
      {children}
    </TanStackQueryClientProvider>
  );
}
