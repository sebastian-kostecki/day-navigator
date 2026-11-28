import { invoke } from "@tauri-apps/api/core";
import type { SaveRequest, SaveResult, SaveSettingsError, SettingsView } from "./types";

/** A failure rejects with `SettingsViewError`. */
export function settingsView(): Promise<SettingsView> {
  return invoke<SettingsView>("settings_view");
}

/** Saves one section. A failure rejects with `SaveSettingsError`. */
export function saveSettingsSection(req: SaveRequest): Promise<SaveResult> {
  return invoke<SaveResult>("save_settings_section", { request: req });
}

/** Polish message for anything thrown by `settingsView` or `saveSettingsSection`. */
export function settingsErrorMessage(e: unknown): string {
  if (typeof e === "object" && e !== null && "kind" in e) {
    // `SettingsViewError` kinds are a subset of the save ones.
    const err = e as SaveSettingsError;
    switch (err.kind) {
      case "ConfigDirUnresolved":
        return "Nie można ustalić folderu ustawień aplikacji.";
      case "ConfigDirMissing":
        return "Brak folderu ustawień aplikacji. Niczego nie zapisano.";
      case "FileInvalid":
        return "Plik settings.json nie jest poprawnym JSON-em z „version”: 1. Popraw go ręcznie.";
      case "Conflict":
        return "Zmieniono w pliku — wczytaj ponownie.";
      case "InvalidOnDisk":
        return "Blok w settings.json jest nieprawidłowy.";
      case "Invalid":
        return "Popraw zaznaczone pola.";
      case "Io":
        return "Nie udało się odczytać lub zapisać settings.json.";
      default:
        return `Nieoczekiwany błąd (${(err as { kind: string }).kind})`;
    }
  }
  return `Nieoczekiwany błąd: ${String(e)}`;
}
