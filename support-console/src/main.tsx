import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createSupportQueryClient } from "./api/queryClient";
import {
  configFromEnv,
  createSupabaseAuthClient,
  createSupportSupabaseClient,
} from "./auth/supabase";
import { isolateSupportFromServiceWorkers } from "./pwa/isolate";
import SupportApp from "./screens/SupportApp";
import "./styles.css";

// A root-scoped customer service worker must not control /support or the
// same-origin /v1 calls. U2 owns the worker's bypass; this is defence in depth.
void isolateSupportFromServiceWorkers().catch((error) => {
  console.error("Could not isolate the support console from the customer service worker.", error);
});

const supabaseConfig = configFromEnv();
const authClient = supabaseConfig
  ? createSupabaseAuthClient(createSupportSupabaseClient(supabaseConfig))
  : null;
const queryClient = createSupportQueryClient();

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <SupportApp authClient={authClient} />
    </QueryClientProvider>
  </StrictMode>,
);
