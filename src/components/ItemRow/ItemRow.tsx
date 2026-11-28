import type { ReactNode } from "react";
import { Flag, type FlagProps } from "../Flag/Flag";
import { SourceChip, type SourceKind } from "../SourceChip/SourceChip";
import "./ItemRow.css";

type RowBase = {
  title: ReactNode;
  /** Source chip before the ref; no chip when absent. */
  source?: SourceKind;
  /** Issue / MR number: "!418", "#27". */
  refId?: string;
  flag?: ReactNode;
  flagTone?: FlagProps["tone"];
  /** Further flags or notes, right after `flag`. */
  extra?: ReactNode;
  /** Hidden look: the whole row is dimmed. */
  dimmed?: boolean;
  /** Label names, shown comma-separated; nothing is rendered when absent or empty. */
  labels?: string[];
  /** Right-aligned mono meta: due date, "2 godz.". */
  meta?: ReactNode;
  selected?: boolean;
  /** Row actions: quiet icon Buttons. */
  children?: ReactNode;
};

type WithCheckbox = RowBase & {
  /** Required with a checkbox: links the label to it. */
  id: string;
  /** Ticked: checkbox checked, title struck through and dimmed. */
  done?: boolean;
  /** Same as `done` (either one ticks the row). */
  checked?: boolean;
  onToggle: () => void;
  /** Accessible name of the checkbox. */
  checkboxLabel?: string;
  checkboxDisabled?: boolean;
};

type WithoutCheckbox = RowBase & {
  id?: string;
  done?: undefined;
  checked?: undefined;
  onToggle?: undefined;
  checkboxLabel?: undefined;
  checkboxDisabled?: undefined;
};

export type ItemRowProps = WithCheckbox | WithoutCheckbox;

export function ItemRow(props: ItemRowProps) {
  const { title, source, refId, flag, flagTone, extra, dimmed, labels, meta, selected, children } =
    props;
  const joinedLabels = labels && labels.length > 0 ? labels.join(", ") : null;
  const done = props.done === true || props.checked === true;
  const classes = ["dn-row", selected && "is-selected", done && "is-done", dimmed && "is-dimmed"]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={classes}>
      {props.onToggle ? (
        <input
          type="checkbox"
          id={props.id}
          className="dn-check"
          checked={done}
          disabled={props.checkboxDisabled}
          aria-label={props.checkboxLabel}
          onChange={props.onToggle}
        />
      ) : null}
      {source ? <SourceChip source={source} /> : null}
      {refId ? <span className="dn-row-ref">{refId}</span> : null}
      {props.onToggle ? (
        <label htmlFor={props.id} className="dn-row-title">
          {title}
        </label>
      ) : (
        <span className="dn-row-title">{title}</span>
      )}
      {flag ? <Flag tone={flagTone}>{flag}</Flag> : null}
      {extra}
      {joinedLabels ? (
        <span className="dn-row-labels code" title={joinedLabels}>
          <span className="dn-sr-only">Etykiety: </span>
          {joinedLabels}
        </span>
      ) : null}
      {meta ? <span className="dn-row-meta">{meta}</span> : null}
      {children ? <div className="dn-row-actions">{children}</div> : null}
    </div>
  );
}
