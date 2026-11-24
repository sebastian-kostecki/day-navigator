import { useCallback, useEffect, useRef, useState } from "react";
import { reminderSettings, reminderSettingsErrorMessage } from "./api";
import type { ReminderThresholds } from "./types";

export type ReminderSettingsState =
  | { state: "loading" }
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; thresholds: ReminderThresholds };

/**
 * The reminder thresholds, read when the view mounts, when the window is focused
 * or becomes visible (so an edited settings.json shows up) and on retry.
 */
export function useReminderSettings() {
  const [state, setState] = useState<ReminderSettingsState>({ state: "loading" });

  // Latest request wins, as in useMonthReport.
  const issued = useRef(0);
  const applied = useRef(0);
  const lastOk = useRef<number | null>(null);

  const refresh = useCallback(async () => {
    const id = ++issued.current;
    let value: ReminderSettingsState;
    try {
      value = { state: "loaded", thresholds: await reminderSettings() };
    } catch (e) {
      value = {
        state: "error",
        message: reminderSettingsErrorMessage(e),
        lastOkAt: lastOk.current,
      };
    }
    if (id < applied.current) return;
    applied.current = id;
    if (value.state === "loaded") lastOk.current = Date.now();
    setState(value);
  }, []);

  useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === "visible") void refresh();
    };
    const onFocus = () => void refresh();
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("focus", onFocus);
    void refresh();
    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onFocus);
    };
  }, [refresh]);

  const retry = useCallback(() => void refresh(), [refresh]);

  return { state, retry };
}
