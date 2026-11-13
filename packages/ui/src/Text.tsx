import type { ElementType, ReactNode } from "react";

// Written out in full: Tailwind only generates the classes it can find as whole strings.
const VARIANT = {
  display: "text-mm-display",
  title: "text-mm-title",
  heading: "text-mm-heading",
  body: "text-mm-body",
  "body-strong": "text-mm-body-strong",
  label: "text-mm-label",
  detail: "text-mm-detail",
  footnote: "text-mm-footnote",
  caption: "text-mm-caption",
} as const;

const TONE = {
  primary: "text-mm-text-primary",
  secondary: "text-mm-text-secondary",
  accent: "text-mm-text-accent",
  danger: "text-mm-status-danger",
  success: "text-mm-status-success",
  warning: "text-mm-status-warning",
  /** Takes the color of the surrounding text. */
  inherit: "",
} as const;

export type TextVariant = keyof typeof VARIANT;
export type TextTone = keyof typeof TONE;

export type TextProps = {
  /** What the text is for. Sets size, weight, line height and tracking together. */
  variant?: TextVariant;
  tone?: TextTone;
  /** The element to render. Pick it for the document outline (`h1`, `h2`, `p`), not for looks. */
  as?: ElementType;
  className?: string;
  children: ReactNode;
};

export function Text({
  variant = "body",
  tone = "primary",
  as: Element = "p",
  className = "",
  children,
}: TextProps) {
  return <Element className={`${VARIANT[variant]} ${TONE[tone]} ${className}`}>{children}</Element>;
}
