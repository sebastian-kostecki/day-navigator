import type { ReactNode } from "react";
import { Button } from "../Button/Button";
import { Icon, type IconName } from "../Icon/Icon";
import "./SourceState.css";

const STATE_ICON: Record<"empty" | "calm" | "error", IconName> = {
  empty: "check-circle",
  calm: "wifi-off",
  error: "alert-triangle",
};

export type SourceStateProps = {
  /** empty = data with nothing in it; calm = unavailable, no alarm (role=status); error = real failure (role=alert). */
  kind: "empty" | "calm" | "error";
  title: string;
  children?: ReactNode;
  /** Ready text about the last successful refresh, rendered as given (no prefix). */
  lastOk?: string;
  /** Label of the secondary action button. */
  action?: string;
  onAction?: () => void;
  /** Disables the action (and marks it busy) when clicking it would change nothing. */
  actionDisabled?: boolean;
};

export function SourceState({
  kind,
  title,
  children,
  lastOk,
  action,
  onAction,
  actionDisabled,
}: SourceStateProps) {
  const role = kind === "error" ? "alert" : kind === "calm" ? "status" : undefined;
  return (
    <section className={`dn-state dn-state-${kind}`} role={role}>
      <Icon
        name={STATE_ICON[kind]}
        size={kind === "error" ? 24 : 36}
        strokeWidth={kind === "error" ? 2 : 1.5}
        className="dn-state-icon"
      />
      <div className="dn-state-body">
        <h2 className="dn-state-title">{title}</h2>
        {children ? <p className="dn-state-text">{children}</p> : null}
        {lastOk ? <p className="dn-state-last">{lastOk}</p> : null}
        {action ? (
          <Button
            variant="secondary"
            size="lg"
            onClick={onAction}
            disabled={actionDisabled}
            aria-busy={actionDisabled}
          >
            {action}
          </Button>
        ) : null}
      </div>
    </section>
  );
}
