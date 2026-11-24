import { useEffect, useState } from "react";

/** How often relative labels ("2 min temu", "3 dni") are recomputed. */
export const RELATIVE_TICK_MS = 30_000;

/**
 * The current time in ms, refreshed every `intervalMs`. `null` stops the tick and
 * keeps the last value.
 */
export function useNow(intervalMs: number | null): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (intervalMs === null) return;
    const handle = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(handle);
  }, [intervalMs]);
  return now;
}
