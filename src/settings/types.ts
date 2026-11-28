// Mirrors the serde output of src-tauri/src/settings/commands.rs and
// src-tauri/src/settings/sections.rs.

import type { SourceStatus } from "../health/types";

export type SectionId =
  "gitlab" | "github" | "todo" | "workTime" | "reminders" | "notices" | "autostart";

export type DimensionForm = {
  include: string[];
  exclude: string[];
};

export type FilterForm = {
  projects: DimensionForm;
  labels: DimensionForm;
  milestones: DimensionForm;
  /** Empty is off. */
  closedDays: string;
};

export type GitlabForm = {
  /** Empty is "not configured". */
  host: string;
  user: string;
  refreshMinutes: string;
  filter: FilterForm;
};

export type GithubForm = {
  /** Empty is "not configured". */
  repos: string[];
  refreshMinutes: string;
};

export type TodoForm = { limit: string };

export type WorkTimeForm = {
  holidays: boolean;
  /** Hours; `,` or `.` as the decimal mark. */
  dailyNormHours: string;
};

export type RemindersForm = {
  longRunningOn: boolean;
  longRunningHours: string;
  notRunningAfterOn: boolean;
  /** `HH:MM`. */
  notRunningAfter: string;
};

export type NoticesForm = {
  newAssignments: boolean;
  sourceProblems: boolean;
  daySummary: boolean;
};

export type AutostartForm = { enabled: boolean };

/** Payload of the `settings-changed` event. */
export type SettingsChanged = { section: SectionId };

/** The form of each section. */
export type SectionForms = {
  gitlab: GitlabForm;
  github: GithubForm;
  todo: TodoForm;
  workTime: WorkTimeForm;
  reminders: RemindersForm;
  notices: NoticesForm;
  autostart: AutostartForm;
};

export type SectionView<F> = {
  /** Current values; the defaults where a block is absent or invalid. */
  form: F;
  /** Keys of the blocks that are invalid on disk. */
  invalidBlocks: string[];
  /** Opaque, per block key; sent back with a save. */
  fingerprints: Record<string, string>;
};

export type FileState = "present" | "missing";

export type SettingsView = {
  file: FileState;
  sections: { [K in SectionId]: SectionView<SectionForms[K]> };
};

/** `section` is the tag, `form` the content. */
export type SectionInput = {
  [K in SectionId]: { section: K; form: SectionForms[K] };
}[SectionId];

export type SaveRequest = {
  section: SectionInput;
  fingerprints: Record<string, string>;
  replaceInvalid: boolean;
};

export type SaveResult = {
  view: SettingsView;
  /** Set for an `autostart` save only. */
  autostart: SourceStatus | null;
};

/** One bad field: `field` is a JSON key path, `index` the list position. */
export type FieldError = {
  field: string;
  index: number | null;
  message: string;
};

export type SettingsViewError =
  { kind: "ConfigDirUnresolved" } | { kind: "FileInvalid" } | { kind: "Io" };

export type SaveSettingsError =
  | { kind: "ConfigDirUnresolved" }
  | { kind: "ConfigDirMissing" }
  | { kind: "FileInvalid" }
  | { kind: "Conflict"; detail: { blocks: string[] } }
  | { kind: "InvalidOnDisk"; detail: { blocks: string[] } }
  | { kind: "Invalid"; detail: { fields: FieldError[] } }
  | { kind: "Io" };
