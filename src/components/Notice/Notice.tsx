import type { ReactNode } from "react";
import { Button } from "../Button/Button";
import { Icon, type IconName } from "../Icon/Icon";
import "./Notice.css";

export type NoticeTone = "calm" | "info" | "warn" | "error";

const TONE_ICON: Record<NoticeTone, IconName> = {
  calm: "wifi-off",
  info: "check-circle",
  warn: "alert-triangle",
  error: "alert-triangle",
};

export type NoticeProps = {
  /** calm = unavailable, never red; info = done / FYI; warn = degraded or misconfigured; error = an action or part failed. */
  tone: NoticeTone;
  children: ReactNode;
  /** Optional bold line above the text. */
  title?: string;
  /** Label of the small secondary button on the right. */
  action?: string;
  onAction?: () => void;
  actionDisabled?: boolean;
  /** Inside a Panel body: keeps the panel's side padding around the banner. */
  inset?: boolean;
  /** Smaller type and padding: under a form field, in a row, in the header. */
  compact?: boolean;
  /** Defaults to alert for error, status otherwise; null when an outer live region announces it. */
  role?: "alert" | "status" | null;
  className?: string;
};

/** A compact banner for in-view messages; full source failures stay ErrorState / SourceState. */
export function Notice({
  tone,
  children,
  title,
  action,
  onAction,
  actionDisabled,
  inset,
  compact,
  role,
  className,
}: NoticeProps) {
  const resolvedRole = role === undefined ? (tone === "error" ? "alert" : "status") : role;
  const classes = [
    "dn-notice",
    `dn-notice-${tone}`,
    inset && "is-inset",
    compact && "is-compact",
    action && "has-action",
    className,
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={classes} role={resolvedRole ?? undefined}>
      <Icon name={TONE_ICON[tone]} size={compact ? 14 : 16} className="dn-notice-icon" />
      <div className="dn-notice-body">
        {title ? <p className="dn-notice-title">{title}</p> : null}
        <div className="dn-notice-text">{children}</div>
      </div>
      {action ? (
        <Button
          variant="secondary"
          size="sm"
          className="dn-notice-action"
          disabled={actionDisabled}
          onClick={onAction}
        >
          {action}
        </Button>
      ) : null}
    </div>
  );
}
