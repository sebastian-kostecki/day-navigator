import type { CSSProperties } from "react";
import "./LabelChip.css";

export type LabelChipProps = {
  name: string;
  /** Label background (validated `#rgb` / `#rrggbb`); null: neutral chip. */
  color: string | null;
  /** Text colour on the label background; null: neutral ink. */
  textColor: string | null;
  pressed?: boolean;
  /** Given: the chip is a toggle button; omitted: a plain span. */
  onToggle?: () => void;
  /** Accessible name of the toggle button. */
  toggleLabel?: string;
};

/** `key::value` split on the last `::` when both parts are non-empty. */
function splitScoped(name: string): { key: string; value: string } | null {
  const at = name.lastIndexOf("::");
  if (at <= 0) return null;
  const key = name.slice(0, at);
  const value = name.slice(at + 2);
  return value === "" ? null : { key, value };
}

/** A GitLab label: filled in its colour; scoped labels get a filled key and an outlined value. */
export function LabelChip({
  name,
  color,
  textColor,
  pressed,
  onToggle,
  toggleLabel,
}: LabelChipProps) {
  const style: Record<string, string> = {};
  if (color) style["--dn-label-bg"] = color;
  if (textColor) style["--dn-label-ink"] = textColor;
  const scoped = splitScoped(name);
  const className = ["dn-label-chip", scoped && "dn-label-chip-scoped", pressed && "is-pressed"]
    .filter(Boolean)
    .join(" ");
  const content = scoped ? (
    <>
      <span className="dn-label-chip-key">{scoped.key}</span>
      <span className="dn-label-chip-value">{scoped.value}</span>
    </>
  ) : (
    <span className="dn-label-chip-text">{name}</span>
  );

  if (onToggle) {
    return (
      <button
        type="button"
        className={className}
        style={style as CSSProperties}
        title={name}
        aria-pressed={pressed ?? false}
        aria-label={toggleLabel}
        onClick={onToggle}
      >
        {content}
      </button>
    );
  }
  return (
    <span className={className} style={style as CSSProperties} title={name}>
      {content}
    </span>
  );
}
