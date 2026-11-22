import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { HealthSnapshot, SourceStatus } from "./types";

function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}

export function useHealth() {
  const [snapshot, setSnapshot] = useState<HealthSnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [checkError, setCheckError] = useState<string | null>(null);
  // Bumped by `retry` to re-run the subscribe-and-load effect.
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    // Keep the newer snapshot by `seq`, never by the wall clock (a slow
    // invoke must not overwrite a newer event, and a clock moved back must
    // not freeze the panel).
    const accept = (next: HealthSnapshot) =>
      setSnapshot((cur) => (!cur || next.seq > cur.seq ? next : cur));

    // Subscribe first and load only once the listener is registered, so no
    // event between the two is lost. A failed subscribe never loads, so a
    // snapshot that can no longer update is never shown without the error.
    listen<HealthSnapshot>("health-changed", (e) => {
      if (cancelled) return;
      setLoadError(null);
      accept(e.payload);
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
        return invoke<HealthSnapshot>("health_status").then((s) => {
          if (cancelled) return;
          setLoadError(null);
          accept(s);
        });
      })
      .catch((e) => {
        if (!cancelled) setLoadError(errorText(e));
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [attempt]);

  /** Re-runs the subscribe and the load that failed. */
  const retry = useCallback(() => setAttempt((n) => n + 1), []);

  // A failed "check now" is not a failed load: the stored snapshot is still
  // readable, so it gets its own error next to the button.
  const checkNow = useCallback(async () => {
    try {
      await invoke("check_health_now");
      setCheckError(null);
    } catch (e) {
      setCheckError(errorText(e));
    }
  }, []);

  const source = useCallback(
    (id: string): SourceStatus | undefined => snapshot?.sources.find((s) => s.id === id),
    [snapshot],
  );

  return { snapshot, loadError, checkError, checkNow, retry, source };
}
