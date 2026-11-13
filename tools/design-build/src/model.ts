// Loads the DTCG token files and turns them into a small, validated model that the emitters share.
//
// Style Dictionary does the parsing and alias resolution. Everything platform-specific happens in
// emit.ts, so a value that some platform cannot express is rejected here, once, for all of them.

import { existsSync } from "node:fs";
import { join } from "node:path";
import StyleDictionary from "style-dictionary";

export type Mode = "light" | "dark";
export const MODES: readonly Mode[] = ["light", "dark"];

/** sRGB, channels 0-255, straight alpha 0-1. */
export type Rgba = { r: number; g: number; b: number; a: number };

export type Shadow = {
  color: Rgba;
  offsetX: number;
  offsetY: number;
  blur: number;
  spread: number;
};

/** A text style. `lineHeight` is a multiple of the size, as in the token files. */
export type Typography = {
  families: string[];
  sizePx: number;
  weight: number;
  lineHeight: number;
  letterSpacingPx: number;
};

export type Value =
  | { kind: "dimension"; px: number }
  | { kind: "duration"; ms: number }
  | { kind: "number"; value: number }
  | { kind: "fontWeight"; value: number }
  | { kind: "fontFamily"; families: string[] }
  | { kind: "cubicBezier"; points: [number, number, number, number] }
  | { kind: "shadow"; shadow: Shadow }
  | { kind: "typography"; typography: Typography };

/** How the person points at the screen. Sizes differ between the two; nothing else does. */
export type InputMode = "pointer" | "touch";
export const INPUT_MODES: readonly InputMode[] = ["pointer", "touch"];

export type ColorToken = { path: string[]; description?: string; light: Rgba; dark: Rgba };
export type ValueToken = { path: string[]; description?: string; value: Value };
/** A length that depends on the input mode, in pixels. */
export type SizeToken = { path: string[]; description?: string; pointer: number; touch: number };

export type Model = {
  /** Paths are relative to the top-level `color` group. */
  colors: ColorToken[];
  values: ValueToken[];
  /** Paths are relative to the top-level `size` group. */
  sizes: SizeToken[];
};

type RawToken = { path: string[]; $type?: string; $value: unknown; $description?: string };

export class TokenError extends Error {}

function fail(path: string[], message: string): never {
  throw new TokenError(`${path.join(".")}: ${message}`);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function toHex(channels: number[]): string {
  return `#${channels.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
}

function parseColor(path: string[], value: unknown): Rgba {
  if (!isRecord(value)) fail(path, "a color must be a DTCG color object");
  // GPUI and React Native only take sRGB, so wide-gamut tokens would silently differ per platform.
  if (value.colorSpace !== "srgb") {
    fail(path, `color space "${String(value.colorSpace)}" is not supported, use "srgb"`);
  }
  const components = value.components;
  if (!Array.isArray(components) || components.length !== 3) {
    fail(path, "an sRGB color needs exactly three components");
  }
  const channels = components.map((component) => {
    if (typeof component !== "number" || component < 0 || component > 1) {
      fail(path, "sRGB components must be numbers between 0 and 1");
    }
    return Math.round(component * 255);
  });
  const alpha = value.alpha ?? 1;
  if (typeof alpha !== "number" || alpha < 0 || alpha > 1) {
    fail(path, "alpha must be a number between 0 and 1");
  }
  if (typeof value.hex === "string" && value.hex.toLowerCase() !== toHex(channels)) {
    fail(path, `hex ${value.hex} does not match the components, which give ${toHex(channels)}`);
  }
  const [r, g, b] = channels as [number, number, number];
  return { r, g, b, a: alpha };
}

function parseUnit(path: string[], value: unknown, unit: string, what: string): number {
  if (!isRecord(value) || typeof value.value !== "number" || value.unit !== unit) {
    fail(path, `${what} must be written as { "value": <number>, "unit": "${unit}" }`);
  }
  return value.value;
}

function parseValue(token: RawToken): Value {
  const { path, $value: value } = token;
  switch (token.$type) {
    case "dimension":
      return { kind: "dimension", px: parseUnit(path, value, "px", "a dimension") };
    case "duration":
      return { kind: "duration", ms: parseUnit(path, value, "ms", "a duration") };
    case "number":
    case "fontWeight":
      if (typeof value !== "number") fail(path, `a ${token.$type} must be a number`);
      return { kind: token.$type, value };
    case "fontFamily": {
      const families = typeof value === "string" ? [value] : value;
      if (!Array.isArray(families) || !families.every((family) => typeof family === "string")) {
        fail(path, "a fontFamily must be a string or a list of strings");
      }
      return { kind: "fontFamily", families };
    }
    case "cubicBezier":
      if (
        !Array.isArray(value) ||
        value.length !== 4 ||
        !value.every((point) => typeof point === "number")
      ) {
        fail(path, "a cubicBezier must be four numbers");
      }
      return { kind: "cubicBezier", points: value as [number, number, number, number] };
    case "shadow": {
      if (!isRecord(value)) fail(path, "a shadow must be a single shadow object");
      // GPUI draws outer shadows only.
      if (value.inset === true) fail(path, "inset shadows are not supported on every platform");
      return {
        kind: "shadow",
        shadow: {
          color: parseColor(path, value.color),
          offsetX: parseUnit(path, value.offsetX, "px", "offsetX"),
          offsetY: parseUnit(path, value.offsetY, "px", "offsetY"),
          blur: parseUnit(path, value.blur, "px", "blur"),
          spread: parseUnit(path, value.spread, "px", "spread"),
        },
      };
    }
    case "typography": {
      if (!isRecord(value)) fail(path, "a typography token must be an object");
      const part = ($type: string, key: string) =>
        parseValue({ path: [...path, key], $type, $value: value[key] });
      const family = part("fontFamily", "fontFamily");
      const size = part("dimension", "fontSize");
      const weight = part("fontWeight", "fontWeight");
      const lineHeight = part("number", "lineHeight");
      const spacing = part("dimension", "letterSpacing");
      if (
        family.kind !== "fontFamily" ||
        size.kind !== "dimension" ||
        weight.kind !== "fontWeight" ||
        lineHeight.kind !== "number" ||
        spacing.kind !== "dimension"
      ) {
        return fail(path, "a typography token is malformed");
      }
      return {
        kind: "typography",
        typography: {
          families: family.families,
          sizePx: size.px,
          weight: weight.value,
          lineHeight: lineHeight.value,
          letterSpacingPx: spacing.px,
        },
      };
    }
    default:
      return fail(path, `token type "${String(token.$type)}" is not supported`);
  }
}

async function loadFiles(tokensDir: string, files: string[]): Promise<RawToken[]> {
  const dictionary = new StyleDictionary({
    source: files.map((file) => join(tokensDir, file)).filter((file) => existsSync(file)),
    usesDtcg: true,
    log: { warnings: "error", verbosity: "default" },
    // No transforms: the emitters need the resolved DTCG values, not CSS strings.
    platforms: { raw: { transforms: [] } },
  });
  const { allTokens } = await dictionary.getPlatformTokens("raw");
  return allTokens as unknown as RawToken[];
}

/** Pairs up a token set that exists once per mode, and insists that both modes define it. */
function pairUp<M extends string>(
  [first, second]: readonly [M, M],
  sets: readonly [RawToken[], RawToken[]],
  each: (token: RawToken, counterpart: RawToken) => void,
): void {
  const others = new Map(sets[1].map((token) => [token.path.join("."), token]));
  for (const token of sets[0]) {
    const key = token.path.join(".");
    const counterpart = others.get(key);
    if (!counterpart) fail(token.path, `is defined for ${first} mode but not for ${second} mode`);
    others.delete(key);
    each(token, counterpart);
  }
  for (const token of others.values()) {
    fail(token.path, `is defined for ${second} mode but not for ${first} mode`);
  }
}

export async function loadModel(tokensDir: string): Promise<Model> {
  // The palette is loaded so that the color files can refer to it, and then left out: only
  // colors with a meaning reach the platforms.
  const colorFiles = (mode: Mode) => ["palette.json", "base.json", `color.${mode}.json`];
  const [light, dark, pointer, touch] = await Promise.all([
    loadFiles(tokensDir, colorFiles("light")),
    loadFiles(tokensDir, colorFiles("dark")),
    loadFiles(tokensDir, ["size.pointer.json"]),
    loadFiles(tokensDir, ["size.touch.json"]),
  ]);

  const colors: ColorToken[] = [];
  const values: ValueToken[] = [];
  pairUp(["light", "dark"], [light, dark], (token, counterpart) => {
    const description = token.$description ? { description: token.$description } : {};
    if (token.path[0] === "palette") {
      // Checked even though it is not emitted: a bad ramp value would surface as a bad alias.
      parseColor(token.path, token.$value);
    } else if (token.$type === "color") {
      if (token.path[0] !== "color") fail(token.path, 'color tokens must live under "color"');
      colors.push({
        path: token.path.slice(1),
        ...description,
        light: parseColor(token.path, token.$value),
        dark: parseColor(token.path, counterpart.$value),
      });
    } else {
      values.push({ path: token.path, ...description, value: parseValue(token) });
    }
  });

  const sizes: SizeToken[] = [];
  pairUp(["pointer", "touch"], [pointer, touch], (token, counterpart) => {
    if (token.path[0] !== "size" || token.$type !== "dimension") {
      fail(token.path, 'the size files hold dimensions under "size" and nothing else');
    }
    sizes.push({
      path: token.path.slice(1),
      ...(token.$description ? { description: token.$description } : {}),
      pointer: parseUnit(token.path, token.$value, "px", "a size"),
      touch: parseUnit(token.path, counterpart.$value, "px", "a size"),
    });
  });
  return { colors, values, sizes };
}
