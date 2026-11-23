import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  WORK_TIME_CHANGED,
  addWorkEntry,
  asWorkTimeError,
  listWorkEntries,
  removeWorkEntry,
  updateWorkEntry,
  workTimeErrorMessage,
} from "./api";
import type { DayEntries, WorkTimeError } from "./types";

type Result =
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; data: DayEntries };

/** The state the view sees; a result for another day counts as loading. */
export type EntriesState =
  | { state: "loading" }
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; data: DayEntries };

function toError(e: unknown): WorkTimeError {
  return asWorkTimeError(e) ?? { kind: "Background" };
}

export function useWorkEntries(day: string) {
  const [result, setResult] = useState<{ day: string; value: Result } | null>(null);
  const [busyIds, setBusyIds] = useState<Set<number>>(new Set());

  // Latest request wins: a slower, older response must not replace a newer one
  // (or the list of a day the user has already left).
  const issued = useRef(0);
  const applied = useRef(0);
  const lastOkAt = useRef<number | null>(null);

  const refresh = useCallback(async () => {
    const id = ++issued.current;
    let value: Result;
    try {
      const data = await listWorkEntries(day);
      value = { state: "loaded", data };
    } catch (e) {
      value = { state: "error", message: workTimeErrorMessage(e), lastOkAt: lastOkAt.current };
    }
    if (id < applied.current) return;
    applied.current = id;
    if (value.state === "loaded") lastOkAt.current = Date.now();
    setResult({ day, value });
  }, [day]);

  // Subscribe first and load only once the listener is registered, so no event
  // between the two is lost.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    listen(WORK_TIME_CHANGED, () => {
      if (!cancelled) void refresh();
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
        void refresh();
      })
      .catch((e) => {
        if (cancelled) return;
        const id = ++issued.current;
        applied.current = id;
        setResult({
          day,
          value: { state: "error", message: workTimeErrorMessage(e), lastOkAt: lastOkAt.current },
        });
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [day, refresh]);

  const withBusy = useCallback(
    async (id: number | null, call: () => Promise<void>): Promise<WorkTimeError | null> => {
      if (id !== null) setBusyIds((b) => new Set(b).add(id));
      try {
        await call();
        void refresh();
        return null;
      } catch (e) {
        return toError(e);
      } finally {
        if (id !== null) {
          setBusyIds((b) => {
            const next = new Set(b);
            next.delete(id);
            return next;
          });
        }
      }
    },
    [refresh],
  );

  const add = useCallback(
    (start: string, end: string) => withBusy(null, () => addWorkEntry(start, end)),
    [withBusy],
  );
  const update = useCallback(
    (id: number, start: string, end: string | null) =>
      withBusy(id, () => updateWorkEntry(id, start, end)),
    [withBusy],
  );
  const remove = useCallback((id: number) => withBusy(id, () => removeWorkEntry(id)), [withBusy]);
  const retry = useCallback(() => void refresh(), [refresh]);

  const entries: EntriesState =
    result !== null && result.day === day ? result.value : { state: "loading" };

  return { entries, add, update, remove, busyIds, retry };
}
