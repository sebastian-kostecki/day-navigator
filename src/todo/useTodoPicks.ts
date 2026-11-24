import { useCallback, useMemo, useState } from "react";
import type { ItemKind } from "../gitlab/types";
import { todoErrorMessage } from "./api";
import { itemsByKey } from "./keys";
import type { TodoItem } from "./types";
import { useTodo } from "./useTodo";

/**
 * The "Do zrobienia" list as a source view needs it: lookup by item key,
 * limit state, and add/remove with a busy flag and the last action error.
 * Display only; the rules stay in Rust.
 */
export function useTodoPicks() {
  const { load, lastSuccessAt, reloadError, retry, refresh, addGitlab, addGithub, remove } =
    useTodo();
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const list = load.state === "loaded" ? load.list : null;
  const byKey = useMemo(() => (list ? itemsByKey(list) : null), [list]);

  const act = useCallback(async (run: () => Promise<string | null>) => {
    setNotice(null);
    setBusy(true);
    const result = await run();
    if (result !== null) setNotice(result);
    setBusy(false);
  }, []);

  const find = useCallback((key: string): TodoItem | undefined => byKey?.get(key), [byKey]);

  const limit = list?.limit ?? null;
  const ready = list !== null;
  return {
    load,
    reloadError,
    retry,
    refresh,
    lastSuccessAt,
    /** The list is loaded; until then `find` knows nothing and buttons should be disabled. */
    ready,
    find,
    /** `disabled` / `disabledHint` for a row's `TodoAddButton`. */
    buttonDisabled: busy || !ready,
    buttonHint: ready ? undefined : "Lista Do zrobienia niewczytana",
    /** Same rule as `TodoView`; an unloaded list counts as at the limit. */
    atLimit: list === null || list.limit === null || list.openCount >= list.limit,
    limit,
    limitError: list?.limitError ? todoErrorMessage(list.limitError) : null,
    busy,
    notice,
    addGitlab: (kind: ItemKind, reference: string) => act(() => addGitlab(kind, reference)),
    addGithub: (reference: string) => act(() => addGithub(reference)),
    remove: (id: number) => act(() => remove(id)),
  };
}

export type TodoPicks = ReturnType<typeof useTodoPicks>;
