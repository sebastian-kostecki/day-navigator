import type { StoreError } from "../components/storeError";
import type { TodoItem } from "../todo/types";

/** Mirrors `day_summary::DaySummary`. `day` is the list's local `YYYY-MM-DD`. */
export interface DaySummary {
  day: string;
  todayMs: number;
  monthDeviationMs: number;
  done: TodoItem[];
  carryOver: TodoItem[];
}

/** Mirrors `day_summary::DaySummaryError`. */
export type DaySummaryError =
  { kind: "Store"; detail: StoreError } | { kind: "InvalidLocalTime" } | { kind: "Background" };
