import { useCallback, useEffect, useRef, useState } from "react";
import type { SourceStatus } from "../health/types";
import { saveSettingsSection, settingsErrorMessage, settingsView } from "./api";
import { fieldValues } from "./fieldErrors";
import type {
  FieldError,
  FileState,
  SaveSettingsError,
  SectionForms,
  SectionId,
  SectionInput,
  SectionView,
  SettingsView,
} from "./types";

/** The last save or reload of a section that did not work. */
export type Failure = {
  message: string;
  /** "Wczytaj ponownie" helps: the file changed, or a reload itself failed. */
  reload: boolean;
};

export type SectionState<F> = {
  /** The draft shown in the inputs. */
  form: F;
  /** The form as loaded; the section is dirty while the draft differs. */
  base: F;
  invalidBlocks: string[];
  /** Of the file `base` was read from, so a later change there still conflicts. */
  fingerprints: Record<string, string>;
  /** Being saved or reloaded. */
  pending: boolean;
  fieldErrors: FieldError[];
  failure: Failure | null;
  saved: boolean;
  /** Asking whether the invalid block on disk may be replaced. */
  confirmReplace: boolean;
  /** The sync outcome of the last save; `autostart` only. */
  status: SourceStatus | null;
};

export type Sections = { [K in SectionId]: SectionState<SectionForms[K]> };

export type SettingsData =
  | { state: "loading" }
  | { state: "error"; message: string; lastOkAt: number | null }
  | { state: "data"; file: FileState; sections: Sections };

/** In the order of the tabs. */
export const SECTION_IDS: SectionId[] = [
  "gitlab",
  "github",
  "todo",
  "workTime",
  "reminders",
  "notices",
  "autostart",
];

/** Time of the last successful load, kept while the app runs. */
let lastLoadedAt: number | null = null;

export function isDirty(section: SectionState<unknown>): boolean {
  return JSON.stringify(section.form) !== JSON.stringify(section.base);
}

function fresh<F>(view: SectionView<F>): SectionState<F> {
  return {
    form: view.form,
    base: view.form,
    invalidBlocks: view.invalidBlocks,
    fingerprints: view.fingerprints,
    pending: false,
    fieldErrors: [],
    failure: null,
    saved: false,
    confirmReplace: false,
    status: null,
  };
}

function freshSection<K extends SectionId>(view: SettingsView, id: K): Sections[K] {
  // Read through the mapped type, so the form type follows `id`.
  const sections: { [S in SectionId]: SectionView<SectionForms[S]> } = view.sections;
  const section: SectionView<SectionForms[K]> = sections[id];
  return fresh(section) as Sections[K];
}

function patch<K extends SectionId>(
  sections: Sections,
  id: K,
  change: Partial<SectionState<SectionForms[K]>>,
): Sections {
  return { ...sections, [id]: { ...sections[id], ...change } };
}

function allFresh(view: SettingsView): Sections {
  const s = view.sections;
  return {
    gitlab: fresh(s.gitlab),
    github: fresh(s.github),
    todo: fresh(s.todo),
    workTime: fresh(s.workTime),
    reminders: fresh(s.reminders),
    notices: fresh(s.notices),
    autostart: fresh(s.autostart),
  };
}

/**
 * The sections after a save: the saved one and every clean one take the new
 * view; an unsaved draft elsewhere stays, with the fingerprints it was made on.
 */
function afterSave(
  previous: Sections,
  view: SettingsView,
  savedId: SectionId,
  status: SourceStatus | null,
): Sections {
  let next = allFresh(view);
  for (const id of SECTION_IDS) {
    const old = previous[id];
    if (id === savedId) next = patch(next, id, { saved: true, status });
    else if (isDirty(old)) next = { ...next, [id]: old };
    else next = patch(next, id, { saved: old.saved, status: old.status });
  }
  return next;
}

function asSaveError(e: unknown): SaveSettingsError | null {
  return typeof e === "object" && e !== null && "kind" in e ? (e as SaveSettingsError) : null;
}

/**
 * The state of one opening of the dialog: the view is loaded on mount, and each
 * section keeps its own draft until the dialog closes.
 */
export function useSettings() {
  const [data, setData] = useState<SettingsData>({ state: "loading" });
  // One save or reload at a time, so an older answer never lands on a newer one.
  const busy = useRef(false);
  // Latest load wins.
  const issued = useRef(0);

  const load = useCallback(async () => {
    const request = ++issued.current;
    let value: SettingsData;
    try {
      const view = await settingsView();
      value = { state: "data", file: view.file, sections: allFresh(view) };
    } catch (e) {
      value = { state: "error", message: settingsErrorMessage(e), lastOkAt: lastLoadedAt };
    }
    if (request !== issued.current) return;
    if (value.state === "data") lastLoadedAt = Date.now();
    setData(value);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const retry = useCallback(() => {
    setData({ state: "loading" });
    void load();
  }, [load]);

  const update = useCallback((change: (sections: Sections) => Sections, file?: FileState) => {
    setData((d) =>
      d.state === "data" ? { ...d, file: file ?? d.file, sections: change(d.sections) } : d,
    );
  }, []);

  const edit = useCallback(
    <K extends SectionId>(id: K, form: SectionForms[K]) => {
      update((sections) => {
        const section = sections[id];
        const before = fieldValues(id, section.form);
        const after = fieldValues(id, form);
        // An edited field drops its errors: list positions may have moved.
        const fieldErrors = section.fieldErrors.filter(
          (e) => JSON.stringify(before[e.field]) === JSON.stringify(after[e.field]),
        );
        return patch(sections, id, { form, fieldErrors, saved: false, confirmReplace: false });
      });
    },
    [update],
  );

  const cancelReplace = useCallback(
    (id: SectionId) => update((s) => patch(s, id, { confirmReplace: false })),
    [update],
  );

  async function save(id: SectionId, replaceInvalid: boolean) {
    if (data.state !== "data" || busy.current) return;
    const section = data.sections[id];
    if (!replaceInvalid && section.invalidBlocks.length > 0) {
      update((s) => patch(s, id, { confirmReplace: true, saved: false }));
      return;
    }
    busy.current = true;
    update((s) =>
      patch(s, id, {
        pending: true,
        fieldErrors: [],
        failure: null,
        saved: false,
        confirmReplace: false,
        status: null,
      }),
    );
    try {
      const result = await saveSettingsSection({
        section: { section: id, form: section.form } as SectionInput,
        fingerprints: section.fingerprints,
        replaceInvalid,
      });
      update((s) => afterSave(s, result.view, id, result.autostart), result.view.file);
    } catch (e) {
      const error = asSaveError(e);
      update((s) => {
        if (error?.kind === "InvalidOnDisk") {
          return patch(s, id, {
            pending: false,
            invalidBlocks: error.detail.blocks,
            confirmReplace: true,
          });
        }
        return patch(s, id, {
          pending: false,
          fieldErrors: error?.kind === "Invalid" ? error.detail.fields : [],
          failure: { message: settingsErrorMessage(e), reload: error?.kind === "Conflict" },
        });
      });
    } finally {
      busy.current = false;
    }
  }

  /** Drops one section's draft and reads that section from the file again. */
  async function reload(id: SectionId) {
    if (data.state !== "data" || busy.current) return;
    busy.current = true;
    update((s) => patch(s, id, { pending: true, saved: false, confirmReplace: false }));
    try {
      const view = await settingsView();
      update((s) => ({ ...s, [id]: freshSection(view, id) }), view.file);
    } catch (e) {
      update((s) =>
        patch(s, id, {
          pending: false,
          failure: { message: settingsErrorMessage(e), reload: true },
        }),
      );
    } finally {
      busy.current = false;
    }
  }

  return { data, retry, edit, save, reload, cancelReplace };
}
