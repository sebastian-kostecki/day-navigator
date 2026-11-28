import type { SectionErrors } from "../fieldErrors";
import type { WorkTimeForm } from "../types";
import { SwitchSetting, TextSetting } from "./controls";

type Props = {
  form: WorkTimeForm;
  onChange: (form: WorkTimeForm) => void;
  errors: SectionErrors;
};

export function WorkTimeSection({ form, onChange, errors }: Props) {
  return (
    <>
      <SwitchSetting
        label="Polskie święta"
        hint="Święta państwowe nie liczą się do normy."
        checked={form.holidays}
        onChange={(holidays) => onChange({ ...form, holidays })}
        error={errors.field("workTime.holidays")}
      />
      <TextSetting
        label="Dzienna norma (godziny)"
        value={form.dailyNormHours}
        onChange={(dailyNormHours) => onChange({ ...form, dailyNormHours })}
        error={errors.field("workTime.dailyNormHours")}
        placeholder="8"
        inputMode="decimal"
      />
      <p className="meta dn-settings-hint">
        Zmiana normy przelicza wszystkie miesiące, także zamknięte.
      </p>
    </>
  );
}
