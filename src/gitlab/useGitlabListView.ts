import { createContext, use, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getGitlabViewState, setGitlabViewState } from "./api";
import type { GitlabViewState, KindFilter } from "./types";
import { EMPTY_SELECTION, type ViewDimension, type ViewSelection } from "./viewFilter";

export type { KindFilter };

export type GitlabListView = {
  kind: KindFilter;
  setKind: (kind: KindFilter) => void;
  showHidden: boolean;
  setShowHidden: (update: (on: boolean) => boolean) => void;
  selection: ViewSelection;
  toggle: (dimension: ViewDimension, value: string) => void;
  clear: () => void;
};

export const GitlabListViewContext = createContext<GitlabListView | null>(null);

/** The list state of the GitLab view; needs a `GitlabListViewProvider` above it. */
export function useGitlabListView(): GitlabListView {
  const value = use(GitlabListViewContext);
  if (value === null) {
    throw new Error("useGitlabListView must be used inside a GitlabListViewProvider");
  }
  return value;
}

const DEFAULT_VIEW: GitlabViewState = { kind: "all", showHidden: false };

/**
 * The one source of the list state: called only by `GitlabListViewProvider`. The type and the
 * hidden expansion are remembered in the app data store; the view filter is in memory only.
 */
export function useGitlabListViewSource(): GitlabListView {
  const [view, setView] = useState<GitlabViewState>(DEFAULT_VIEW);
  const [selection, setSelection] = useState<ViewSelection>(EMPTY_SELECTION);
  // `latest` is what the next save sends, `changed` marks fields the user set before a load
  // landed, `loadedOk` says the stored value has been merged in, `chain` queues the loads and
  // the saves so they run one at a time. Refs are touched only in the setters, the load helper
  // and the mount effect, never during render.
  const latest = useRef<GitlabViewState>(DEFAULT_VIEW);
  const changed = useRef({ kind: false, showHidden: false });
  const loadedOk = useRef(false);
  const chain = useRef<Promise<void>>(Promise.resolve());

  // Reads the stored value and merges it under what the user already changed. Resolves true when
  // merged, false when the load failed or `isCancelled` says a StrictMode run replaced this one.
  const loadAndMerge = useCallback(async (isCancelled: () => boolean): Promise<boolean> => {
    try {
      const stored = await getGitlabViewState();
      if (isCancelled()) return false;
      const merged: GitlabViewState = {
        kind: changed.current.kind ? latest.current.kind : stored.kind,
        showHidden: changed.current.showHidden ? latest.current.showHidden : stored.showHidden,
      };
      latest.current = merged;
      loadedOk.current = true;
      setView(merged);
      return true;
    } catch {
      return false;
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    chain.current = loadAndMerge(() => cancelled).then(() => {});
    return () => {
      cancelled = true;
    };
  }, [loadAndMerge]);

  const apply = useCallback(
    (next: GitlabViewState, field: "kind" | "showHidden") => {
      changed.current[field] = true;
      latest.current = next;
      setView(next);
      chain.current = chain.current
        .then(async () => {
          // A failed mount load is retried first; without the stored value a save would
          // overwrite the other stored field with a default.
          if (!loadedOk.current && !(await loadAndMerge(() => false))) return;
          await setGitlabViewState(latest.current);
        })
        .catch(() => {});
    },
    [loadAndMerge],
  );

  const setKind = useCallback(
    (kind: KindFilter) => {
      if (kind === latest.current.kind) return;
      apply({ ...latest.current, kind }, "kind");
    },
    [apply],
  );
  const setShowHidden = useCallback(
    (update: (on: boolean) => boolean) => {
      const showHidden = update(latest.current.showHidden);
      if (showHidden === latest.current.showHidden) return;
      apply({ ...latest.current, showHidden }, "showHidden");
    },
    [apply],
  );

  const toggle = useCallback((dimension: ViewDimension, value: string) => {
    setSelection((sel) => {
      const current = sel[dimension];
      const next = current.includes(value)
        ? current.filter((v) => v !== value)
        : [...current, value];
      return { ...sel, [dimension]: next };
    });
  }, []);
  const clear = useCallback(() => setSelection(EMPTY_SELECTION), []);

  return useMemo(
    () => ({
      kind: view.kind,
      setKind,
      showHidden: view.showHidden,
      setShowHidden,
      selection,
      toggle,
      clear,
    }),
    [view, setKind, setShowHidden, selection, toggle, clear],
  );
}
