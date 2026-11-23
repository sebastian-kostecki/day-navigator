import "./Segmented.css";

export type SegmentedOption = { value: string; label: string };

export type SegmentedProps = {
  options: SegmentedOption[];
  value: string;
  onChange?: (value: string) => void;
  /** Accessible name of the group. */
  label: string;
};

export function Segmented({ options, value, onChange, label }: SegmentedProps) {
  return (
    <div role="group" aria-label={label} className="dn-seg">
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            aria-pressed={on}
            className={on ? "dn-seg-btn is-on" : "dn-seg-btn"}
            onClick={() => onChange?.(o.value)}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
