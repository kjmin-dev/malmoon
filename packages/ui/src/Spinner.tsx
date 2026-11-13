const SIZE = { sm: "size-4", md: "size-6", lg: "size-8" } as const;

/**
 * Says that something is in progress and how long is unknown. Takes the color of the text around
 * it. With `label` it is announced; without one it is decoration next to text that already says
 * what is happening.
 */
export function Spinner({ size = "md", label }: { size?: keyof typeof SIZE; label?: string }) {
  return (
    <span
      className="inline-flex shrink-0"
      {...(label ? { role: "status", "aria-label": label } : { "aria-hidden": true })}
    >
      <svg
        viewBox="0 0 24 24"
        fill="none"
        aria-hidden="true"
        // People who ask for less motion get a slow pulse instead of a spin, not a frozen ring.
        className={`${SIZE[size]} animate-spin motion-reduce:animate-pulse`}
      >
        <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="3" opacity="0.25" />
        <path
          d="M21 12a9 9 0 0 0-9-9"
          stroke="currentColor"
          strokeWidth="3"
          strokeLinecap="round"
        />
      </svg>
    </span>
  );
}
