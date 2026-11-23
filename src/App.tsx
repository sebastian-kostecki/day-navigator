import { useState } from "react";
import { AppHeader } from "./components";
import { GitlabView } from "./gitlab/GitlabView";
import { GithubView } from "./github/GithubView";
import { HealthPanel } from "./health/HealthPanel";
import OwnTasksView from "./own-tasks/OwnTasksView";
import TodoView from "./todo/TodoView";
import WorkTimeView from "./worktime/WorkTimeView";
import { WorkTimerHeader } from "./worktime/WorkTimerHeader";
import "./App.css";

type View = "todo" | "gitlab" | "github" | "tasks" | "work-time" | "health";

const VIEWS: { id: View; label: string }[] = [
  { id: "todo", label: "Do zrobienia" },
  { id: "gitlab", label: "Praca (GitLab)" },
  { id: "github", label: "Projekty (GitHub)" },
  { id: "tasks", label: "Moje zadania" },
  { id: "work-time", label: "Czas pracy" },
  { id: "health", label: "Stan źródeł" },
];

function App() {
  const [view, setView] = useState<View>("todo");

  return (
    <>
      <AppHeader
        items={VIEWS.map((v) => ({ id: v.id, label: v.label, active: view === v.id }))}
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
