import { useCallback, useEffect, useRef, useState } from "react";
import { addOwnTask, editOwnTask, listOwnTasks, ownTaskErrorMessage, setOwnTaskDone } from "./api";
import type { OwnTask, OwnTaskInput } from "./types";

export type OwnTasksLoad =
  | { state: "loading" }
  | { state: "error"; message: string }
  | { state: "loaded"; tasks: OwnTask[] };

/** A mutation resolves to an error message to show inline, or `null` on success. */
type MutationResult = Promise<string | null>;

export function useOwnTasks() {
  const [load, setLoad] = useState<OwnTasksLoad>({ state: "loading" });
  const [lastSuccessAt, setLastSuccessAt] = useState<Date | null>(null);

  // Only the latest reload may set the list: an older listing resolving late
  // would bring back stale rows.
  const latestLoad = useRef(0);
  const reload = useCallback(async () => {
    const id = ++latestLoad.current;
    try {
      const tasks = await listOwnTasks();
      if (id === latestLoad.current) {
        setLoad({ state: "loaded", tasks });
        setLastSuccessAt(new Date());
      }
    } catch (e) {
      // A failed reload never replaces an already loaded list.
      if (id === latestLoad.current) {
        setLoad((prev) =>
          prev.state === "loaded" ? prev : { state: "error", message: ownTaskErrorMessage(e) },
        );
      }
    }
  }, []);

  useEffect(() => {
    // reload() sets state only after awaiting the backend, never synchronously.
    void reload();
  }, [reload]);

  const retry = useCallback(async () => {
    setLoad({ state: "loading" });
    await reload();
  }, [reload]);

  const mutate = useCallback(
    async (run: () => Promise<unknown>): MutationResult => {
      try {
        await run();
      } catch (e) {
        return ownTaskErrorMessage(e);
      }
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

  return { load, lastSuccessAt, reload: retry, add, edit, setDone };
}
