import { useState } from "react";
import { GitlabView } from "./gitlab/GitlabView";
import { HealthPanel } from "./health/HealthPanel";
import OwnTasksView from "./own-tasks/OwnTasksView";
import TodoView from "./todo/TodoView";
import WorkTimeView from "./worktime/WorkTimeView";
import { WorkTimerHeader } from "./worktime/WorkTimerHeader";
import "./App.css";

type View = "todo" | "gitlab" | "tasks" | "work-time" | "health";

const VIEWS: { id: View; label: string }[] = [
  { id: "todo", label: "Do zrobienia" },
  { id: "gitlab", label: "Praca (GitLab)" },
  { id: "tasks", label: "Moje zadania" },
  { id: "work-time", label: "Czas pracy" },
  { id: "health", label: "Stan źródeł" },
];

function App() {
  const [view, setView] = useState<View>("todo");

  return (
    <>
      <header className="app-header">
        <nav className="app-nav" aria-label="Widoki">
          {VIEWS.map((v) => (
            <button
              key={v.id}
              type="button"
              className="nav-button"
              aria-current={view === v.id ? "page" : undefined}
              onClick={() => setView(v.id)}
            >
              {v.label}
            </button>
          ))}
        </nav>
        <WorkTimerHeader />
      </header>
      {view === "todo" && <TodoView />}
      {view === "gitlab" && <GitlabView />}
      {view === "tasks" && <OwnTasksView />}
      {view === "work-time" && <WorkTimeView />}
      {view === "health" && <HealthPanel />}
    </>
  );
}

export default App;
