import { defineConfig } from "vitest/config";

// Vitest bundles its own Vite; the React plugin is only needed for Fast Refresh
// in `vite dev`, so tests use esbuild's automatic JSX transform directly.
export default defineConfig({
  esbuild: { jsx: "automatic" },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
