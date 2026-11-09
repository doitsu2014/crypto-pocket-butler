import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { VitePWA } from "vite-plugin-pwa";

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    VitePWA({
      strategies: "injectManifest",
      // The worker source stays at src/pwa/sw.ts; the plugin nests its output
      // when `filename` contains a directory, so scope the source directory
      // instead and let the worker land at the site root.
      srcDir: "src/pwa",
      filename: "sw.ts",
      registerType: "autoUpdate",
      injectRegister: "auto",
      manifest: {
        name: "Crypto Pocket Butler",
        short_name: "Pocket Butler",
        description:
          "Portfolio, drift, and order intent — one considered view of your crypto holdings.",
        theme_color: "#c96442",
        background_color: "#faf9f5",
        display: "standalone",
        start_url: "/dashboard",
        scope: "/",
        icons: [
          {
            src: "/logo-mark.svg",
            sizes: "any",
            type: "image/svg+xml",
            purpose: "any",
          },
        ],
      },
      injectManifest: {
        globPatterns: ["**/*.{js,css,html,svg,woff2}"],
      },
    }),
  ],
  build: {
    rollupOptions: {
      output: {
        // Lazy journey pages are named route-* so the declared size budget
        // (`dist/assets/route-*.js`) measures them; the per-chunk limit is
        // enforced file-by-file by scripts/check-size.mjs.
        chunkFileNames: (chunk) =>
          chunk.name.endsWith("Page") ? "assets/route-[name]-[hash].js" : "assets/[name]-[hash].js",
      },
    },
  },
  server: {
    port: 5173,
  },
});
