import type { ReactNode } from "react";

/**
 * A surface that groups related content. The outline, not a shadow, sets it apart: shadows
 * barely show in dark mode. `title` renders as a heading; `actions` sit across from it.
 */
export function Card({
  title,
  description,
  actions,
  className = "",
  children,
}: {
  title?: string;
  description?: string;
  actions?: ReactNode;
  className?: string;
  children?: ReactNode;
}) {
  return (
    <section
      className={`flex flex-col gap-4 rounded-mm-lg border border-mm-border-default bg-mm-background-surface p-4 ${className}`}
    >
      {(title || actions) && (
        <header className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            {title && <h2 className="text-mm-heading text-mm-text-primary">{title}</h2>}
            {description && <p className="text-mm-detail text-mm-text-secondary">{description}</p>}
          </div>
          {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
        </header>
      )}
      {children}
    </section>
  );
}
