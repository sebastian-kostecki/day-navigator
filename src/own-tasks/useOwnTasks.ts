import { useCallback, useEffect, useRef, useState } from "react";
import {
  addOwnTask,
  editOwnTask,
  isOwnTaskNotFound,
  listOwnTasks,
  ownTaskErrorMessage,
  setOwnTaskDone,
} from "./api";
import type { OwnTask, OwnTaskInput } from "./types";

export type OwnTasksLoad =
  | { state: "loading" }
  | { state: "error"; message: string }
  | { state: "loaded"; tasks: OwnTask[] };

/** A mutation resolves to an error message to show inline, or `null` on success. */
type MutationResult = Promise<string | null>;

type ListResult = { ok: true; tasks: OwnTask[] } | { ok: false; message: string };

/** Fetches the list and never rejects, so callers set state only after it settles. */
function settleList(): Promise<ListResult> {
  return listOwnTasks().then(
    (tasks) => ({ ok: true, tasks }),
    (e: unknown) => ({ ok: false, message: ownTaskErrorMessage(e) }),
  );
}

export function useOwnTasks() {
  const [load, setLoad] = useState<OwnTasksLoad>({ state: "loading" });
  const [lastSuccessAt, setLastSuccessAt] = useState<Date | null>(null);
  // A failed reload over an already loaded list: the list stays readable but
  // is stale, and the view must say so.
  const [reloadError, setReloadError] = useState<string | null>(null);

  // Only the latest reload may set the list: an older listing resolving late
  // would bring back stale rows.
  const latestLoad = useRef(0);
  const applyList = useCallback((id: number, result: ListResult) => {
    if (id !== latestLoad.current) return;
    if (result.ok) {
      setLoad({ state: "loaded", tasks: result.tasks });
      setLastSuccessAt(new Date());
      setReloadError(null);
    } else {
      const { message } = result;
      // A failed reload never replaces an already loaded list; it is
      // surfaced next to it instead (`reloadError`).
      setLoad((prev) => (prev.state === "loaded" ? prev : { state: "error", message }));
      setReloadError(message);
    }
  }, []);

  const reload = useCallback(async () => {
    const id = ++latestLoad.current;
    applyList(id, await settleList());
  }, [applyList]);

  useEffect(() => {
    const id = ++latestLoad.current;
    // State is set only in the callback, after the backend answers.
    void settleList().then((result) => applyList(id, result));
  }, [applyList]);

  const retry = useCallback(async () => {
    setLoad({ state: "loading" });
    await reload();
  }, [reload]);

  const mutate = useCallback(
    async (run: () => Promise<unknown>): MutationResult => {
      try {
        await run();
      } catch (e) {
        // The task is gone, so the shown list is stale: refresh it.
        if (isOwnTaskNotFound(e)) void reload();
        return ownTaskErrorMessage(e);
      }
      // The change is saved even if this reload fails: report success so the
      // form never invites a duplicate; a failed reload shows `reloadError`.
      await reload();
      return null;
    },
    [reload],
  );

  const add = useCallback(
    (input: OwnTaskInput): MutationResult => mutate(() => addOwnTask(input)),
    [mutate],
  );
  const edit = useCallback(
    (id: number, input: OwnTaskInput): MutationResult => mutate(() => editOwnTask(id, input)),
    [mutate],
  );
  const setDone = useCallback(
    (id: number, done: boolean): MutationResult => mutate(() => setOwnTaskDone(id, done)),
    [mutate],
  );

  return { load, lastSuccessAt, reloadError, retry, refresh: reload, add, edit, setDone };
}
