import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ItemKind } from "../gitlab/types";
import {
  addTodoGitlabItem,
  addTodoOwnTask,
  listTodo,
  removeTodo,
  setTodoTicked,
  todoErrorMessage,
} from "./api";
import type { TodoList } from "./types";

const TODO_CHANGED = "todo-changed";

export type TodoLoad =
  { state: "loading" } | { state: "error"; message: string } | { state: "loaded"; list: TodoList };

/** A mutation resolves to an error message to show inline, or `null` on success. */
type MutationResult = Promise<string | null>;

type ListResult = { ok: true; list: TodoList } | { ok: false; message: string };

/** Awaits a list-returning call and never rejects. */
function settle(call: () => Promise<TodoList>): Promise<ListResult> {
  return call().then(
    (list) => ({ ok: true, list }),
    (e: unknown) => ({ ok: false, message: todoErrorMessage(e) }),
  );
}

/** Milliseconds until the next local midnight, plus a second of slack. */
function msToNextMidnight(): number {
  const now = new Date();
  const next = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1, 0, 0, 1);
  return Math.max(1000, next.getTime() - now.getTime());
}

export function useTodo() {
  const [load, setLoad] = useState<TodoLoad>({ state: "loading" });
  const [lastSuccessAt, setLastSuccessAt] = useState<Date | null>(null);
  // A failed reload over an already loaded list: the list stays readable but
  // is stale, and the view must say so.
  const [reloadError, setReloadError] = useState<string | null>(null);
  // Bumped by `retry` to re-run the subscribe-and-load effect.
  const [attempt, setAttempt] = useState(0);

  // Only the latest call may set the list: an older answer resolving late
  // would bring back stale rows.
  const latest = useRef(0);
  const apply = useCallback((id: number, result: ListResult) => {
    if (id !== latest.current) return;
    if (result.ok) {
      setLoad({ state: "loaded", list: result.list });
      setLastSuccessAt(new Date());
      setReloadError(null);
    } else {
      const { message } = result;
      setLoad((prev) => (prev.state === "loaded" ? prev : { state: "error", message }));
      setReloadError(message);
    }
  }, []);

  const reload = useCallback(async () => {
    const id = ++latest.current;
    apply(id, await settle(listTodo));
  }, [apply]);

  // Subscribe first and load only once the listener is registered, so no
  // event between the two is lost. WebView timers do not advance while the
  // laptop sleeps, so also reload when the window becomes visible or focused,
  // and once at the next local midnight as a fallback.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let midnight: ReturnType<typeof setTimeout> | undefined;

    const onVisible = () => {
      if (document.visibilityState === "visible") void reload();
    };
    const onFocus = () => void reload();
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("focus", onFocus);

    const arm = () => {
      midnight = setTimeout(() => {
        void reload();
        if (!cancelled) arm();
      }, msToNextMidnight());
    };
    arm();

    listen(TODO_CHANGED, () => {
      if (!cancelled) void reload();
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
        void reload();
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        apply(++latest.current, { ok: false, message: todoErrorMessage(e) });
      });

    return () => {
      cancelled = true;
      unlisten?.();
      clearTimeout(midnight);
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onFocus);
    };
  }, [reload, apply, attempt]);

  const retry = useCallback(() => {
    setLoad({ state: "loading" });
    setAttempt((n) => n + 1);
  }, []);

  const mutate = useCallback(
    async (call: () => Promise<TodoList>): MutationResult => {
      const id = ++latest.current;
      try {
        const list = await call();
        // A reload started meanwhile may have read the store before this
        // change landed, so its answer must not be the last word: read again.
        if (id === latest.current) apply(id, { ok: true, list });
        else void reload();
      } catch (e) {
        // A refusal means the shown list was stale, and a change can commit
        // before the follow-up read fails: either way, read the store again.
        void reload();
        return todoErrorMessage(e);
      }
      return null;
    },
    [apply, reload],
  );

  const addOwn = useCallback(
    (taskId: number): MutationResult => mutate(() => addTodoOwnTask(taskId)),
    [mutate],
  );
  const addGitlab = useCallback(
    (kind: ItemKind, reference: string): MutationResult =>
      mutate(() => addTodoGitlabItem(kind, reference)),
    [mutate],
  );
  const setTicked = useCallback(
    (id: number, ticked: boolean): MutationResult => mutate(() => setTodoTicked(id, ticked)),
    [mutate],
  );
  const remove = useCallback(
    (id: number): MutationResult => mutate(() => removeTodo(id)),
    [mutate],
  );

  return {
    load,
    lastSuccessAt,
    reloadError,
    retry,
    refresh: reload,
    addOwn,
    addGitlab,
    setTicked,
    remove,
  };
}
