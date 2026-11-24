/** Polish plural: `one` for 1, `few` for 2–4 (not 12–14), `many` otherwise. */
export function plural(n: number, one: string, few: string, many: string): string {
  if (n === 1) return one;
  const units = n % 10;
  const tens = n % 100;
  return units >= 2 && units <= 4 && (tens < 12 || tens > 14) ? few : many;
}
