import { useEffect, useState } from "react";
import { GRANT_CLOCK_INTERVAL_MS } from "../domain/grants";

/**
 * Tracks "now" while a grant is live. The banner refreshes at expiresAt and
 * polls every 30 s as a fallback; reads attempted in the up-to-30-second
 * window after expiry are refused by the server, which is accepted.
 */
export function useGrantClock(
  expiresAt: string | null | undefined,
  intervalMs: number = GRANT_CLOCK_INTERVAL_MS,
): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!expiresAt) {
      return;
    }

    const tick = () => setNow(Date.now());
    const interval = setInterval(tick, intervalMs);
    const expiry = Date.parse(expiresAt);
    let timeout: ReturnType<typeof setTimeout> | undefined;
    if (Number.isFinite(expiry)) {
      timeout = setTimeout(tick, Math.max(0, expiry - Date.now()));
    }

    return () => {
      clearInterval(interval);
      if (timeout !== undefined) {
        clearTimeout(timeout);
      }
    };
  }, [expiresAt, intervalMs]);

  return now;
}
