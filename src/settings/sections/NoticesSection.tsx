import type { SectionErrors } from "../fieldErrors";
import type { NoticesForm } from "../types";
import { SwitchSetting } from "./controls";

const SWITCHES = [
  {
    key: "newAssignments",
    label: "Nowe przypisania",
    hint: "Nowe zgłoszenia i merge requesty z GitLaba, także prośby o review.",
  },
  {
    key: "sourceProblems",
    label: "Problemy ze źródłami (GitLab, GitHub)",
    hint: "Gdy GitLab lub GitHub przestaje działać i gdy wraca.",
  },
  {
    key: "daySummary",
    label: "Podsumowanie dnia",
    hint: "Po zatrzymaniu licznika z menu w zasobniku.",
  },
] as const;

type Props = {
  form: NoticesForm;
  onChange: (form: NoticesForm) => void;
  errors: SectionErrors;
};

export function NoticesSection({ form, onChange, errors }: Props) {
  return (
    <>
      {SWITCHES.map(({ key, label, hint }) => (
        <SwitchSetting
          key={key}
          label={label}
          hint={hint}
          checked={form[key]}
          onChange={(on) => onChange({ ...form, [key]: on })}
          error={errors.field(`notices.${key}`)}
        />
      ))}
      <p className="meta dn-settings-hint">
        Alarm o zatrzymanym sprawdzaniu źródeł jest zawsze włączony.
      </p>
    </>
  );
}
