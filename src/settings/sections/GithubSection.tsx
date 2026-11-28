import type { SectionErrors } from "../fieldErrors";
import type { GithubForm } from "../types";
import { ListSetting, TextSetting } from "./controls";

const REPOS_MAX = 50;

type Props = {
  form: GithubForm;
  onChange: (form: GithubForm) => void;
  errors: SectionErrors;
};

export function GithubSection({ form, onChange, errors }: Props) {
  return (
    <>
      <ListSetting
        label="Repozytoria"
        values={form.repos}
        onChange={(repos) => onChange({ ...form, repos })}
        errors={errors.list("github.repos", form.repos.length)}
        max={REPOS_MAX}
        placeholder="właściciel/nazwa"
      />
      <p className="meta dn-settings-hint">Pusta lista = GitHub nie jest skonfigurowany.</p>
      <TextSetting
        label="Odświeżanie co (minuty)"
        value={form.refreshMinutes}
        onChange={(refreshMinutes) => onChange({ ...form, refreshMinutes })}
        error={errors.field("githubRefresh.minutes")}
        placeholder="10"
        inputMode="numeric"
      />
    </>
  );
}
