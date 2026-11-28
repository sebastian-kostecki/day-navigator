import type { ReactNode } from "react";
import "./Flag.css";

export type FlagProps = {
  /** change = amber (Zmienione, Z <dnia>, Draft); info = blue (closed in the
   * source; Do review, an assigned MR that also awaits the user's review);
   * new = blue (Nowe, a freshly assigned item). */
  tone?: "change" | "info" | "new";
  children: ReactNode;
  className?: string;
};

export function Flag({ tone = "change", children, className }: FlagProps) {
  return (
    <span className={["dn-flag", `dn-flag-${tone}`, className].filter(Boolean).join(" ")}>
      {children}
    </span>
  );
}
