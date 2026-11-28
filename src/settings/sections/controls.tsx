import { useId, type InputHTMLAttributes } from "react";
import { Button, ChipListField, Field, Notice } from "../../components";
import type { ListErrors } from "../fieldErrors";

type TextSettingProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  error: string | undefined;
  placeholder?: string;
  inputMode?: InputHTMLAttributes<HTMLInputElement>["inputMode"];
  disabled?: boolean;
};

/** A text input with its message under it. Numbers go to Rust as typed. */
export function TextSetting({ label, value, onChange, error, ...rest }: TextSettingProps) {
  const errorId = useId();
  return (
    <div className="dn-settings-field">
      <Field
        label={label}
        value={value}
        onChange={(e) => onChange(e.currentTarget.value)}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        autoComplete="off"
        spellCheck={false}
        {...rest}
      />
      {error && (
        <div id={errorId}>
          <Notice tone="error" compact role={null}>
            {error}
          </Notice>
        </div>
      )}
    </div>
  );
}

type ListSettingProps = {
  label: string;
  values: string[];
  onChange: (values: string[]) => void;
  errors: ListErrors;
  max: number;
  placeholder?: string;
};

export function ListSetting({ errors, ...rest }: ListSettingProps) {
  return (
    <div className="dn-settings-field">
      <ChipListField errors={errors.items} {...rest} />
      {errors.general && (
        <Notice tone="error" compact role={null}>
          {errors.general}
        </Notice>
      )}
    </div>
  );
}

type SwitchSettingProps = {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** One line under the label. */
  hint?: string;
  error?: string;
};

export function SwitchSetting({ label, checked, onChange, hint, error }: SwitchSettingProps) {
  const hintId = useId();
  return (
    <div className="dn-settings-field">
      <div className="dn-settings-switch">
        <div className="dn-settings-switch-text">
          <span className="body">{label}</span>
          {hint && (
            <span id={hintId} className="meta">
              {hint}
            </span>
          )}
        </div>
        <Button
          size="sm"
          variant="quiet"
          pressed={checked}
          aria-label={label}
          aria-describedby={hint ? hintId : undefined}
          onClick={() => onChange(!checked)}
        >
          {checked ? "WŁ." : "WYŁ."}
        </Button>
      </div>
      {error && (
        <Notice tone="error" compact role={null}>
          {error}
        </Notice>
      )}
    </div>
  );
}
