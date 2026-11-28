import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { markGithubSeen } from "./api";

function markSeen() {
  // Rust logs a failure; the only effect of one is that the flags show again.
  markGithubSeen().catch(() => {});
}

/**
 * Reports the end of a GitHub visit: when the active view leaves "github", and when
 * the window is hidden while "github" is active. The previous view lives in a ref, so a
 * StrictMode double effect stays idempotent (no mark on mount). Takes the view id only:
 * it never reads the work timer.
 */
export function useGithubSeenMark(view: string) {
  const previous = useRef(view);

  useEffect(() => {
    if (previous.current === "github" && view !== "github") markSeen();
    previous.current = view;
  }, [view]);

  useEffect(() => {
    if (view !== "github") return;
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
