import type { ReactNode } from "react";
import "./DataTable.css";

export type DataTableProps = {
  /** Accessible name of the table. */
  label: string;
  /**
   * Required: the view sets `--dn-table-cols` (the grid column template) on this
   * class, and optionally `--dn-table-pad-x`; without it the rows collapse.
   */
  className: string;
  children: ReactNode;
};

/** A grid table: structure and type only; the column template belongs to the view. */
export function DataTable({ label, className, children }: DataTableProps) {
  return (
    <div role="table" aria-label={label} className={`dn-table ${className}`}>
      {children}
    </div>
  );
}

export type DataTableRowProps = {
  /** Header row: caps labels, control height, stronger bottom line. */
  head?: boolean;
  className?: string;
  children: ReactNode;
};

export function DataTableRow({ head, className, children }: DataTableRowProps) {
  return (
    <div
      role="row"
      className={["dn-table-row", head ? "dn-table-head label-caps" : "dn-table-body", className]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </div>
  );
}

export type DataTableCellProps = {
  /** Column header cell (role=columnheader) instead of a data cell. */
  header?: boolean;
  /** muted = secondary text; faint = meta text (numbers, dates, labels). */
  tone?: "muted" | "faint";
  /** One line with an ellipsis. */
  truncate?: boolean;
  className?: string;
  title?: string;
  children?: ReactNode;
};

export function DataTableCell({
  header,
  tone,
  truncate,
  className,
  title,
  children,
}: DataTableCellProps) {
  return (
    <span
      role={header ? "columnheader" : "cell"}
      title={title}
      className={[
        "dn-table-cell",
        tone && `dn-table-cell-${tone}`,
        truncate && "dn-table-cell-truncate",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </span>
  );
}
