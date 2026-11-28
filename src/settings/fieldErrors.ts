// Maps the field-error keys of src-tauri to the inputs of each section.

import type { FieldError, SectionForms, SectionId } from "./types";

type FieldValues = { [K in SectionId]: (form: SectionForms[K]) => Record<string, unknown> };

/** Per section: every field-error key that has an input, with that input's value. */
const FIELD_VALUES: FieldValues = {
  gitlab: ({ host, user, refreshMinutes, filter }) => ({
    "gitlab.host": host,
    "gitlab.user": user,
    "gitlabRefresh.minutes": refreshMinutes,
    "gitlabFilter.closedDays": filter.closedDays,
    "gitlabFilter.projects.include": filter.projects.include,
    "gitlabFilter.projects.exclude": filter.projects.exclude,
    "gitlabFilter.labels.include": filter.labels.include,
    "gitlabFilter.labels.exclude": filter.labels.exclude,
    "gitlabFilter.milestones.include": filter.milestones.include,
    "gitlabFilter.milestones.exclude": filter.milestones.exclude,
  }),
  github: (form) => ({
    "github.repos": form.repos,
    "githubRefresh.minutes": form.refreshMinutes,
  }),
  todo: (form) => ({ "todo.limit": form.limit }),
  workTime: (form) => ({
    "workTime.holidays": form.holidays,
    "workTime.dailyNormHours": form.dailyNormHours,
  }),
  reminders: (form) => ({
    "reminders.longRunningHours": [form.longRunningOn, form.longRunningHours],
    "reminders.notRunningAfter": [form.notRunningAfterOn, form.notRunningAfter],
  }),
  notices: (form) => ({
    "notices.newAssignments": form.newAssignments,
    "notices.sourceProblems": form.sourceProblems,
    "notices.daySummary": form.daySummary,
  }),
  autostart: (form) => ({ "autostart.enabled": form.enabled }),
};

export function fieldValues<K extends SectionId>(
  id: K,
  form: SectionForms[K],
): Record<string, unknown> {
  return FIELD_VALUES[id](form);
}

/** The errors no input of the section shows; they go to the top of it. */
export function unmappedErrors<K extends SectionId>(
  id: K,
  form: SectionForms[K],
  errors: FieldError[],
): FieldError[] {
  const known = fieldValues(id, form);
  return errors.filter((e) => !(e.field in known));
}

export type ListErrors = {
  /** By position in the list. */
  items: Record<number, string>;
  /** About the list as a whole. */
  general: string | undefined;
};

export type SectionErrors = {
  /** The message of a single-value field. */
  field: (key: string) => string | undefined;
  list: (key: string, length: number) => ListErrors;
};

export function sectionErrors(errors: FieldError[]): SectionErrors {
  const of = (key: string) => errors.filter((e) => e.field === key);
  const join = (found: FieldError[]) =>
    found.length === 0 ? undefined : found.map((e) => e.message).join(" ");
  return {
    field: (key) => join(of(key)),
    list: (key, length) => {
      const items: Record<number, string> = {};
      const general: FieldError[] = [];
      for (const e of of(key)) {
        if (e.index === null || e.index >= length) general.push(e);
        else items[e.index] = items[e.index] ? `${items[e.index]} ${e.message}` : e.message;
      }
      return { items, general: join(general) };
    },
  };
}
