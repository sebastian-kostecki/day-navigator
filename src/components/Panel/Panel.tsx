import type { ReactNode } from "react";
import "./Panel.css";

export type PanelProps = {
  title?: ReactNode;
  /** Right side of the head, mono: "[4/5]", "15 pozycji". */
  meta?: ReactNode;
  footer?: ReactNode;
  tone?: "default" | "danger";
  /** Heading element of the title; h2 by default. */
  headingLevel?: 1 | 2;
  className?: string;
  children?: ReactNode;
};

export function Panel({
  title,
  meta,
  footer,
  tone = "default",
  headingLevel = 2,
  className,
  children,
}: PanelProps) {
  const Heading = headingLevel === 1 ? "h1" : "h2";
  const classes = ["dn-panel", tone === "danger" && "is-danger", className]
    .filter(Boolean)
    .join(" ");
  return (
    <section className={classes}>
      {title ? (
        <div className="dn-panel-head">
          <Heading className="dn-panel-title">{title}</Heading>
          {meta ? <span className="dn-panel-meta">{meta}</span> : null}
        </div>
      ) : null}
      {children}
      {footer ? <div className="dn-panel-foot">{footer}</div> : null}
    </section>
  );
}
