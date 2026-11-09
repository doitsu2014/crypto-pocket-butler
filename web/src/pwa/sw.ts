import { shouldBypass } from "./shouldBypass";

/**
 * The root-scoped service worker (NFR9.5, security design).
 *
 * Only same-origin static assets are cached. `/support/*`, every `/v1/*`
 * gateway call and every cross-origin Supabase/WalletConnect request bypass the
 * worker; financial data is never served stale.
 */

interface PrecacheEntry {
  url: string;
  revision?: string | null;
}

interface LifecycleEventLike {
  waitUntil(promise: Promise<unknown>): void;
}

interface FetchEventLike {
  request: Request;
  respondWith(response: Promise<Response>): void;
}

interface WorkerScopeLike {
  readonly location: Location;
  addEventListener(
    type: "install" | "activate",
    listener: (event: LifecycleEventLike) => void,
  ): void;
  addEventListener(type: "fetch", listener: (event: FetchEventLike) => void): void;
  skipWaiting(): Promise<void>;
  readonly clients: { claim(): Promise<void> };
}

declare global {
  interface Window {
    /** Injected by `vite-plugin-pwa` at build time (injectManifest). */
    __WB_MANIFEST: PrecacheEntry[];
  }
}

const worker = self as unknown as WorkerScopeLike;
const CACHE = "cpb-shell-v1";

// The literal `self.__WB_MANIFEST` is the injection point workbox replaces.
const precache: PrecacheEntry[] = self.__WB_MANIFEST ?? [];
const precachedPaths = new Set(
  precache.map((entry) => new URL(entry.url, worker.location.origin).pathname),
);

worker.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(precache.map((entry) => entry.url)))
      .then(() => worker.skipWaiting()),
  );
});

worker.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))),
      )
      .then(() => worker.clients.claim()),
  );
});

worker.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);

  // A non-GET, gateway, support-console or cross-origin request is untouched.
  if (event.request.method !== "GET" || shouldBypass(url, worker.location.origin)) {
    return;
  }

  // A navigation tries the network first and falls back to the cached shell.
  if (event.request.mode === "navigate") {
    event.respondWith(
      fetch(event.request).catch(() =>
        caches.match("/index.html").then((cached) => cached ?? Response.error()),
      ),
    );
    return;
  }

  if (precachedPaths.has(url.pathname)) {
    event.respondWith(caches.match(event.request).then((cached) => cached ?? fetch(event.request)));
  }
});
