import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

/*
 * The support console is a separate SPA served under `/support` by the
 * existing web image. Building writes into `web/dist/support`, which the
 * web image copies verbatim, so the two bundles ship as one immutable
 * static artefact.
 */
export default defineConfig({
  base: "/support/",
  plugins: [react(), tailwindcss()],
  build: {
    outDir: "../web/dist/support",
    emptyOutDir: true,
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ["react", "react-dom", "@tanstack/react-query"],
          supabase: ["@supabase/supabase-js"],
        },
      },
    },
  },
  server: {
    port: 5174,
  },
});
