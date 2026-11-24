import "./HoursBar.css";

export type HoursBarProps = {
  /** Worked hours. */
  hours: number;
  /** The day's norm in hours, from the backend; `0` hides the norm tick. */
  norm: number;
  /** Scale of the bar in hours. */
  max?: number;
  /** Accent fill for the current day. */
  today?: boolean;
  /** Accessible text, e.g. "6:30 na tle normy 8:00". */
  label: string;
};

/** Hours worked against the day's norm: a fill on a track, a tick at the norm. */
export function HoursBar({ hours, norm, max = 10, today, label }: HoursBarProps) {
  const pct = Math.max(0, Math.min(100, (hours / max) * 100));
  return (
    <div className="dn-hours" role="img" aria-label={label}>
      <div
        className={today ? "dn-hours-fill is-today" : "dn-hours-fill"}
        style={{ width: `${pct}%` }}
      />
      {norm > 0 && <div className="dn-hours-norm" style={{ left: `${(norm / max) * 100}%` }} />}
    </div>
  );
}
