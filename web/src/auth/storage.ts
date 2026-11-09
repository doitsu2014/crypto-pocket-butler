/**
 * The Supabase storage adapter (NFR9.1).
 *
 * Every auth key lives in memory except the PKCE `${storageKey}-code-verifier`,
 * which alone is written to sessionStorage so the full-page callback can
 * complete. Access and refresh tokens therefore never touch browser storage.
 */

export interface SessionStorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export interface MemoryStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export function createMemoryStorage(
  storageKey: string,
  sessionStorageImpl: SessionStorageLike,
): MemoryStorage {
  const memory = new Map<string, string>();
  const verifierKey = `${storageKey}-code-verifier`;

  return {
    getItem(key) {
      return key === verifierKey
        ? sessionStorageImpl.getItem(key)
        : (memory.get(key) ?? null);
    },
    setItem(key, value) {
      if (key === verifierKey) {
        sessionStorageImpl.setItem(key, value);
      } else {
        memory.set(key, value);
      }
    },
    removeItem(key) {
      if (key === verifierKey) {
        sessionStorageImpl.removeItem(key);
      } else {
        memory.delete(key);
      }
    },
  };
}
