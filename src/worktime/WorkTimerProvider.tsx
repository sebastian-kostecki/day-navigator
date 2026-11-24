import { useMemo, type ReactNode } from "react";
import { WorkTimerContext, useWorkTimerSource } from "./useWorkTimer";

/** Runs the timer source once and shares it with the header and every view. */
export function WorkTimerProvider({ children }: { children: ReactNode }) {
  const { timer, nowMs, busy, actionError, start, stop, retry } = useWorkTimerSource();
  const value = useMemo(
    () => ({ timer, nowMs, busy, actionError, start, stop, retry }),
    [timer, nowMs, busy, actionError, start, stop, retry],
  );
  return <WorkTimerContext value={value}>{children}</WorkTimerContext>;
}
