import type { ReactNode } from "react";
import "./Chip.css";

export type ChipProps = {
  /** warn = amber text, soft = soft ink, faint = faint ink. Omitted: inherited colour. */
  tone?: "warn" | "soft" | "faint";
  children: ReactNode;
  className?: string;
};

/** A small outlined value marker (priority); `Flag` stays the change signal. */
export function Chip({ tone, children, className }: ChipProps) {
  return (
    <span className={["dn-chip", tone && `dn-chip-${tone}`, className].filter(Boolean).join(" ")}>
      {children}
    </span>
  );
}
