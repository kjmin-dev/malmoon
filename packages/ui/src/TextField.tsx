import { Description, FieldError, TextField as HeroTextField, Input, Label } from "@heroui/react";
import type { HTMLInputAutoCompleteAttribute, HTMLInputTypeAttribute } from "react";

export type TextFieldProps = {
  /** Always visible. A placeholder is not a label: it disappears as soon as the person types. */
  label: string;
  value: string;
  onChange: (value: string) => void;
  /** Help shown under the field. Replaced by `error` while there is one. */
  description?: string;
  /** What is wrong and how to fix it. Clear it when the value changes. */
  error?: string | undefined;
  placeholder?: string;
  name?: string;
  type?: HTMLInputTypeAttribute;
  inputMode?: "text" | "numeric" | "decimal" | "tel" | "email" | "url" | "search";
  autoComplete?: HTMLInputAutoCompleteAttribute;
  autoFocus?: boolean;
  required?: boolean;
  disabled?: boolean;
};

/**
 * A labelled single-line text field. The label, the description and the error message are tied
 * to the input for assistive technology, and the error is announced when it appears.
 *
 * An invalid field blocks its form from being submitted, so `error` has to be cleared as soon as
 * the person edits the value.
 */
export function TextField({
  label,
  value,
  onChange,
  description,
  error,
  placeholder,
  name,
  type = "text",
  inputMode,
  autoComplete,
  autoFocus = false,
  required = false,
  disabled = false,
}: TextFieldProps) {
  return (
    <HeroTextField
      fullWidth
      type={type}
      value={value}
      onChange={onChange}
      isInvalid={error !== undefined}
      isRequired={required}
      isDisabled={disabled}
      autoFocus={autoFocus}
      {...(name ? { name } : {})}
      {...(inputMode ? { inputMode } : {})}
      {...(autoComplete ? { autoComplete } : {})}
    >
      <Label>{label}</Label>
      <Input {...(placeholder ? { placeholder } : {})} />
      {description && <Description>{description}</Description>}
      <FieldError>{error}</FieldError>
    </HeroTextField>
  );
}
