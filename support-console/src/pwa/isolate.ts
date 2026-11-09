/**
 * A root-scoped customer service worker controls every same-origin path,
 * including `/support` and the same-origin `/v1` C2 calls. Registering no
 * worker for the console does not stop the customer's worker, so the console
 * unregisters any registration whose scope covers `/support` at startup.
 *
 * U2 owns the customer worker and must also bypass `/support` and `/v1`; this
 * module is the console-side defence in depth.
 */

export interface ServiceWorkerRegistrationLike {
  scope: string;
  unregister(): Promise<boolean>;
}

export interface ServiceWorkerContainerLike {
  getRegistrations(): Promise<ReadonlyArray<ServiceWorkerRegistrationLike>>;
}

function normalizePath(path: string): string {
  const withLeading = path.startsWith("/") ? path : `/${path}`;
  return withLeading.endsWith("/") ? withLeading : `${withLeading}/`;
}

function normalizeScope(scope: string): string {
  try {
    return normalizePath(new URL(scope, "https://support.invalid").pathname);
  } catch {
    return normalizePath(scope);
  }
}

/** True when a registration's scope controls the support console's path. */
export function scopeCoversSupport(scope: string, supportPath = "/support/"): boolean {
  return normalizePath(supportPath).startsWith(normalizeScope(scope));
}

/**
 * Unregisters every registration whose scope covers `/support` and returns the
 * count. A missing Service Worker API is not an error.
 */
export async function isolateSupportFromServiceWorkers(
  container: ServiceWorkerContainerLike | null | undefined =
    typeof navigator !== "undefined" ? (navigator.serviceWorker ?? null) : null,
): Promise<number> {
  if (!container) {
    return 0;
  }

  const registrations = await container.getRegistrations();
  let unregistered = 0;
  for (const registration of registrations) {
    if (scopeCoversSupport(registration.scope)) {
      await registration.unregister();
      unregistered += 1;
    }
  }
  return unregistered;
}
