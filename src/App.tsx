import { HealthPanel } from "./health/HealthPanel";
import OwnTasksView from "./own-tasks/OwnTasksView";
import "./App.css";

// No navigation yet (S-01): the views are stacked.
function App() {
  return (
    <>
      <OwnTasksView />
      <HealthPanel />
    </>
  );
}

export default App;
