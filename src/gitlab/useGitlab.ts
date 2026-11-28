import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  dismissGitlabChange,
  dismissGitlabMergeRequestChange,
  getGitlabSnapshot,
  hiddenIssueErrorMessage,
  hideGitlabIssue,
  hideGitlabMergeRequest,
  isHiddenIssueStale,
  refreshGitlab,
  unhideGitlabIssue,
  unhideGitlabMergeRequest,
} from "./api";
import type { GitlabSnapshot, ItemKind } from "./types";

function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}

/** Issue and merge request ids are separate numbering spaces; the busy set keys both. */
export function busyKey(kind: ItemKind, id: number): string {
  return `${kind}:${id}`;
}

const HIDE: Record<ItemKind, (id: number) => Promise<void>> = {
  issue: hideGitlabIssue,
  mergeRequest: hideGitlabMergeRequest,
};
const UNHIDE: Record<ItemKind, (id: number) => Promise<void>> = {
  issue: unhideGitlabIssue,
  mergeRequest: unhideGitlabMergeRequest,
};
const DISMISS: Record<ItemKind, (id: number) => Promise<void>> = {
  issue: dismissGitlabChange,
  mergeRequest: dismissGitlabMergeRequestChange,
};

/** `refreshOnMount: false` only reads the stored snapshot and listens, so a view that
 * just displays it never starts a `glab` round. */
export function useGitlab({ refreshOnMount = true }: { refreshOnMount?: boolean } = {}) {
  const [snapshot, setSnapshot] = useState<GitlabSnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  // Row actions: the new snapshot arrives through `gitlab-changed`, so a
  // success changes no local list; a failure becomes an inline notice that
  // lasts until the next action or the next finished round.
  const [actionError, setActionError] = useState<string | null>(null);
  const [busy, setBusy] = useState<ReadonlySet<string>>(new Set());
  // Bumped by `retry` to re-run the subscribe-and-load effect.
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    // Keep the newer snapshot by `seq`, never by the wall clock.
    const accept = (next: GitlabSnapshot) =>
      setSnapshot((cur) => (!cur || next.seq > cur.seq ? next : cur));

    // Subscribe first and load only once the listener is registered, so no
    // event between the two is lost.
    listen<GitlabSnapshot>("gitlab-changed", (e) => {
      if (cancelled) return;
      setLoadError(null);
      // A finished round replaces the list the failed action was based on.
      if (!e.payload.refreshing) setActionError(null);
      accept(e.payload);
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
        return getGitlabSnapshot().then((s) => {
          if (cancelled) return;
          setLoadError(null);
          accept(s);
          // At app start the thread's first round already runs; ask for one
          // more only when nothing is loading, so there is no back-to-back round.
          if (refreshOnMount && s.state.state !== "starting" && !s.refreshing) {
            return refreshGitlab().catch((e) => {
              if (!cancelled) setRefreshError(errorText(e));
            });
          }
        });
      })
      .catch((e) => {
        if (!cancelled) setLoadError(errorText(e));
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [attempt, refreshOnMount]);

  /** Re-runs the subscribe and the load that failed. */
  const retry = useCallback(() => setAttempt((n) => n + 1), []);

  // A failed refresh request is not a failed load: the stored snapshot is
  // still readable, so it gets its own error.
  const refresh = useCallback(async () => {
    try {
      await refreshGitlab();
      setRefreshError(null);
    } catch (e) {
      setRefreshError(errorText(e));
    }
  }, []);

  const act = useCallback(
    async (kind: ItemKind, id: number, run: (id: number) => Promise<void>) => {
      const key = busyKey(kind, id);
      setActionError(null);
      setBusy((b) => new Set(b).add(key));
      try {
        await run(id);
      } catch (e) {
        setActionError(hiddenIssueErrorMessage(e, kind));
        // The row was stale (own-tasks pattern): fetch a fresh list.
        if (isHiddenIssueStale(e)) void refresh();
      } finally {
        setBusy((b) => {
          const next = new Set(b);
          next.delete(key);
          return next;
        });
      }
    },
    [refresh],
  );

  const hide = useCallback((kind: ItemKind, id: number) => act(kind, id, HIDE[kind]), [act]);
  const unhide = useCallback((kind: ItemKind, id: number) => act(kind, id, UNHIDE[kind]), [act]);
  const dismiss = useCallback((kind: ItemKind, id: number) => act(kind, id, DISMISS[kind]), [act]);

  return {
    snapshot,
    loadError,
    refresh,
    refreshError,
    retry,
    actionError,
    busy,
    hide,
    unhide,
    dismiss,
  };
}
