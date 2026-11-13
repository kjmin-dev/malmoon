import type { ReactNode } from "react";

const TONE = {
  neutral: "bg-mm-badge-neutral-background text-mm-badge-neutral-text",
  accent: "bg-mm-badge-accent-background text-mm-badge-accent-text",
  success: "bg-mm-badge-success-background text-mm-badge-success-text",
  warning: "bg-mm-badge-warning-background text-mm-badge-warning-text",
  danger: "bg-mm-badge-danger-background text-mm-badge-danger-text",
} as const;

export type BadgeTone = keyof typeof TONE;

/**
 * A short status or category. The words carry the meaning; the color only reinforces it, so a
 * badge still reads for someone who cannot tell the tones apart. `dot` adds a small mark for
 * live states such as "connected".
 */
export function Badge({
  tone = "neutral",
  dot = false,
  children,
}: {
  tone?: BadgeTone;
  dot?: boolean;
  children: ReactNode;
}) {
  return (
    <span
      className={`inline-flex h-6 w-fit shrink-0 items-center gap-1.5 whitespace-nowrap rounded-mm-sm px-2 text-mm-footnote ${TONE[tone]}`}
    >
      {dot && <span aria-hidden className="size-1.5 rounded-full bg-current" />}
      {children}
    </span>
  );
}
