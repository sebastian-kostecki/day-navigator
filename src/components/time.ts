/** Shared time formatting for the views: pure functions, local time, Polish labels. */

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

/** Local `HH:MM` of an instant. */
function clock(d: Date): string {
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

/** Local `HH:MM` of an instant given in UTC ms. */
export function formatClock(ms: number): string {
  return clock(new Date(ms));
}

/** Whole local calendar days from `ms` to `nowMs` (DST-safe; negative when `ms` is later). */
function calendarDays(ms: number, nowMs: number): number {
  const a = new Date(ms);
  const b = new Date(nowMs);
  const from = new Date(a.getFullYear(), a.getMonth(), a.getDate()).getTime();
  const to = new Date(b.getFullYear(), b.getMonth(), b.getDate()).getTime();
  return Math.round((to - from) / DAY);
}

/**
 * When something happened, for "odświeżone …" notes: "przed chwilą" (under a
 * minute, or in the future), "N min temu" (under an hour), "N godz. temu"
 * (earlier today), "wczoraj HH:MM" (the previous calendar day), else "DD.MM HH:MM".
 */
export function formatAgo(ms: number, nowMs: number): string {
  const diff = nowMs - ms;
  if (diff < MINUTE) return "przed chwilą";
  if (diff < HOUR) return `${Math.floor(diff / MINUTE)} min temu`;
  const days = calendarDays(ms, nowMs);
  if (days <= 0) return `${Math.floor(diff / HOUR)} godz. temu`;
  const d = new Date(ms);
  if (days === 1) return `wczoraj ${clock(d)}`;
  return `${pad2(d.getDate())}.${pad2(d.getMonth() + 1)} ${clock(d)}`;
}

/**
 * Age of an item in local calendar days: "dziś", "1 dzień", "N dni" (under a
 * week), "tydz." (7–13 days), "N tyg." (under 8 weeks), else "DD.MM.YYYY".
 */
export function formatAge(ms: number, nowMs: number): string {
  const days = calendarDays(ms, nowMs);
  if (days <= 0) return "dziś";
  if (days === 1) return "1 dzień";
  if (days < 7) return `${days} dni`;
  if (days < 14) return "tydz.";
  if (days < 56) return `${Math.floor(days / 7)} tyg.`;
  const d = new Date(ms);
  return `${pad2(d.getDate())}.${pad2(d.getMonth() + 1)}.${d.getFullYear()}`;
}

/** Local `DD.MM`, or `DD.MM.YYYY` when `ms` is not in the same local year as `nowMs`. */
export function formatDay(ms: number, nowMs: number): string {
  const d = new Date(ms);
  const day = `${pad2(d.getDate())}.${pad2(d.getMonth() + 1)}`;
  return d.getFullYear() === new Date(nowMs).getFullYear() ? day : `${day}.${d.getFullYear()}`;
}

/** `H:MM:SS` for a duration in milliseconds; negative counts as zero. */
export function formatHms(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return `${h}:${pad2(m)}:${pad2(s)}`;
}

/**
 * `H:MM` for a duration in milliseconds, in whole minutes. Unsigned: floored,
 * negative counts as zero. Signed: rounded toward zero, `+H:MM` / `−H:MM`
 * (U+2212), plain `0:00` under a minute either way.
 */
export function formatHm(ms: number, signed = false): string {
  const minutes = signed ? Math.trunc(ms / MINUTE) : Math.max(0, Math.floor(ms / MINUTE));
  const abs = Math.abs(minutes);
  const body = `${Math.floor(abs / 60)}:${pad2(abs % 60)}`;
  if (!signed || minutes === 0) return body;
  return `${minutes < 0 ? "−" : "+"}${body}`;
}
