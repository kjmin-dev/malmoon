import type { CaptionUpdate } from "@malmoon/session";

/**
 * Live captions on the prompter's own surface: dark and translucent in both color schemes, like
 * the desktop overlay. Draft segments are dimmed because recognition may still revise them.
 */
export function Captions({ segments }: { segments: readonly CaptionUpdate[] }) {
  return (
    <div className="flex min-h-48 flex-col justify-end gap-3 rounded-mm-lg bg-mm-overlay-surface p-4 shadow-mm-overlay">
      {segments.length === 0 && (
        <p className="text-mm-body text-mm-overlay-text-draft">자막을 기다리는 중</p>
      )}
      {segments.map((segment) => (
        <div key={segment.segment_id}>
          <p
            className={`text-mm-caption transition-colors duration-200 ease-mm-standard ${
              segment.stage === "draft"
                ? "text-mm-overlay-text-draft"
                : "text-mm-overlay-text-primary"
            }`}
          >
            {segment.original}
          </p>
          {segment.translation && (
            <p className="text-mm-body text-mm-overlay-text-draft">{segment.translation}</p>
          )}
        </div>
      ))}
    </div>
  );
}
