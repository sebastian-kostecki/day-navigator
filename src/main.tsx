import React from "react";
import ReactDOM from "react-dom/client";
import "./styles/tokens.css";
import "./styles/layout.css";
import App from "./App";
import { SettingsDialogProvider } from "./settings/SettingsDialogProvider";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <SettingsDialogProvider>
      <App />
    </SettingsDialogProvider>
  </React.StrictMode>,
);
