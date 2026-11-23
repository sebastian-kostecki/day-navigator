import type { StoreError } from "../components/storeError";

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

/** Mirrors `own_tasks::OwnTaskError`. */
export type OwnTaskError =
  | { kind: "Store"; detail: StoreError }
  | { kind: "TitleEmpty" }
  | { kind: "TitleTooLong"; detail: { max: number } }
  | { kind: "TitleMultiline" }
  | { kind: "NoteTooLong"; detail: { max: number } }
  | { kind: "InvalidDue" }
  | { kind: "NotFound" };
