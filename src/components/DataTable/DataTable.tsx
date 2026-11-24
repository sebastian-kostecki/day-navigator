import type { ReactNode } from "react";
import "./DataTable.css";

function cx(...parts: (string | false | undefined)[]): string {
  return parts.filter(Boolean).join(" ");
}

export type DataTableProps = {
  /** Accessible name of the table. */
  label: string;
  /** The view sets `--dn-table-cols` (and optionally `--dn-table-pad-x`) here. */
  className?: string;
  children: ReactNode;
};

/** A grid table: structure and type only; the column template belongs to the view. */
export function DataTable({ label, className, children }: DataTableProps) {
  return (
    <div role="table" aria-label={label} className={cx("dn-table", className)}>
      {children}
    </div>
  );
}

export type DataTableRowProps = {
  head?: boolean;
  className?: string;
  children: ReactNode;
};

export function DataTableRow({ head, className, children }: DataTableRowProps) {
  return (
    <div
      role="row"
      className={cx("dn-table-row", head ? "dn-table-head label-caps" : "dn-table-body", className)}
    >
      {children}
    </div>
  );
}

export type DataTableCellProps = {
  header?: boolean;
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
      className={cx(
        "dn-table-cell",
        tone && `dn-table-cell-${tone}`,
        truncate && "dn-table-cell-truncate",
        className,
      )}
    >
      {children}
    </span>
  );
}
