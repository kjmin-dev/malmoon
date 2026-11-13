import { border, radius, space, typography } from "@malmoon/tokens";
import { useState } from "react";
import { StyleSheet, TextInput, type TextInputProps, View } from "react-native";
import { Text } from "./Text.tsx";
import { CONTROL_HEIGHT, useColors } from "./theme.ts";

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
  disabled?: boolean;
  /** Called when the person presses the keyboard's return key. */
  onSubmit?: () => void;
} & Pick<
  TextInputProps,
  | "autoCapitalize"
  | "autoComplete"
  | "autoFocus"
  | "keyboardType"
  | "returnKeyType"
  | "secureTextEntry"
  | "textContentType"
>;

/** A labelled single-line text field. */
export function TextField({
  label,
  value,
  onChange,
  description,
  error,
  placeholder,
  disabled = false,
  onSubmit,
  ...input
}: TextFieldProps) {
  const colors = useColors();
  const [focused, setFocused] = useState(false);
  const invalid = error !== undefined;
  // Focus and errors thicken the outline as well as recoloring it, so neither depends on color
  // alone. The padding shrinks by the same amount, so the text does not move.
  const emphasized = focused || invalid;
  const outline = emphasized ? border.width.thick : border.width.thin;
  const outlineColor = invalid
    ? colors.fieldBorderInvalid
    : focused
      ? colors.fieldBorderFocus
      : colors.fieldBorder;

  return (
    <View style={styles.field}>
      <Text variant="label" tone={disabled ? "secondary" : "primary"}>
        {label}
      </Text>
      <TextInput
        {...input}
        accessibilityLabel={label}
        accessibilityHint={error ?? description}
        aria-invalid={invalid}
        editable={!disabled}
        value={value}
        onChangeText={onChange}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        {...(onSubmit ? { onSubmitEditing: onSubmit } : {})}
        placeholder={placeholder}
        placeholderTextColor={colors.fieldPlaceholder}
        style={[
          styles.input,
          {
            borderWidth: outline,
            borderColor: outlineColor,
            paddingHorizontal: space[3] - (outline - border.width.thin),
            color: disabled ? colors.textDisabled : colors.fieldText,
            backgroundColor: disabled ? colors.backgroundDisabled : colors.fieldBackground,
          },
        ]}
      />
      {invalid ? (
        <View accessibilityRole="alert" accessibilityLiveRegion="polite">
          <Text variant="detail" tone="danger">
            {error}
          </Text>
        </View>
      ) : (
        description && (
          <Text variant="detail" tone="secondary">
            {description}
          </Text>
        )
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  field: {
    alignSelf: "stretch",
    gap: space[1],
  },
  input: {
    minHeight: CONTROL_HEIGHT.md,
    borderRadius: radius.md,
    // No line height: iOS centers single-line input text on its own, and a line height
    // pushes it off center.
    fontSize: typography.body.fontSize,
  },
});
