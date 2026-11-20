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

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    // Keep the newer snapshot by check time. `strict` is for the one-off
    // invoke result, which must not replace an equally old event (the
    // watchdog re-emits with the same time when the loop stalls).
    const accept = (next: HealthSnapshot, strict: boolean) =>
      setSnapshot((cur) => {
        if (!cur) return next;
        const a = next.checkedAtMs ?? -1;
        const b = cur.checkedAtMs ?? -1;
        return (strict ? a > b : a >= b) ? next : cur;
      });

    listen<HealthSnapshot>("health-changed", (e) => {
      if (cancelled) return;
      setLoadError(null);
      accept(e.payload, false);
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch((e) => {
        if (!cancelled) setLoadError(errorText(e));
      });

    invoke<HealthSnapshot>("health_status")
      .then((s) => {
        if (cancelled) return;
        setLoadError(null);
        accept(s, true);
      })
      .catch((e) => {
        if (!cancelled) setLoadError(errorText(e));
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const checkNow = useCallback(async () => {
    try {
      await invoke("check_health_now");
    } catch (e) {
      setLoadError(errorText(e));
    }
  }, []);

  const source = useCallback(
    (id: string): SourceStatus | undefined =>
      snapshot?.sources.find((s) => s.id === id),
    [snapshot],
  );

  return { snapshot, loadError, checkNow, source };
}
