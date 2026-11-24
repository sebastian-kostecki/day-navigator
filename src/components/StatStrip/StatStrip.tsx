import "./StatStrip.css";

export type Stat = {
  label: string;
  value: string;
  sub?: string;
  /** Value colour: info (blue, above norm / OK), warn (orange, below norm), soft (ink-soft); muted = soft. */
  tone?: "info" | "warn" | "soft" | "muted";
};

export type StatStripProps = { items: Stat[] };

export function StatStrip({ items }: StatStripProps) {
  return (
    <div className="dn-stats">
      {items.map((s) => (
        <div key={s.label} className="dn-stat">
          <span className="dn-stat-label">{s.label}</span>
          <span className={s.tone ? `dn-stat-value is-${s.tone}` : "dn-stat-value"}>{s.value}</span>
          {s.sub ? <span className="dn-stat-sub">{s.sub}</span> : null}
        </div>
      ))}
    </div>
  );
}
