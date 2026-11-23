/** Mirrors `store::StoreError`. */
export type StoreError =
  | { kind: "DirUnavailable"; detail: { kind: string } }
  | { kind: "Damaged"; detail: { reason: string } }
  | { kind: "Missing" }
  | { kind: "NewerSchema"; detail: { found: number; supported: number } }
  | { kind: "MigrationFailed"; detail: { version: number; reason: string } }
  | { kind: "Busy" }
  | { kind: "Io"; detail: { kind: string } }
  | { kind: "Sql"; detail: { reason: string } };

/** Polish message for a store failure, shared by every feature on the app data store. */
export function storeErrorMessage(e: StoreError): string {
  switch (e.kind) {
    case "DirUnavailable":
      return "Dane aplikacji: folder danych jest niedostępny.";
    case "Damaged":
      return "Dane aplikacji: plik danych jest uszkodzony.";
    case "Missing":
      return "Dane aplikacji: brakuje pliku danych, chociaż istnieją kopie zapasowe.";
    case "NewerSchema":
      return "Dane aplikacji pochodzą z nowszej wersji aplikacji.";
    case "MigrationFailed":
      return "Dane aplikacji: nie udało się zaktualizować struktury danych.";
    case "Busy":
      return "Dane aplikacji są chwilowo zajęte — spróbuj ponownie.";
    case "Io":
      return "Dane aplikacji: błąd odczytu lub zapisu.";
    case "Sql":
      return "Dane aplikacji: błąd bazy danych.";
    default:
      return "Dane aplikacji: nieoczekiwany błąd.";
  }
}
