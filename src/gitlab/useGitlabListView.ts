import { createContext, use, useCallback, useMemo, useState } from "react";
import { EMPTY_SELECTION, type ViewDimension, type ViewSelection } from "./viewFilter";

export type KindFilter = "all" | "issue" | "mergeRequest";

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

/** The one source of the list state: called only by `GitlabListViewProvider`. In memory only. */
export function useGitlabListViewSource(): GitlabListView {
  const [kind, setKind] = useState<KindFilter>("all");
  const [showHidden, setShowHidden] = useState(false);
  const [selection, setSelection] = useState<ViewSelection>(EMPTY_SELECTION);

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
    () => ({ kind, setKind, showHidden, setShowHidden, selection, toggle, clear }),
    [kind, showHidden, selection, toggle, clear],
  );
}
