// Pure display rules for the "Stan źródeł" view: pill text, glyphs, card
// texts and the head summary. No state, no backend calls.

import { formatAgo, type PillState } from "../components";
import type { AppDataDetails, SourceStatus } from "./types";

export type SourceFact = { label: string; value: string; tone?: "warn" };

export type Tone = "ok" | "calm" | "error";

export function toneOf(s: SourceStatus): Tone {
  switch (s.state) {
    case "ok":
      return "ok";
    case "notConfigured":
    case "unavailable":
      return "calm";
    case "error":
      return "error";
  }
}

export function pillState(s: SourceStatus): PillState {
  return toneOf(s);
}

export function pillText(s: SourceStatus): string {
  switch (s.state) {
    case "ok":
      return "OK";
    case "notConfigured":
      return "NIESKONFIGUROWANE";
    case "unavailable":
      return "NIEDOSTĘPNY";
    case "error":
      return "BŁĄD";
  }
}

export const GLYPH: Record<Tone, string> = { ok: "●", calm: "○", error: "▲" };

/** The three sources shown as big cards, in this order; every other source is a row. */
export type CardSpec = {
  id: string;
  /** Used only when the source is missing from the snapshot. */
  fallbackName: string;
  kindLabel: string;
};

export const CARD_SOURCES: CardSpec[] = [
  { id: "gitlab", fallbackName: "GitLab", kindLabel: "przez glab · tylko odczyt" },
  { id: "github", fallbackName: "GitHub", kindLabel: "przez gh · tylko odczyt" },
  { id: "app-data", fallbackName: "Dane aplikacji", kindLabel: "SQLite · kopie dzienne" },
];

export const CARD_IDS = new Set(CARD_SOURCES.map((c) => c.id));

const SETTINGS_KEY: Record<string, string> = { gitlab: "gitlab", github: "github" };

/** Main message of a card and an optional calm note under it. */
export function cardMessage(s: SourceStatus): { text: string; note?: string } {
  switch (s.state) {
    case "ok":
      return {
        text: s.id === "app-data" ? "Baza otwarta i sprawdzona." : "Odświeżone bez błędów.",
      };
    case "notConfigured": {
      const key = SETTINGS_KEY[s.id];
      return {
        text: key
          ? `Brak wpisu „${key}” w settings.json — to źródło jest wyłączone.`
          : "Brak wpisu w settings.json — to źródło jest wyłączone.",
      };
    }
    case "unavailable":
      return {
        text: s.message,
        note: "To spokojny stan: bez alarmu i bez powiadomień. Lista wróci po odzyskaniu połączenia.",
      };
    case "error":
      return { text: s.message };
  }
}

/** Short message for a compact row; only states that carry one. */
export function rowMessage(s: SourceStatus): string | null {
  return s.state === "error" || s.state === "unavailable" ? s.message : null;
}

/** "YYYY-MM-DD" to "DD.MM.YYYY" by reformatting the string (no timezone parsing). */
function formatBackupDate(date: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date);
  return m ? `${m[3]}.${m[2]}.${m[1]}` : date;
}

/** Facts for the "Dane aplikacji" card, after "Sprawdzono". */
export function appDataFacts(appData: AppDataDetails, nowMs: number): SourceFact[] {
  const facts: SourceFact[] = [];
  const { backups, schemaVersion } = appData;
  if (backups.state === "listed") {
    const tone = backups.stale ? ("warn" as const) : undefined;
    let last: string;
    if (backups.last === null) last = "brak";
    else if (backups.last.atMs != null) last = formatAgo(backups.last.atMs, nowMs);
    else last = formatBackupDate(backups.last.date);
    facts.push({ label: "Ostatnia kopia", value: last, tone });
    facts.push({ label: "Kopie", value: `${backups.kept} z ${backups.keep}` });
  } else {
    facts.push({ label: "Kopie", value: "nie można odczytać folderu", tone: "warn" });
  }
  if (schemaVersion !== null) facts.push({ label: "Schemat", value: `v${schemaVersion}` });
  return facts;
}

/** "1 błąd", "2 błędy", "5 błędów", "22 błędy". */
export function errorsLabel(n: number): string {
  if (n === 1) return "1 błąd";
  const d = n % 10;
  const dd = n % 100;
  if (d >= 2 && d <= 4 && !(dd >= 12 && dd <= 14)) return `${n} błędy`;
  return `${n} błędów`;
}

/**
 * "X z N dostępne · E błędów": X sources are OK out of N configured ones
 * (not-configured sources are left out of N), E are errors.
 */
export function summaryText(sources: SourceStatus[]): string {
  const configured = sources.filter((s) => s.state !== "notConfigured");
  const ok = configured.filter((s) => s.state === "ok").length;
  const errors = configured.filter((s) => s.state === "error").length;
  return `${ok} z ${configured.length} dostępne · ${errorsLabel(errors)}`;
}
