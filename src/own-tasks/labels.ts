import type { Priority, Space } from "./types";

export const SPACE_LABEL: Record<Space, string> = { work: "Praca", private: "Prywatne" };

export const SPACE_OPTIONS: { value: Space; label: string }[] = [
  { value: "work", label: "Praca" },
  { value: "private", label: "Prywatne" },
];

// `Field` renders select options with the label as the value, so the
// priority select works on labels and these maps translate to the domain value.
export const NO_PRIORITY = "—";
export const PRIORITY_LABEL: Record<Priority, string> = {
  high: "wysoki",
  medium: "normalny",
  low: "niski",
};
export const PRIORITY_OPTIONS = [NO_PRIORITY, ...Object.values(PRIORITY_LABEL)];

export function priorityFromLabel(label: string): Priority | "" {
  const found = (Object.keys(PRIORITY_LABEL) as Priority[]).find(
    (p) => PRIORITY_LABEL[p] === label,
  );
  return found ?? "";
}

export function priorityToLabel(priority: Priority | ""): string {
  return priority === "" ? NO_PRIORITY : PRIORITY_LABEL[priority];
}

export const TITLE_MAX = 200;
export const NOTE_MAX = 2000;

// No `maxLength` on the fields on purpose: the browser would silently cut
// pasted text and counts UTF-16 units, not characters. Rust is the only limit;
// the counters count characters the same way.
export const charCount = (value: string) => [...value].length;

/** Today as a local `YYYY-MM-DD`, comparable with a task's `due`. */
export function todayIso(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** `dziś`, `27.11` this year, `27.11.2027` in another year. */
export function formatDue(due: string, today: string): string {
  if (due === today) return "dziś";
  const [year, month, day] = due.split("-");
  if (!year || !month || !day) return due;
  return year === today.slice(0, 4) ? `${day}.${month}` : `${day}.${month}.${year}`;
}
