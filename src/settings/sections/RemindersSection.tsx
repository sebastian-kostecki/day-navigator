import type { SectionErrors } from "../fieldErrors";
import type { RemindersForm } from "../types";
import { SwitchSetting, TextSetting } from "./controls";

type Props = {
  form: RemindersForm;
  onChange: (form: RemindersForm) => void;
  errors: SectionErrors;
};

export function RemindersSection({ form, onChange, errors }: Props) {
  return (
    <>
      <SwitchSetting
        label="Licznik biegnie za długo"
        hint="Potem co godzinę, dopóki licznik biegnie."
        checked={form.longRunningOn}
        onChange={(longRunningOn) => onChange({ ...form, longRunningOn })}
      />
      <TextSetting
        label="Licznik biegnie dłużej niż (godziny)"
        value={form.longRunningHours}
        onChange={(longRunningHours) => onChange({ ...form, longRunningHours })}
        error={errors.field("reminders.longRunningHours")}
        placeholder="10"
        inputMode="numeric"
        disabled={!form.longRunningOn}
      />
      <SwitchSetting
        label="Brak START-u w dzień roboczy"
        hint="Gdy dziś nie ma jeszcze wpisu; potem co godzinę."
        checked={form.notRunningAfterOn}
        onChange={(notRunningAfterOn) => onChange({ ...form, notRunningAfterOn })}
      />
      <TextSetting
        label="Brak START-u po godzinie (GG:MM)"
        value={form.notRunningAfter}
        onChange={(notRunningAfter) => onChange({ ...form, notRunningAfter })}
        error={errors.field("reminders.notRunningAfter")}
        placeholder="09:00"
        disabled={!form.notRunningAfterOn}
      />
    </>
  );
}
