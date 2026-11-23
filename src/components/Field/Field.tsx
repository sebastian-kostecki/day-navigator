import {
  useId,
  type InputHTMLAttributes,
  type SelectHTMLAttributes,
  type TextareaHTMLAttributes,
} from "react";
import "./Field.css";

type FieldBase = {
  /** Always visible; linked to the control through htmlFor. */
  label: string;
  /** Takes the free width in a wrapping form row. */
  grow?: boolean;
  id?: string;
  className?: string;
};

type InputField = FieldBase &
  Omit<InputHTMLAttributes<HTMLInputElement>, "id" | "className"> & {
    multiline?: false;
    options?: undefined;
  };

type TextareaField = FieldBase &
  Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, "id" | "className"> & {
    /** Renders a textarea. */
    multiline: true;
    options?: undefined;
  };

type SelectField = FieldBase &
  Omit<SelectHTMLAttributes<HTMLSelectElement>, "id" | "className"> & {
    /** Renders a select with these options. */
    options: string[];
    multiline?: false;
  };

export type FieldProps = InputField | TextareaField | SelectField;

export function Field(props: FieldProps) {
  const generatedId = useId();
  const id = props.id ?? generatedId;
  const classes = ["dn-field", props.grow && "is-grow", props.className].filter(Boolean).join(" ");

  let control;
  if (props.multiline === true) {
    const { label, grow, id: _id, className, multiline, options, ...rest } = props;
    void [label, grow, _id, className, multiline, options];
    control = <textarea id={id} className="dn-input" {...rest} />;
  } else if (props.options !== undefined) {
    const { label, grow, id: _id, className, multiline, options, ...rest } = props;
    void [label, grow, _id, className, multiline];
    control = (
      <select id={id} className="dn-input" {...rest}>
        {options.map((o) => (
          <option key={o} value={o}>
            {o}
          </option>
        ))}
      </select>
    );
  } else {
    const { label, grow, id: _id, className, multiline, options, ...rest } = props;
    void [label, grow, _id, className, multiline, options];
    control = <input id={id} type="text" className="dn-input" {...rest} />;
  }

  return (
    <div className={classes}>
      <label htmlFor={id} className="dn-field-label">
        {props.label}
      </label>
      {control}
    </div>
  );
}
