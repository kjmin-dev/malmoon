import { typography } from "@malmoon/tokens";
import type { ReactNode } from "react";
import { Text as NativeText, type StyleProp, type TextStyle } from "react-native";
import { type Colors, useColors } from "./theme.ts";

export type TextVariant =
  | "display"
  | "title"
  | "heading"
  | "body"
  | "body-strong"
  | "label"
  | "detail"
  | "footnote"
  | "caption";

export type TextTone =
  | "primary"
  | "secondary"
  | "accent"
  | "danger"
  | "success"
  | "warning"
  | "inherit";

const STYLE: Record<TextVariant, TextStyle> = {
  display: typography.display,
  title: typography.title,
  heading: typography.heading,
  body: typography.body,
  "body-strong": typography.bodyStrong,
  label: typography.label,
  detail: typography.detail,
  footnote: typography.footnote,
  caption: typography.caption,
};

function toneColor(colors: Colors, tone: TextTone): string | undefined {
  switch (tone) {
    case "primary":
      return colors.textPrimary;
    case "secondary":
      return colors.textSecondary;
    case "accent":
      return colors.textAccent;
    case "danger":
      return colors.statusDanger;
    case "success":
      return colors.statusSuccess;
    case "warning":
      return colors.statusWarning;
    case "inherit":
      return undefined;
  }
}

export type TextProps = {
  /** What the text is for. Sets size, weight, line height and tracking together. */
  variant?: TextVariant;
  tone?: TextTone;
  /** Marks the text as a heading for screen readers. */
  heading?: boolean;
  numberOfLines?: number;
  style?: StyleProp<TextStyle>;
  children: ReactNode;
};

/**
 * Text in one of the design system's roles. It grows with the system's text size setting, and
 * Korean breaks between words rather than between syllables.
 */
export function Text({
  variant = "body",
  tone = "primary",
  heading = false,
  numberOfLines,
  style,
  children,
}: TextProps) {
  const color = toneColor(useColors(), tone);
  return (
    <NativeText
      {...(heading ? { accessibilityRole: "header" as const } : {})}
      {...(numberOfLines ? { numberOfLines } : {})}
      // Android has no equivalent yet.
      lineBreakStrategyIOS="hangul-word"
      style={[STYLE[variant], color ? { color } : null, style]}
    >
      {children}
    </NativeText>
  );
}
