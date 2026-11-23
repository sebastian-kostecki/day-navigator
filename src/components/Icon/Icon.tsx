import "./Icon.css";

export type IconName =
  | "plus"
  | "close"
  | "refresh"
  | "edit"
  | "hide"
  | "show"
  | "external"
  | "chevron-left"
  | "chevron-right"
  | "wifi-off"
  | "alert"
  | "check-circle"
  | "repo";

const PATHS: Record<IconName, string[]> = {
  plus: ["M12 5v14M5 12h14"],
  close: ["M6 6l12 12M18 6L6 18"],
  refresh: ["M20 11a8 8 0 1 0-2.3 5.7", "M20 4v7h-7"],
  edit: ["M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z"],
  hide: [
    "M3 3l18 18",
    "M10.6 5.1A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3.2 4.1M6.6 6.6C3.8 8.4 2 12 2 12s3.5 7 10 7a9.7 9.7 0 0 0 5.4-1.6",
    "M9.9 9.9a3 3 0 0 0 4.2 4.2",
  ],
  show: ["M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z", "M15 12a3 3 0 1 1-6 0a3 3 0 1 1 6 0"],
  external: ["M14 4h6v6", "M20 4l-9 9", "M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"],
  "chevron-left": ["M15 6l-6 6 6 6"],
  "chevron-right": ["M9 6l6 6-6 6"],
  "wifi-off": [
    "M2 8.5a15 15 0 0 1 20 0",
    "M5 12a10 10 0 0 1 9-2.6",
    "M8.5 15.5a5 5 0 0 1 4.2-1.4",
    "M12 19h.01",
    "M3 3l18 18",
  ],
  alert: [
    "M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z",
    "M12 9v4M12 17h.01",
  ],
  "check-circle": ["M21 12a9 9 0 1 1-18 0a9 9 0 1 1 18 0", "M8 12l3 3 5-6"],
  repo: ["M4 4h12a4 4 0 0 1 4 4v12H8a4 4 0 0 1-4-4z", "M4 16a4 4 0 0 1 4-4h12"],
};

export type IconProps = {
  name: IconName;
  /** Pixel size, default 14. */
  size?: number;
  /** Accessible name; without it the icon is decorative (aria-hidden). */
  label?: string;
  strokeWidth?: number;
  className?: string;
};

export function Icon({ name, size = 14, label, strokeWidth = 2, className }: IconProps) {
  const a11y = label
    ? ({ role: "img", "aria-label": label } as const)
    : ({ "aria-hidden": true } as const);
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className ? `dn-icon ${className}` : "dn-icon"}
      {...a11y}
    >
      {PATHS[name].map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  );
}
