import "./Segmented.css";

export type SegmentedOption = { value: string; label: string };

export type SegmentedProps = {
  options: SegmentedOption[];
  value: string;
  onChange?: (value: string) => void;
  /** Accessible name of the group. */
  label: string;
  /** sm = control height (default); md = matches the 40px fields. */
  size?: "sm" | "md";
  /** A request is pending: every button is disabled. */
  disabled?: boolean;
};

export function Segmented({
  options,
  value,
  onChange,
  label,
  size = "sm",
  disabled,
}: SegmentedProps) {
  return (
    <div role="group" aria-label={label} className={size === "md" ? "dn-seg dn-seg-md" : "dn-seg"}>
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            aria-pressed={on}
            className={on ? "dn-seg-btn is-on" : "dn-seg-btn"}
            disabled={disabled}
            onClick={() => onChange?.(o.value)}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
