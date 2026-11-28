import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  dismissGithubChange,
  getGithubSnapshot,
  githubHiddenErrorMessage,
  hideGithubIssue,
  isGithubHiddenStale,
  refreshGithub,
  unhideGithubIssue,
} from "./api";
import type { GithubSnapshot } from "./types";

function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}

/** `refreshOnMount: false` only reads the stored snapshot and listens, so a view that
 * just displays it never starts a `gh` round. */
export function useGithub({ refreshOnMount = true }: { refreshOnMount?: boolean } = {}) {
  const [snapshot, setSnapshot] = useState<GithubSnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  // References with an action in flight.
  const [busy, setBusy] = useState<ReadonlySet<string>>(new Set());
  // Bumped by `retry` to re-run the subscribe-and-load effect.
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    // Keep the newer snapshot by `seq`, never by the wall clock.
    const accept = (next: GithubSnapshot) =>
      setSnapshot((cur) => (!cur || next.seq > cur.seq ? next : cur));

    // Subscribe first and load only once the listener is registered, so no
    // event between the two is lost.
    listen<GithubSnapshot>("github-changed", (e) => {
      if (cancelled) return;
      setLoadError(null);
      accept(e.payload);
    })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
        return getGithubSnapshot().then((s) => {
          if (cancelled) return;
          setLoadError(null);
          accept(s);
          // At app start the thread's first round already runs; ask for one
          // more only when nothing is loading, so there is no back-to-back round.
          if (refreshOnMount && s.state.state !== "starting" && !s.refreshing) {
            return refreshGithub().catch((e) => {
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
      await refreshGithub();
      setRefreshError(null);
    } catch (e) {
      setRefreshError(errorText(e));
    }
  }, []);

  // No optimistic update: the new snapshot arrives as `github-changed`.
  const act = useCallback(
    async (reference: string, run: (reference: string) => Promise<void>) => {
      setActionError(null);
      setBusy((b) => new Set(b).add(reference));
      try {
        await run(reference);
      } catch (e) {
        setActionError(githubHiddenErrorMessage(e));
        // The row was stale: fetch a fresh list.
        if (isGithubHiddenStale(e)) void refresh();
      } finally {
        setBusy((b) => {
          const next = new Set(b);
          next.delete(reference);
          return next;
        });
      }
    },
    [refresh],
  );

  const hide = useCallback((reference: string) => act(reference, hideGithubIssue), [act]);
  const unhide = useCallback((reference: string) => act(reference, unhideGithubIssue), [act]);
  const dismiss = useCallback((reference: string) => act(reference, dismissGithubChange), [act]);

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
