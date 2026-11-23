import { useState } from "react";
import { GitlabView } from "./gitlab/GitlabView";
import { HealthPanel } from "./health/HealthPanel";
import OwnTasksView from "./own-tasks/OwnTasksView";
import "./App.css";

type View = "gitlab" | "tasks" | "health";

const VIEWS: { id: View; label: string }[] = [
  { id: "gitlab", label: "Praca (GitLab)" },
  { id: "tasks", label: "Moje zadania" },
  { id: "health", label: "Stan źródeł" },
];

function App() {
  const [view, setView] = useState<View>("gitlab");

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
        {/* S-08: the always-visible work-time counter goes here. */}
      </header>
      {view === "gitlab" && <GitlabView />}
      {view === "tasks" && <OwnTasksView />}
      {view === "health" && <HealthPanel />}
    </>
  );
}

export default App;
