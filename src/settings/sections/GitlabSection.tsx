import type { SectionErrors } from "../fieldErrors";
import type { DimensionForm, FilterForm, GitlabForm } from "../types";
import { ListSetting, TextSetting } from "./controls";

const FILTER_MAX = 100;

const DIMENSIONS = [
  { key: "projects", label: "Projekty", placeholder: "grupa/projekt" },
  { key: "labels", label: "Etykiety", placeholder: "etykieta" },
  { key: "milestones", label: "Kamienie milowe", placeholder: "kamień milowy" },
] as const;

const RULES = [
  { key: "include", label: "uwzględnij" },
  { key: "exclude", label: "pomiń" },
] as const;

type Props = {
  form: GitlabForm;
  onChange: (form: GitlabForm) => void;
  errors: SectionErrors;
};

export function GitlabSection({ form, onChange, errors }: Props) {
  const setFilter = (change: Partial<FilterForm>) =>
    onChange({ ...form, filter: { ...form.filter, ...change } });
  const setRule = (key: (typeof DIMENSIONS)[number]["key"], change: Partial<DimensionForm>) =>
    setFilter({ [key]: { ...form.filter[key], ...change } });

  return (
    <>
      <TextSetting
        label="Host GitLaba"
        value={form.host}
        onChange={(host) => onChange({ ...form, host })}
        error={errors.field("gitlab.host")}
        placeholder="puste = nie skonfigurowano"
      />
      <TextSetting
        label="Login (opcjonalnie)"
        value={form.user}
        onChange={(user) => onChange({ ...form, user })}
        error={errors.field("gitlab.user")}
      />
      {form.host === "" && form.user !== "" && (
        <p className="meta dn-settings-hint">Bez hosta login nie zostanie zapisany.</p>
      )}
      <TextSetting
        label="Odświeżanie co (minuty)"
        value={form.refreshMinutes}
        onChange={(refreshMinutes) => onChange({ ...form, refreshMinutes })}
        error={errors.field("gitlabRefresh.minutes")}
        placeholder="10"
        inputMode="numeric"
      />
      <h3 className="panel-title">Filtr listy</h3>
      {DIMENSIONS.map((dimension) =>
        RULES.map((rule) => {
          const values = form.filter[dimension.key][rule.key];
          return (
            <ListSetting
              key={`${dimension.key}.${rule.key}`}
              label={`${dimension.label} — ${rule.label}`}
              values={values}
              onChange={(next) => setRule(dimension.key, { [rule.key]: next })}
              errors={errors.list(`gitlabFilter.${dimension.key}.${rule.key}`, values.length)}
              max={FILTER_MAX}
              placeholder={dimension.placeholder}
            />
          );
        }),
      )}
      <TextSetting
        label="Zamknięte z ostatnich N dni"
        value={form.filter.closedDays}
        onChange={(closedDays) => setFilter({ closedDays })}
        error={errors.field("gitlabFilter.closedDays")}
        placeholder="puste = wyłączone"
        inputMode="numeric"
      />
    </>
  );
}
