import { Notice } from "../../components";
import type { SourceStatus } from "../../health/types";
import type { SectionErrors } from "../fieldErrors";
import type { AutostartForm } from "../types";
import { SwitchSetting } from "./controls";

/** The outcome of the sync a save ran. */
function SyncStatus({ status }: { status: SourceStatus }) {
  switch (status.state) {
    case "ok":
      return (
        <Notice tone="info" compact>
          Autostart włączony, wpis w Windows jest na miejscu.
        </Notice>
      );
    case "notConfigured":
      return (
        <Notice tone="info" compact>
          Autostart wyłączony.
        </Notice>
      );
    case "unavailable":
      // A development build leaves the registry untouched and says so here.
      return (
        <Notice tone="calm" compact>
          {status.message}
        </Notice>
      );
    case "error":
      return (
        <Notice tone="error" compact>
          {status.message}
        </Notice>
      );
  }
}

type Props = {
  form: AutostartForm;
  onChange: (form: AutostartForm) => void;
  errors: SectionErrors;
  status: SourceStatus | null;
};

export function AutostartSection({ form, onChange, errors, status }: Props) {
  return (
    <>
      <SwitchSetting
        label="Uruchamiaj razem z Windows"
        hint="Aplikacja startuje do zasobnika systemowego."
        checked={form.enabled}
        onChange={(enabled) => onChange({ ...form, enabled })}
        error={errors.field("autostart.enabled")}
      />
      {status && <SyncStatus status={status} />}
    </>
  );
}
