import type { CaptionUpdate } from "./generated/CaptionUpdate.ts";

/**
 * Applies one caption update to a list of segments and returns the new list, or the same list if
 * the update was stale and changed nothing. At most `maxFinal` settled segments are kept; drafts
 * always are.
 *
 * This mirrors `CaptionBuffer` in malmoon-core and is tested against the same cases. It exists
 * only until the core is available to TypeScript as WebAssembly and native bindings.
 */
export function applyCaption(
  segments: readonly CaptionUpdate[],
  update: CaptionUpdate,
  maxFinal: number,
): readonly CaptionUpdate[] {
  const finalCount = segments.filter((segment) => segment.stage === "final").length;
  const index = segments.findIndex((segment) => segment.segment_id >= update.segment_id);
  const current = index === -1 ? undefined : segments[index];
  let next: CaptionUpdate[];

  if (current?.segment_id === update.segment_id) {
    // A draft that arrives after its segment settled is out of order: ignore it.
    if (current.stage === "final" && update.stage === "draft") return segments;
    if (
      current.stage === update.stage &&
      current.channel === update.channel &&
      current.original === update.original &&
      current.translation === update.translation
    ) {
      return segments;
    }
    next = [...segments.slice(0, index), update, ...segments.slice(index + 1)];
  } else {
    // A settled segment older than everything kept has already scrolled away.
    if (update.stage === "final" && index === 0 && finalCount >= maxFinal) return segments;
    const at = index === -1 ? segments.length : index;
    next = [...segments.slice(0, at), update, ...segments.slice(at)];
  }

  let excess = next.filter((segment) => segment.stage === "final").length - maxFinal;
  return next.filter((segment) => {
    if (excess > 0 && segment.stage === "final") {
      excess -= 1;
      return false;
    }
    return true;
  });
}
