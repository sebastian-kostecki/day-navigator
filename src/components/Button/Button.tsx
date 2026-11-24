import type { ButtonHTMLAttributes, ReactNode, Ref } from "react";
import { Icon, type IconName } from "../Icon/Icon";
import "./Button.css";

export type ButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> & {
  /** primary = one per view (accent fill); secondary = default; quiet = in-row tools; dashed = "add missing". */
  variant?: "primary" | "secondary" | "quiet" | "dashed";
  /** sm 36px, md 40px, lg 44px. */
  size?: "sm" | "md" | "lg";
  icon?: IconName;
  /** Icon-only square button: pass aria-label. */
  iconOnly?: boolean;
  /** Toggle state, rendered as aria-pressed. */
  pressed?: boolean;
  /** React 19 ref as a prop, e.g. to move focus to the button. */
  ref?: Ref<HTMLButtonElement>;
  children?: ReactNode;
};

export function Button({
  variant = "secondary",
  size = "md",
  icon,
  iconOnly,
  pressed,
  className,
  children,
  type = "button",
  ...rest
}: ButtonProps) {
  const classes = [
    "dn-btn",
    `dn-btn-${variant}`,
    `dn-btn-${size}`,
    iconOnly && "dn-btn-icon",
    pressed && "is-pressed",
    className,
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <button type={type} aria-pressed={pressed} {...rest} className={classes}>
      {icon ? <Icon name={icon} /> : null}
      {iconOnly ? null : children}
    </button>
  );
}
