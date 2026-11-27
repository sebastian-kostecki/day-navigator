import type { StoreError } from "../components/storeError";
import type { ItemKind } from "../gitlab/types";

/** Mirrors `events::ItemSource`. */
export type EventItemSource = "own" | "gitlab" | "github";

/** Mirrors `health::SourceId` (its stable key). */
export type EventSourceId =
  "settings" | "app-data" | "gitlab" | "github" | "reminders" | "autostart";

/** Mirrors `events::Subject`; `reference` is null for an own task. */
interface EventSubject {
  source: EventItemSource;
  reference: string | null;
  title: string;
}

/** Mirrors `events::EventItem` (tagged on `kind`). `at` is local `YYYY-MM-DDTHH:MM:SS`. */
export type EventEntry = { id: number; at: string } & (
  | { kind: "assigned"; itemKind: ItemKind; reference: string; title: string }
  | ({ kind: "ticked" } & EventSubject)
  | ({ kind: "removed" } & EventSubject)
  | ({ kind: "closedInSource" } & EventSubject)
  | { kind: "workStarted" }
  | { kind: "workStopped" }
  | { kind: "sourceError"; sourceId: EventSourceId; label: string; reason: string }
  | { kind: "sourceRecovered"; sourceId: EventSourceId; label: string }
);

/** Mirrors `events::EventLog`: today's events, newest first, at most 200. */
export interface EventLog {
  events: EventEntry[];
  truncated: boolean;
}

/** Mirrors `events::EventError`. */
export type EventError = { kind: "Store"; detail: StoreError } | { kind: "Background" };

/** Emitted (no payload) after events were stored. */
export const EVENTS_CHANGED = "events-changed";
