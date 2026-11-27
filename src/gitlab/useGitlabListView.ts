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
  // `latest` is what the next save sends, `changed` marks fields the user set before the load
  // landed, `chain` queues the load and the saves so they run one at a time. Refs are touched
  // only in the setters and the mount effect, never during render.
  const latest = useRef<GitlabViewState>(DEFAULT_VIEW);
  const changed = useRef({ kind: false, showHidden: false });
  const chain = useRef<Promise<void>>(Promise.resolve());

  useEffect(() => {
    let cancelled = false;
    const loaded = getGitlabViewState()
      .then((stored) => {
        if (cancelled) return;
        const merged: GitlabViewState = {
          kind: changed.current.kind ? latest.current.kind : stored.kind,
          showHidden: changed.current.showHidden ? latest.current.showHidden : stored.showHidden,
        };
        latest.current = merged;
        setView(merged);
      })
      .catch(() => {});
    chain.current = loaded;
    return () => {
      cancelled = true;
    };
  }, []);

  const apply = useCallback((next: GitlabViewState, field: "kind" | "showHidden") => {
    changed.current[field] = true;
    latest.current = next;
    setView(next);
    chain.current = chain.current.then(() => setGitlabViewState(latest.current)).catch(() => {});
  }, []);

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
