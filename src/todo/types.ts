import type { StoreError } from "../components/storeError";
import type { Space } from "../own-tasks/types";
import type { ItemKind } from "../gitlab/types";

/** Mirrors `todo::TodoEntry` (tagged on `kind`, lowercase). */
export type TodoEntry =
  | { kind: "own"; taskId: number; space: Space; title: string }
  | {
      kind: "gitlab";
      itemKind: ItemKind;
      reference: string;
      title: string;
      webUrl: string | null;
      closedInSource: boolean;
    };

/** Mirrors `todo::TodoItem`. Timestamps are local `YYYY-MM-DDTHH:MM:SS`. */
export interface TodoItem {
  id: number;
  /** Local date `YYYY-MM-DD`. */
  pickedOn: string;
  carriedOver: boolean;
  ticked: boolean;
  pickedAt: string;
  entry: TodoEntry;
}

/** Mirrors `todo::TodoError`. */
export type TodoError =
  | { kind: "Store"; detail: StoreError }
  | { kind: "LimitReached"; detail: { limit: number } }
  | { kind: "AlreadyOnList" }
  | { kind: "TaskDone" }
  | { kind: "NotFound" }
  | { kind: "NotInGitlabList" }
  | { kind: "SettingsInvalid" }
  | { kind: "ConfigDirUnresolved" }
  | { kind: "Background" };

/** Mirrors `todo::TodoList`. `limit` is null exactly when `limitError` is set. */
export interface TodoList {
  /** Local date the list is for, `YYYY-MM-DD`. */
  day: string;
  limit: number | null;
  limitError: TodoError | null;
  openCount: number;
  items: TodoItem[];
}
