import { Button, Notice } from "../components";
import { useMonthCopy } from "./useMonthCopy";

/** Copies the shown month's report to the clipboard as tab-separated text. */
export function MonthCopyButton({ month, enabled }: { month: string; enabled: boolean }) {
  const { copy, busy, copied, error } = useMonthCopy(month);
  return (
    <div className="wt-month-copy">
      <div role="status">
        {copied && (
          <Notice tone="info" compact role={null}>
            Skopiowano.
          </Notice>
        )}
      </div>
      {error !== null && (
        <Notice tone="error" compact>
          {error}
        </Notice>
      )}
      <Button variant="quiet" size="sm" disabled={!enabled || busy} onClick={() => void copy()}>
        {busy ? "Kopiuję…" : "Kopiuj miesiąc"}
      </Button>
    </div>
  );
}
