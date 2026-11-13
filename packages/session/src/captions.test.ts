// The same cases as the CaptionBuffer tests in crates/malmoon-core/src/caption.rs.

import assert from "node:assert/strict";
import { test } from "node:test";
import { applyCaption } from "./captions.ts";
import type { CaptionUpdate } from "./generated/CaptionUpdate.ts";

function update(
  segment_id: number,
  stage: CaptionUpdate["stage"],
  original: string,
  translation: string | null = null,
): CaptionUpdate {
  return { segment_id, channel: "system", stage, original, translation };
}

function run(maxFinal: number, updates: CaptionUpdate[]): readonly CaptionUpdate[] {
  return updates.reduce<readonly CaptionUpdate[]>(
    (segments, next) => applyCaption(segments, next, maxFinal),
    [],
  );
}

const texts = (segments: readonly CaptionUpdate[]) => segments.map((segment) => segment.original);

test("a draft is replaced in place", () => {
  const segments = run(3, [
    update(1, "draft", "Let's"),
    update(1, "draft", "Let's ship"),
    update(1, "final", "Let's ship it."),
  ]);
  assert.deepEqual(texts(segments), ["Let's ship it."]);
});

test("a late draft does not undo a final", () => {
  const settled = run(3, [update(1, "final", "Done.")]);
  assert.equal(applyCaption(settled, update(1, "draft", "Don"), 3), settled);
});

test("a final can be refined, and an identical update changes nothing", () => {
  const settled = run(3, [update(1, "final", "Done.")]);
  const refined = applyCaption(settled, update(1, "final", "Done.", "끝났습니다."), 3);
  assert.equal(refined[0]?.translation, "끝났습니다.");
  assert.equal(applyCaption(refined, update(1, "final", "Done.", "끝났습니다."), 3), refined);
});

test("the oldest finals are dropped and drafts survive", () => {
  const segments = run(2, [
    update(1, "final", "one"),
    update(2, "draft", "two"),
    update(3, "final", "three"),
    update(4, "final", "four"),
  ]);
  assert.deepEqual(texts(segments), ["two", "three", "four"]);
});

test("segments stay in speaking order when updates interleave", () => {
  const segments = run(5, [update(2, "draft", "second"), update(1, "draft", "first")]);
  assert.deepEqual(texts(segments), ["first", "second"]);
});

test("a segment older than a full buffer is ignored", () => {
  const full = run(1, [update(5, "final", "five")]);
  assert.equal(applyCaption(full, update(2, "final", "two"), 1), full);
});
