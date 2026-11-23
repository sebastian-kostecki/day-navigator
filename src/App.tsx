import { useState } from "react";
import { HealthPanel } from "./health/HealthPanel";
import TasksView from "./tasks/TasksView";
import WorkTimeView from "./worktime/WorkTimeView";
import { WorkTimerHeader } from "./worktime/WorkTimerHeader";
import "./App.css";

type View = "tasks" | "work-time" | "health";

const VIEWS: { id: View; label: string }[] = [
  { id: "tasks", label: "Zadania" },
  { id: "work-time", label: "Czas pracy" },
  { id: "health", label: "Stan źródeł" },
];

function App() {
  const [view, setView] = useState<View>("tasks");

  return (
    <>
      <header className="app-header">
        <WorkTimerHeader />
        <nav className="view-switch" aria-label="Widoki">
          {VIEWS.map((v) => (
            <button
              key={v.id}
              type="button"
              aria-current={view === v.id ? "page" : undefined}
              onClick={() => setView(v.id)}
            >
              {v.label}
            </button>
          ))}
        </nav>
      </header>
      {view === "tasks" && <TasksView />}
      {view === "work-time" && <WorkTimeView />}
      {view === "health" && <HealthPanel />}
    </>
  );
}

export default App;
