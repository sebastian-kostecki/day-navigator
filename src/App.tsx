import { useEffect, useState } from "react";
import { AppHeader, Button } from "./components";
import { DaySummaryPanel } from "./day-summary/DaySummaryPanel";
import { GitlabListViewProvider } from "./gitlab/GitlabListViewProvider";
import { GitlabView } from "./gitlab/GitlabView";
import { useGitlabSeenMark } from "./gitlab/useGitlabSeenMark";
import { GithubView } from "./github/GithubView";
import { HealthPanel } from "./health/HealthPanel";
import OwnTasksView from "./own-tasks/OwnTasksView";
import { SettingsDialog } from "./settings/SettingsDialog";
import { useSettingsDialog } from "./settings/useSettingsDialog";
import TodoView from "./todo/TodoView";
import WorkTimeView from "./worktime/WorkTimeView";
import { WorkTimerHeader } from "./worktime/WorkTimerHeader";
import { WorkTimerProvider } from "./worktime/WorkTimerProvider";

type View = "todo" | "gitlab" | "github" | "tasks" | "work-time" | "health";

/** Order matters: the n-th view is shown with key n. */
const VIEWS: { id: View; label: string }[] = [
  { id: "todo", label: "Do zrobienia" },
  { id: "gitlab", label: "GitLab" },
  { id: "github", label: "GitHub" },
  { id: "tasks", label: "Moje zadania" },
  { id: "work-time", label: "Czas pracy" },
  { id: "health", label: "Źródła" },
];

/** Typing into a field must not switch views. */
function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

function App() {
  const [view, setView] = useState<View>("todo");
  useGitlabSeenMark(view);
  const { open: settingsOpen, openAt: openSettings } = useSettingsDialog();
  // Bumped by each header STOP; 0 means no summary.
  const [summaryRun, setSummaryRun] = useState(0);

  // Digits 1–6 switch views; never with a modifier, on auto-repeat, while typing
  // or under the settings dialog.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (settingsOpen !== null) return;
      if (event.ctrlKey || event.altKey || event.metaKey || event.repeat) return;
      if (event.defaultPrevented || isEditable(event.target)) return;
      if (!/^[1-9]$/.test(event.key)) return;
      const next = VIEWS[Number(event.key) - 1];
      if (!next) return;
      event.preventDefault();
      setView(next.id);
      // Keep keyboard focus on the tab just chosen, not on the previous one
      // (or on <body> when the focused element belonged to the old view).
      requestAnimationFrame(() => {
        document
          .querySelector<HTMLButtonElement>(`.dn-nav [data-view="${next.id}"]`)
          ?.focus({ preventScroll: true });
      });
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [settingsOpen]);

  return (
    <WorkTimerProvider>
      <AppHeader
        items={VIEWS.map((v, i) => ({
          id: v.id,
          label: v.label,
          key: String(i + 1),
          active: view === v.id,
        }))}
        onSelect={(id) => setView(id as View)}
      >
        <WorkTimerHeader onStopped={() => setSummaryRun((n) => n + 1)} />
        <Button iconOnly icon="settings" aria-label="Ustawienia" onClick={() => openSettings()} />
      </AppHeader>
      {summaryRun > 0 && <DaySummaryPanel key={summaryRun} onClose={() => setSummaryRun(0)} />}
      <GitlabListViewProvider>
        {view === "todo" && <TodoView />}
        {view === "gitlab" && <GitlabView />}
        {view === "github" && <GithubView />}
        {view === "tasks" && <OwnTasksView />}
        {view === "work-time" && <WorkTimeView />}
        {view === "health" && <HealthPanel />}
      </GitlabListViewProvider>
      <SettingsDialog />
    </WorkTimerProvider>
  );
}

export default App;
