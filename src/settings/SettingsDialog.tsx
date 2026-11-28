import { useState } from "react";
import { Button, ChipListField, Dialog } from "../components";
import { useSettingsDialog } from "./useSettingsDialog";

/** Placeholder body: the sections are not built yet, and nothing here is saved. */
export function SettingsDialog() {
  const { open, close } = useSettingsDialog();
  const [values, setValues] = useState(["przykład-alfa", "przykład-beta"]);

  return (
    <Dialog
      open={open !== null}
      onClose={close}
      title="Ustawienia"
      footer={<Button onClick={close}>Zamknij</Button>}
    >
      <ChipListField
        label="Lista próbna"
        values={values}
        onChange={setValues}
        placeholder="wpisz i naciśnij Enter"
      />
    </Dialog>
  );
}
