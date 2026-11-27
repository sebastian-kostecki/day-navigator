import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  WORK_TIME_CHANGED,
  addWorkEntry,
  listWorkEntries,
  removeWorkEntry,
  setDayOff as setDayOffCall,
  updateWorkEntry,
  workTimeErrorMessage,
} from "./api";
import type { DayEntries, DayMark } from "./types";

type Result =
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; data: DayEntries };

/** The state the view sees; a result for another day counts as loading. */
export type EntriesState =
  | { state: "loading" }
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "loaded"; data: DayEntries };

export function useWorkEntries(day: string) {
  const [result, setResult] = useState<{ day: string; value: Result } | null>(null);
  const [busyIds, setBusyIds] = useState<Set<number>>(new Set());
  // A day-off request stays pending until the reloaded day agrees with it, so
  // the control neither snaps back nor re-enables before the list catches up.
  const [dayOffRequest, setDayOffRequest] = useState<{ day: string; mark: DayMark } | null>(null);
  // Bumped by retry to redo the subscribe and the load together.
  const [attempt, setAttempt] = useState(0);

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
  // between the two is lost. A failed subscribe never loads, and retry redoes
  // both, so a list that can no longer update is never shown.
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
  }, [day, refresh, attempt]);

  /**
   * Resolves to `null` on success, or to the raw failure for
   * `workTimeErrorMessage`. The list itself refreshes via `work-time-changed`.
   */
  const withBusy = useCallback(
    async (id: number | null, call: () => Promise<void>): Promise<unknown> => {
      if (id !== null) setBusyIds((b) => new Set(b).add(id));
      try {
        await call();
        return null;
      } catch (e) {
        return e || new Error("brak szczegółów");
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
    [],
  );

  const add = useCallback(
    (start: string, end: string, note: string) =>
      withBusy(null, () => addWorkEntry(start, end, note)),
    [withBusy],
  );
  const update = useCallback(
    (id: number, start: string, end: string | null, note: string) =>
      withBusy(id, () => updateWorkEntry(id, start, end, note)),
    [withBusy],
  );
  const remove = useCallback((id: number) => withBusy(id, () => removeWorkEntry(id)), [withBusy]);
  const setDayMark = useCallback(
    async (mark: DayMark): Promise<unknown> => {
      setDayOffRequest({ day, mark });
      const err = await withBusy(null, () => setDayOffCall(day, mark));
      if (err) setDayOffRequest(null);
      return err;
    },
    [day, withBusy],
  );
  const retry = useCallback(() => setAttempt((n) => n + 1), []);

  const entries: EntriesState =
    result !== null && result.day === day ? result.value : { state: "loading" };

  const storedMark: DayMark =
    entries.state === "loaded" &&
    (entries.data.dayKind === "dayOff" || entries.data.dayKind === "halfDayOff")
      ? entries.data.dayKind
      : "workday";
  // A request is done once the reloaded day agrees, or once another day is
  // shown: a later change from elsewhere (the days table) must not leave it
  // pending against the new state.
  if (
    dayOffRequest !== null &&
    (dayOffRequest.day !== day || (entries.state === "loaded" && dayOffRequest.mark === storedMark))
  ) {
    setDayOffRequest(null);
  }
  const pending =
    dayOffRequest !== null &&
    dayOffRequest.day === day &&
    entries.state === "loaded" &&
    dayOffRequest.mark !== storedMark;
  const dayMark = pending ? dayOffRequest.mark : storedMark;

  return {
    entries,
    add,
    update,
    remove,
    setDayMark,
    dayMark,
    dayMarkBusy: pending,
    busyIds,
    retry,
  };
}
