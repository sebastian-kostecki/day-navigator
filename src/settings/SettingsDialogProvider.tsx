import { useCallback, useMemo, useState, type ReactNode } from "react";
import type { SectionId } from "./types";
import { SettingsDialogContext } from "./useSettingsDialog";

/** Holds whether the settings dialog is open, and at which section. */
export function SettingsDialogProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState<SectionId | null>(null);
  const openAt = useCallback((section: SectionId = "gitlab") => setOpen(section), []);
  const close = useCallback(() => setOpen(null), []);
  const value = useMemo(() => ({ open, openAt, close }), [open, openAt, close]);
  return <SettingsDialogContext value={value}>{children}</SettingsDialogContext>;
}
