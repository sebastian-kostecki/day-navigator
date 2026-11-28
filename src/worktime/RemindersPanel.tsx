import { Button, Panel, StatusPill } from "../components";
import { ErrorState } from "../components/ErrorState";
import { useSettingsDialog } from "../settings/useSettingsDialog";
import { formatClock, formatHm } from "./format";
import { useReminderSettings } from "./useReminderSettings";

const HOUR_MS = 3_600_000;

function ReminderRow({
  label,
  hint,
  value,
  fromFile,
}: {
  label: string;
  hint: string;
  /** `null` means the reminder is off. */
  value: string | null;
  fromFile: boolean;
}) {
  const on = value !== null;
  return (
    <div className="wt-reminder">
      <dt>{label}</dt>
      <dd className="wt-reminder-value">
        <span className="code">{on ? value : "—"}</span>
        <StatusPill state={on ? "ok" : "calm"} glyph={false}>
          {on ? "WŁ." : "WYŁ."}
        </StatusPill>
        {!fromFile && <span className="meta">domyślnie</span>}
      </dd>
      <dd className="meta wt-reminder-hint">{hint}</dd>
    </div>
  );
}

export function RemindersPanel() {
  const { state, retry } = useReminderSettings();
  const { openAt } = useSettingsDialog();

  let body: React.ReactNode;
  if (state.state === "loading") {
    body = <p className="dn-loading wt-panel-body">Ładowanie…</p>;
  } else if (state.state === "error") {
    body = (
      <div className="wt-panel-body">
        <ErrorState
          title="Nie udało się wczytać przypomnień"
          message={state.message}
          lastSuccess={
            state.lastOkAt === null ? undefined : `ostatnio OK ${formatClock(state.lastOkAt)}`
          }
          onRetry={retry}
        />
      </div>
    );
  } else {
    const { longRunning, notRunningAfter } = state.thresholds;
    body = (
      <dl className="wt-reminders">
        <ReminderRow
          label="Licznik biegnie dłużej niż"
          hint="potem co godzinę, dopóki licznik biegnie"
          value={longRunning.hours === null ? null : formatHm(longRunning.hours * HOUR_MS)}
          fromFile={longRunning.fromFile}
        />
        <ReminderRow
          label="Brak START-u w dzień roboczy po"
          hint="gdy dziś nie ma jeszcze wpisu; potem co godzinę"
          value={notRunningAfter.time}
          fromFile={notRunningAfter.fromFile}
        />
      </dl>
    );
  }

  return (
    <Panel
      title="Przypomnienia"
      footer={
        <Button variant="quiet" size="sm" icon="settings" onClick={() => openAt("reminders")}>
          Ustawienia
        </Button>
      }
    >
      {body}
    </Panel>
  );
}
