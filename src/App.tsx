import { HealthPanel } from "./health/HealthPanel";
import TasksView from "./tasks/TasksView";
import "./App.css";

// No navigation yet (S-01): the views are stacked.
function App() {
  return (
    <>
      <TasksView />
      <HealthPanel />
    </>
  );
}

export default App;
