import type { ReactNode } from "react";

const TONE = {
  info: {
    surface: "bg-mm-banner-info-background",
    accent: "text-mm-banner-info-accent",
    // A circled "i".
    icon: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm0 5.2a1.3 1.3 0 1 1 0 2.6 1.3 1.3 0 0 1 0-2.6ZM10.9 11h2.2v6h-2.2v-6Z",
  },
  success: {
    surface: "bg-mm-banner-success-background",
    accent: "text-mm-banner-success-accent",
    // A circled check mark.
    icon: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm4.7 7.7-5.5 6a1 1 0 0 1-1.5 0l-2.4-2.6 1.5-1.4 1.6 1.8 4.8-5.2 1.5 1.4Z",
  },
  warning: {
    surface: "bg-mm-banner-warning-background",
    accent: "text-mm-banner-warning-accent",
    // A triangle with "!".
    icon: "M12 2.5 1.5 20.5h21L12 2.5Zm-1.1 6.5h2.2v6h-2.2V9Zm1.1 7.3a1.3 1.3 0 1 1 0 2.6 1.3 1.3 0 0 1 0-2.6Z",
  },
  danger: {
    surface: "bg-mm-banner-danger-background",
    accent: "text-mm-banner-danger-accent",
    // A circled "!".
    icon: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm-1.1 5h2.2v7h-2.2V7Zm1.1 8.2a1.3 1.3 0 1 1 0 2.6 1.3 1.3 0 0 1 0-2.6Z",
  },
} as const;

export type BannerTone = keyof typeof TONE;

/**
 * A message about the whole view or a section of it. Each tone has its own icon shape, so the
 * tone does not depend on color. Problems (`warning`, `danger`) interrupt a screen reader;
 * `info` and `success` wait their turn.
 */
export function Banner({
  tone = "info",
  title,
  children,
}: {
  tone?: BannerTone;
  title?: string;
  children: ReactNode;
}) {
  const { surface, accent, icon } = TONE[tone];
  const urgent = tone === "danger" || tone === "warning";
  return (
    <div
      role={urgent ? "alert" : "status"}
      className={`flex items-start gap-3 rounded-mm-md p-3 text-mm-banner-text ${surface}`}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true" className={`mt-0.5 size-5 shrink-0 ${accent}`}>
        <path fill="currentColor" d={icon} />
      </svg>
      <div className="flex min-w-0 flex-col gap-0.5 text-mm-detail">
        {title && <p className="text-mm-label">{title}</p>}
        <div>{children}</div>
      </div>
    </div>
  );
}
