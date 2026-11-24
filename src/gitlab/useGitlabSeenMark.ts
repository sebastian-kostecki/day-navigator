import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { markGitlabSeen } from "./api";

function markSeen() {
  // Rust logs a failure; the only effect of one is that the flags show again.
  markGitlabSeen().catch(() => {});
}

/**
 * Reports the end of a GitLab visit: when the active view leaves "gitlab", and when
 * the window is hidden while "gitlab" is active. The previous view lives in a ref, so a
 * StrictMode double effect stays idempotent (no mark on mount).
 */
export function useGitlabSeenMark(view: string) {
  const previous = useRef(view);

  useEffect(() => {
    if (previous.current === "gitlab" && view !== "gitlab") markSeen();
    previous.current = view;
  }, [view]);

  useEffect(() => {
    if (view !== "gitlab") return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen("main-hidden", markSeen)
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [view]);
}
