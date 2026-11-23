/** `H:MM:SS` for a non-negative duration in milliseconds. */
export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

/** Local `HH:MM` of an instant. */
export function formatClock(ms: number): string {
  return new Date(ms).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
}

function hm(totalMinutes: number): string {
  return `${Math.floor(totalMinutes / 60)}:${String(totalMinutes % 60).padStart(2, "0")}`;
}

/** `H:MM` for a non-negative duration, floored to whole minutes. */
export function formatHm(ms: number): string {
  return hm(Math.max(0, Math.floor(ms / 60000)));
}

/** `+H:MM` / `−H:MM` (U+2212), rounded toward zero; plain `0:00` under a minute. */
export function formatSigned(ms: number): string {
  const minutes = Math.trunc(ms / 60000);
  if (minutes === 0) return "0:00";
  return `${minutes < 0 ? "−" : "+"}${hm(Math.abs(minutes))}`;
}

/** `N dzień roboczy` / `N dni robocze` / `N dni roboczych` (Polish plural). */
export function formatWorkdays(n: number): string {
  if (n === 1) return "1 dzień roboczy";
  const units = n % 10;
  const tens = n % 100;
  if (units >= 2 && units <= 4 && (tens < 12 || tens > 14)) return `${n} dni robocze`;
  return `${n} dni roboczych`;
}

/** Polish month name and year for `YYYY-MM`. */
export function formatMonth(month: string): string {
  const [y, m] = month.split("-").map(Number);
  return new Date(y, m - 1, 1).toLocaleDateString("pl-PL", { month: "long", year: "numeric" });
}
