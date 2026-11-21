import { invoke } from "@tauri-apps/api/core";

export type VaultError = { kind: string; detail?: unknown };

export type TaskError =
  | { kind: "Vault"; detail: VaultError }
  | { kind: "TasksNotConfigured" }
  | { kind: "FolderMissing"; detail: { path: string } }
  | { kind: "TemplateMissing"; detail: { path: string } }
  | { kind: "TemplateUnsupported"; detail: { placeholder: string } }
  | { kind: "TemplateFieldMissing"; detail: { key: string } }
  | { kind: "InvalidTitle"; detail: { reason: string } }
  | { kind: "UnknownStatus"; detail: { status: string } }
  | { kind: "UnknownPriority"; detail: { priority: string } }
  | { kind: "InvalidDue"; detail: { value: string } };

export interface TaskRow {
  path: string;
  title: string;
  status: string | null;
  priority: string | null;
  due: string | null;
  statusHash: string | null;
  doneHash: string | null;
  problem: VaultError | null;
}

export interface NewTask {
  title: string;
  priority: string | null;
  due: string | null;
}

export type DoneUpdate = "Set" | "Cleared" | "Unchanged" | "Absent";

export interface TaskSettingsView {
  statuses: string[];
  doneStatus: string;
  priorities: string[];
  folder: string;
}

export function getWorkTaskSettings(): Promise<TaskSettingsView> {
  return invoke<TaskSettingsView>("get_work_task_settings");
}

export function listWorkTasks(): Promise<TaskRow[]> {
  return invoke<TaskRow[]>("list_work_tasks");
}

export function createWorkTask(newTask: NewTask): Promise<string> {
  return invoke<string>("create_work_task", { new: newTask });
}

export function setWorkTaskStatus(
  path: string,
  status: string,
  statusHash: string,
  doneHash: string | null,
): Promise<DoneUpdate> {
  return invoke<DoneUpdate>("set_work_task_status", {
    path,
    status,
    statusHash,
    doneHash,
  });
}

function asTaskError(e: unknown): TaskError | null {
  if (typeof e === "object" && e !== null && "kind" in e) {
    return e as TaskError;
  }
  return null;
}

export function isConflict(e: unknown): boolean {
  const err = asTaskError(e);
  return err?.kind === "Vault" && err.detail.kind === "Conflict";
}

function str(detail: unknown, key: string): string {
  if (typeof detail === "object" && detail !== null && key in detail) {
    const v = (detail as Record<string, unknown>)[key];
    if (typeof v === "string") return v;
  }
  return "";
}

export function vaultErrorMessage(e: VaultError): string {
  const d = e.detail;
  switch (e.kind) {
    case "NotConfigured":
      return "Brak pliku ustawień";
    case "SettingsInvalid":
      return `Nieprawidłowe ustawienia: ${str(d, "reason")}`;
    case "SpaceNotConfigured":
      return "Przestrzeń robocza nie jest skonfigurowana w settings.json";
    case "RootMissing":
      return "Folder vaulta nie istnieje";
    case "RootsOverlap":
      return "Foldery vaultów pracy i prywatnego nakładają się";
    case "InvalidPath":
      return `Nieprawidłowa ścieżka: ${str(d, "path")}`;
    case "OutsideVault":
      return `Ścieżka poza vaultem: ${str(d, "path")}`;
    case "Excluded":
      return `Ścieżka jest wykluczona: ${str(d, "path")}`;
    case "Hidden":
      return `Ścieżka jest ukryta: ${str(d, "path")}`;
    case "NotMarkdown":
      return `To nie jest notatka Markdown: ${str(d, "path")}`;
    case "NotFound":
      return `Nie znaleziono: ${str(d, "path")}`;
    case "AlreadyExists":
      return `Już istnieje: ${str(d, "path")}`;
    case "NotUtf8":
      return `Notatka nie jest w UTF-8: ${str(d, "path")}`;
    case "NoFrontmatter":
      return "Notatka nie ma frontmattera";
    case "FieldNotFound":
      return `Notatka nie ma pola ${str(d, "key")}`;
    case "FieldAmbiguous":
      return `Pole ${str(d, "key")} występuje wielokrotnie`;
    case "FieldLayoutUnsupported":
      return `Pole ${str(d, "key")} ma nieobsługiwany układ — popraw je w Obsidianie`;
    case "SectionNotFound":
      return `Nie znaleziono sekcji ${str(d, "heading")}`;
    case "SectionAmbiguous":
      return `Sekcja ${str(d, "heading")} występuje wielokrotnie`;
    case "InvalidValue":
      return `Nieprawidłowa wartość: ${str(d, "reason")}`;
    case "Conflict":
      return "Notatka zmieniła się w Obsidianie — odświeżono listę";
    case "Busy":
      return `Plik jest zajęty: ${str(d, "path")}`;
    case "ReadOnly":
      return `Plik jest tylko do odczytu: ${str(d, "path")}`;
    case "ReplaceIncomplete":
      return `Zapis nie został dokończony: ${str(d, "path")}`;
    case "Io":
      return `Błąd odczytu lub zapisu: ${str(d, "path")}`;
    default:
      return `Nieoczekiwany błąd (${e.kind})`;
  }
}

export function taskErrorMessage(e: TaskError): string {
  switch (e.kind) {
    case "Vault":
      return vaultErrorMessage(e.detail);
    case "TasksNotConfigured":
      return "Zadania nie są skonfigurowane w settings.json";
    case "FolderMissing":
      return `Folder zadań nie istnieje: ${e.detail.path}`;
    case "TemplateMissing":
      return `Brak szablonu zadania: ${e.detail.path}`;
    case "TemplateUnsupported":
      return `Szablon używa nieobsługiwanego znacznika ${e.detail.placeholder}`;
    case "TemplateFieldMissing":
      return `Szablon nie ma pola ${e.detail.key}`;
    case "InvalidTitle":
      return `Nieprawidłowy tytuł: ${e.detail.reason}`;
    case "UnknownStatus":
      return `Nieznany status: ${e.detail.status}`;
    case "UnknownPriority":
      return `Nieznany priorytet: ${e.detail.priority}`;
    case "InvalidDue":
      return `Nieprawidłowy termin: ${e.detail.value}`;
    default:
      return `Nieoczekiwany błąd (${(e as { kind: string }).kind})`;
  }
}

/** Message for anything thrown by an invoke call. */
export function anyErrorMessage(e: unknown): string {
  const err = asTaskError(e);
  if (err) return taskErrorMessage(err);
  return `Nieoczekiwany błąd: ${String(e)}`;
}
