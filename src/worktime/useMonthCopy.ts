import { useState } from "react";
import { workMonthReportText, workTimeErrorMessage } from "./api";

type Outcome = { month: string; copied: true } | { month: string; error: string };

function clipboardMessage(e: unknown): string {
  const name = e instanceof Error ? e.name : "nieznany błąd";
  return `Nie udało się skopiować do schowka (${name})`;
}

/**
 * One copy of the shown month's report to the clipboard. The outcome belongs to
 * the month it was clicked in, so a confirmation never shows on another month.
 * The text is never kept in state and never logged.
 */
export function useMonthCopy(month: string) {
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);

  async function copy() {
    setOutcome(null);
    setBusy(true);
    try {
      let text: string;
      try {
        text = await workMonthReportText(`${month}-01`);
      } catch (e) {
        setOutcome({ month, error: workTimeErrorMessage(e) });
        return;
      }
      try {
        await navigator.clipboard.writeText(text);
      } catch (e) {
        setOutcome({ month, error: clipboardMessage(e) });
        return;
      }
      setOutcome({ month, copied: true });
    } finally {
      setBusy(false);
    }
  }

  const shown = outcome !== null && outcome.month === month ? outcome : null;
  return {
    copy,
    busy,
    copied: shown !== null && "copied" in shown,
    error: shown !== null && "error" in shown ? shown.error : null,
  };
}
