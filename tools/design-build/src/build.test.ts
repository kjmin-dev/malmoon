import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { checkContrast, contrastRatio } from "./contrast.ts";
import { emitCss, emitRust, emitTailwind, emitTypeScript } from "./emit.ts";
import { loadModel, type Model, TokenError } from "./model.ts";

const tokensDir = join(import.meta.dirname, "../../../design/tokens");

const srgb = (components: number[], extra: object = {}) => ({
  $type: "color",
  $value: { colorSpace: "srgb", components, ...extra },
});

/** Writes a minimal token set, with `base` merged into base.json, and loads it. */
async function loadWith(base: object, dark: object = { color: { ink: srgb([1, 1, 1]) } }) {
  const dir = await mkdtemp(join(tmpdir(), "malmoon-tokens-"));
  await writeFile(join(dir, "base.json"), JSON.stringify(base));
  await writeFile(
    join(dir, "color.light.json"),
    JSON.stringify({ color: { ink: srgb([0, 0, 0]) } }),
  );
  await writeFile(join(dir, "color.dark.json"), JSON.stringify(dark));
  return loadModel(dir);
}

test("the committed tokens load and pass every contrast check", async () => {
  const model = await loadModel(tokensDir);
  assert.ok(model.colors.length > 0);
  assert.deepEqual(checkContrast(model), []);
});

test("a low-contrast pairing is reported with its mode", async () => {
  const model = await loadModel(tokensDir);
  const colors = model.colors.map((token) =>
    token.path.join(".") === "text.secondary"
      ? { ...token, light: { r: 240, g: 240, b: 240, a: 1 } }
      : token,
  );
  const failures = checkContrast({ ...model, colors });
  assert.equal(failures.length, 2);
  assert.match(failures[0] ?? "", /^text\.secondary on background\.base \(light\): 1\.14:1/);
});

test("contrast ratio matches the WCAG reference values", () => {
  const black = { r: 0, g: 0, b: 0, a: 1 };
  const white = { r: 255, g: 255, b: 255, a: 1 };
  assert.equal(contrastRatio(black, white), 21);
  assert.equal(contrastRatio(white, white), 1);
});

test("a color outside sRGB is rejected", async () => {
  const base = {
    color: {
      wide: { $type: "color", $value: { colorSpace: "display-p3", components: [1, 0, 0] } },
    },
  };
  await assert.rejects(loadWith(base), (error: unknown) => {
    assert.ok(error instanceof TokenError);
    assert.match(error.message, /^color\.wide: color space "display-p3" is not supported/);
    return true;
  });
});

test("a hex that disagrees with the components is rejected", async () => {
  const base = { color: { off: srgb([1, 0, 0], { hex: "#00ff00" }) } };
  await assert.rejects(
    loadWith(base),
    /hex #00ff00 does not match the components, which give #ff0000/,
  );
});

test("an inset shadow is rejected", async () => {
  const px = (value: number) => ({ value, unit: "px" });
  const shadow = {
    $type: "shadow",
    $value: {
      color: srgb([0, 0, 0]).$value,
      offsetX: px(0),
      offsetY: px(1),
      blur: px(2),
      spread: px(0),
      inset: true,
    },
  };
  await assert.rejects(
    loadWith({ shadow: { pressed: shadow } }),
    /inset shadows are not supported/,
  );
});

test("a color missing from one mode is rejected", async () => {
  await assert.rejects(
    loadWith({}, { color: {} }),
    /color\.ink: is defined for light mode but not for dark mode/,
  );
});

const sample: Model = {
  colors: [
    {
      path: ["text", "on-accent"],
      description: "Text on accent.",
      light: { r: 255, g: 255, b: 255, a: 1 },
      dark: { r: 16, g: 16, b: 20, a: 0.5 },
    },
    {
      path: ["overlay", "surface"],
      light: { r: 16, g: 16, b: 20, a: 0.72 },
      dark: { r: 16, g: 16, b: 20, a: 0.72 },
    },
  ],
  values: [
    { path: ["radius", "md"], value: { kind: "dimension", px: 10 } },
    { path: ["font", "line-height", "tight"], value: { kind: "number", value: 1.25 } },
    { path: ["motion", "duration", "fast"], value: { kind: "duration", ms: 120 } },
  ],
};

test("CSS follows the color scheme only where the modes differ", () => {
  const css = emitCss(sample);
  assert.match(css, /--mm-color-text-on-accent: light-dark\(#ffffff, rgb\(16 16 20 \/ 0\.5\)\);/);
  assert.match(css, /--mm-color-overlay-surface: rgb\(16 16 20 \/ 0\.72\);/);
  assert.match(css, /--mm-radius-md: 10px;/);
});

test("the Tailwind theme is namespaced and skips groups without a utility", () => {
  const theme = emitTailwind(sample);
  assert.match(theme, /--color-mm-overlay-surface: var\(--mm-color-overlay-surface\);/);
  assert.match(theme, /--radius-mm-md: var\(--mm-radius-md\);/);
  assert.match(theme, /--leading-mm-tight: var\(--mm-font-line-height-tight\);/);
  assert.doesNotMatch(theme, /duration/);
});

test("TypeScript uses camelCase keys and plain numbers", () => {
  const source = emitTypeScript(sample);
  assert.match(source, /"textOnAccent": "rgba\(16, 16, 20, 0\.5\)"/);
  assert.match(source, /export const font = \{\n {2}"lineHeight": \{\n {4}"tight": 1\.25/);
  assert.match(source, /"fast": 120/);
});

test("Rust gets one struct field per color and typed constants", () => {
  const source = emitRust(sample);
  assert.match(source, /\/\/\/ Text on accent\.\n {4}pub text_on_accent: Color,/);
  assert.match(source, /text_on_accent: Color::rgba\(16, 16, 20, 0\.5\),/);
  assert.match(source, /pub const RADIUS_MD: f32 = 10\.0;/);
  assert.match(source, /pub const MOTION_DURATION_FAST: Duration = Duration::from_millis\(120\);/);
});
