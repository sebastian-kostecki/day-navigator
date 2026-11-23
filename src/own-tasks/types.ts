export type Space = "work" | "private";
export type Priority = "high" | "medium" | "low";

/** Mirrors `own_tasks::OwnTask`. Timestamps are local `YYYY-MM-DDTHH:MM:SS`. */
export interface OwnTask {
  id: number;
  space: Space;
  title: string;
  note: string;
  due: string | null;
  priority: Priority | null;
  createdAt: string;
  updatedAt: string;
  doneAt: string | null;
}

/** Mirrors `own_tasks::OwnTaskInput`; edit replaces every field. */
export interface OwnTaskInput {
  space: Space;
  title: string;
  note: string;
  due: string | null;
  priority: Priority | null;
}

/** Mirrors `store::StoreError`. */
export type StoreError =
  | { kind: "DirUnavailable"; detail: { kind: string } }
  | { kind: "Damaged"; detail: { reason: string } }
  | { kind: "Missing" }
  | { kind: "NewerSchema"; detail: { found: number; supported: number } }
  | { kind: "MigrationFailed"; detail: { version: number; reason: string } }
  | { kind: "Busy" }
  | { kind: "Io"; detail: { kind: string } }
  | { kind: "Sql"; detail: { reason: string } };

/** Mirrors `own_tasks::OwnTaskError`. */
export type OwnTaskError =
  | { kind: "Store"; detail: StoreError }
  | { kind: "TitleEmpty" }
  | { kind: "TitleTooLong"; detail: { max: number } }
  | { kind: "TitleMultiline" }
  | { kind: "NoteTooLong"; detail: { max: number } }
  | { kind: "InvalidDue" }
  | { kind: "NotFound" };
