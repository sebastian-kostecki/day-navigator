import { createContext, use } from "react";
import type { SectionId } from "./types";

export type SettingsDialogState = {
  /** The section shown, or null while the dialog is closed. */
  open: SectionId | null;
  /** Opens the dialog at a section; the first one when none is given. */
  openAt: (section?: SectionId) => void;
  close: () => void;
};

export const SettingsDialogContext = createContext<SettingsDialogState | null>(null);

export function useSettingsDialog(): SettingsDialogState {
  const value = use(SettingsDialogContext);
  if (!value) throw new Error("useSettingsDialog() needs a SettingsDialogProvider");
  return value;
}
