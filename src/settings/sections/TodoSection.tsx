import type { SectionErrors } from "../fieldErrors";
import type { TodoForm } from "../types";
import { TextSetting } from "./controls";

type Props = {
  form: TodoForm;
  onChange: (form: TodoForm) => void;
  errors: SectionErrors;
};

export function TodoSection({ form, onChange, errors }: Props) {
  return (
    <TextSetting
      label="Limit pozycji na liście „Do zrobienia”"
      value={form.limit}
      onChange={(limit) => onChange({ ...form, limit })}
      error={errors.field("todo.limit")}
      placeholder="5"
      inputMode="numeric"
    />
  );
}
