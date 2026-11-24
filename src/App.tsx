import { useEffect, useState } from "react";
import { AppHeader } from "./components";
import { GitlabView } from "./gitlab/GitlabView";
import { useGitlabSeenMark } from "./gitlab/useGitlabSeenMark";
import { GithubView } from "./github/GithubView";
import { HealthPanel } from "./health/HealthPanel";
import OwnTasksView from "./own-tasks/OwnTasksView";
import TodoView from "./todo/TodoView";
import WorkTimeView from "./worktime/WorkTimeView";
import { WorkTimerHeader } from "./worktime/WorkTimerHeader";

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

  // Digits 1–6 switch views; never with a modifier, on auto-repeat or while typing.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
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
  }, []);

  return (
    <>
      <AppHeader
        items={VIEWS.map((v, i) => ({
          id: v.id,
          label: v.label,
          key: String(i + 1),
          active: view === v.id,
        }))}
        onSelect={(id) => setView(id as View)}
      >
        <WorkTimerHeader />
      </AppHeader>
      {view === "todo" && <TodoView />}
      {view === "gitlab" && <GitlabView />}
      {view === "github" && <GithubView />}
      {view === "tasks" && <OwnTasksView />}
      {view === "work-time" && <WorkTimeView />}
      {view === "health" && <HealthPanel />}
    </>
  );
}

export default App;
