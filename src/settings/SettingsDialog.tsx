import { useEffect, useState, type ReactNode } from "react";
import { Button, Dialog, ErrorState, Notice, Tabs, formatClock } from "../components";
import { openSettingsErrorMessage, openSettingsFile } from "../gitlab/api";
import { sectionErrors } from "./fieldErrors";
import { AutostartSection } from "./sections/AutostartSection";
import { GithubSection } from "./sections/GithubSection";
import { GitlabSection } from "./sections/GitlabSection";
import { NoticesSection } from "./sections/NoticesSection";
import { RemindersSection } from "./sections/RemindersSection";
import { SectionShell } from "./sections/SectionShell";
import { TodoSection } from "./sections/TodoSection";
import { WorkTimeSection } from "./sections/WorkTimeSection";
import type { SectionId } from "./types";
import { SECTION_IDS, isDirty, useSettings } from "./useSettings";
import { useSettingsDialog } from "./useSettingsDialog";
import "./SettingsDialog.css";

const LABELS: Record<SectionId, string> = {
  gitlab: "GitLab",
  github: "GitHub",
  todo: "Do zrobienia",
  workTime: "Czas pracy",
  reminders: "Przypomnienia",
  notices: "Powiadomienia",
  autostart: "Autostart",
};

const PANEL_ID = "settings-panel";
const TAB_PREFIX = "settings-tab";

/** Mounted per opening, so drafts and late answers go away with the dialog. */
function SettingsScreen({ active }: { active: SectionId }) {
  const { openAt, close } = useSettingsDialog();
  const { data, retry, edit, save, reload, cancelReplace } = useSettings();
  const [closeRequested, setCloseRequested] = useState(false);
  const [openError, setOpenError] = useState<string | null>(null);

  const dirty = data.state === "data" ? SECTION_IDS.filter((id) => isDirty(data.sections[id])) : [];
  const confirmingClose = closeRequested && dirty.length > 0;

  useEffect(() => {
    if (closeRequested && dirty.length === 0) close();
  }, [closeRequested, dirty.length, close]);

  const requestClose = () => {
    // Text still in a list input is added on blur, so it counts as an edit.
    if (document.activeElement instanceof HTMLInputElement) document.activeElement.blur();
    setCloseRequested(true);
  };

  const openFile = () => {
    setOpenError(null);
    openSettingsFile().catch((e: unknown) => setOpenError(openSettingsErrorMessage(e)));
  };

  let body: ReactNode;
  if (data.state === "loading") {
    body = <p className="dn-loading">Ładowanie…</p>;
  } else if (data.state === "error") {
    body = (
      <ErrorState
        title="Nie udało się wczytać ustawień"
        message={data.message}
        lastSuccess={
          data.lastOkAt === null ? undefined : `ostatnio OK ${formatClock(data.lastOkAt)}`
        }
        onRetry={retry}
      />
    );
  } else {
    const { sections } = data;
    const busy = SECTION_IDS.some((id) => sections[id].pending);
    const shell = <K extends SectionId>(id: K) => ({
      id,
      section: sections[id],
      hidden: id !== active,
      busy,
      onSave: (replaceInvalid: boolean) => void save(id, replaceInvalid),
      onReload: () => void reload(id),
      onCancelReplace: () => cancelReplace(id),
    });
    const errors = (id: SectionId) => sectionErrors(sections[id].fieldErrors);

    body = (
      <div className="dn-settings">
        {data.file === "missing" && (
          <Notice tone="info" compact>
            Brak pliku — zostanie utworzony przy zapisie.
          </Notice>
        )}
        <Tabs
          label="Sekcje ustawień"
          tabs={SECTION_IDS.map((id) => ({
            value: id,
            label: dirty.includes(id) ? `${LABELS[id]} •` : LABELS[id],
          }))}
          value={active}
          onChange={openAt}
          panelId={PANEL_ID}
          idPrefix={TAB_PREFIX}
        />
        <div
          id={PANEL_ID}
          className="dn-settings-panel"
          role="tabpanel"
          aria-labelledby={`${TAB_PREFIX}-${active}`}
        >
          {/* All stay mounted: text not yet added to a list survives a tab switch. */}
          <SectionShell {...shell("gitlab")}>
            <GitlabSection
              form={sections.gitlab.form}
              onChange={(form) => edit("gitlab", form)}
              errors={errors("gitlab")}
            />
          </SectionShell>
          <SectionShell {...shell("github")}>
            <GithubSection
              form={sections.github.form}
              onChange={(form) => edit("github", form)}
              errors={errors("github")}
            />
          </SectionShell>
          <SectionShell {...shell("todo")}>
            <TodoSection
              form={sections.todo.form}
              onChange={(form) => edit("todo", form)}
              errors={errors("todo")}
            />
          </SectionShell>
          <SectionShell {...shell("workTime")}>
            <WorkTimeSection
              form={sections.workTime.form}
              onChange={(form) => edit("workTime", form)}
              errors={errors("workTime")}
            />
          </SectionShell>
          <SectionShell {...shell("reminders")}>
            <RemindersSection
              form={sections.reminders.form}
              onChange={(form) => edit("reminders", form)}
              errors={errors("reminders")}
            />
          </SectionShell>
          <SectionShell {...shell("notices")}>
            <NoticesSection
              form={sections.notices.form}
              onChange={(form) => edit("notices", form)}
              errors={errors("notices")}
            />
          </SectionShell>
          <SectionShell {...shell("autostart")}>
            <AutostartSection
              form={sections.autostart.form}
              onChange={(form) => edit("autostart", form)}
              errors={errors("autostart")}
              status={sections.autostart.status}
            />
          </SectionShell>
        </div>
      </div>
    );
  }

  const footer = confirmingClose ? (
    <>
      <Notice tone="warn" compact role="alert">
        Niezapisane zmiany w: {dirty.map((id) => LABELS[id]).join(", ")}
      </Notice>
      {/* Keyed, so "Wróć" mounts anew and takes the focus. */}
      <Button key="discard" onClick={close}>
        Zamknij bez zapisu
      </Button>
      <Button key="back" autoFocus onClick={() => setCloseRequested(false)}>
        Wróć
      </Button>
    </>
  ) : (
    <>
      {openError && (
        <Notice tone="error" compact>
          {openError}
        </Notice>
      )}
      <Button
        key="open"
        variant="quiet"
        icon="external"
        title="Otwórz settings.json w domyślnym edytorze"
        onClick={openFile}
      >
        Otwórz settings.json
      </Button>
      <Button key="close" onClick={requestClose}>
        Zamknij
      </Button>
    </>
  );

  return (
    <Dialog open onClose={requestClose} title="Ustawienia" footer={footer}>
      {body}
    </Dialog>
  );
}

export function SettingsDialog() {
  const { open } = useSettingsDialog();
  return open === null ? null : <SettingsScreen active={open} />;
}
