import type { ReactNode } from "react";
import "./StatusPill.css";

export type PillState = "ok" | "calm" | "error";

const PILL: Record<PillState, { glyph: string; text: string }> = {
  ok: { glyph: "●", text: "OK" },
  calm: { glyph: "○", text: "NIEDOSTĘPNY" },
  error: { glyph: "▲", text: "BŁĄD" },
};

export type StatusPillProps = {
  /** ok ●, calm ○ (no VPN), error ▲. */
  state: PillState;
  /** Defaults to OK / NIEDOSTĘPNY / BŁĄD. */
  children?: ReactNode;
  /** Show the ●/○/▲ glyph before the text; false renders the text only. Default true. */
  glyph?: boolean;
  className?: string;
};

export function StatusPill({ state, children, glyph = true, className }: StatusPillProps) {
  const p = PILL[state];
  return (
    <span className={["dn-pill", `dn-pill-${state}`, className].filter(Boolean).join(" ")}>
      {glyph ? <span aria-hidden="true">{p.glyph}</span> : null}
      {children ?? p.text}
    </span>
  );
}
