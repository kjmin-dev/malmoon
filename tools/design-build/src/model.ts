// Loads the DTCG token files and turns them into a small, validated model that the emitters share.
//
// Style Dictionary does the parsing and alias resolution. Everything platform-specific happens in
// emit.ts, so a value that some platform cannot express is rejected here, once, for all of them.

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

export type Value =
  | { kind: "dimension"; px: number }
  | { kind: "duration"; ms: number }
  | { kind: "number"; value: number }
  | { kind: "fontWeight"; value: number }
  | { kind: "fontFamily"; families: string[] }
  | { kind: "cubicBezier"; points: [number, number, number, number] }
  | { kind: "shadow"; shadow: Shadow };

export type ColorToken = { path: string[]; description?: string; light: Rgba; dark: Rgba };
export type ValueToken = { path: string[]; description?: string; value: Value };

export type Model = {
  /** Paths are relative to the top-level `color` group. */
  colors: ColorToken[];
  values: ValueToken[];
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
    default:
      return fail(path, `token type "${String(token.$type)}" is not supported`);
  }
}

async function loadMode(tokensDir: string, mode: Mode): Promise<RawToken[]> {
  const dictionary = new StyleDictionary({
    source: [join(tokensDir, "base.json"), join(tokensDir, `color.${mode}.json`)],
    usesDtcg: true,
    log: { warnings: "error", verbosity: "default" },
    // No transforms: the emitters need the resolved DTCG values, not CSS strings.
    platforms: { raw: { transforms: [] } },
  });
  const { allTokens } = await dictionary.getPlatformTokens("raw");
  return allTokens as unknown as RawToken[];
}

export async function loadModel(tokensDir: string): Promise<Model> {
  const [light, dark] = await Promise.all(MODES.map((mode) => loadMode(tokensDir, mode)));
  if (!light || !dark) throw new TokenError("both color modes must load");

  const darkByPath = new Map(dark.map((token) => [token.path.join("."), token]));
  const colors: ColorToken[] = [];
  const values: ValueToken[] = [];

  for (const token of light) {
    const key = token.path.join(".");
    const counterpart = darkByPath.get(key);
    if (!counterpart) fail(token.path, "is defined for light mode but not for dark mode");
    darkByPath.delete(key);

    const description = token.$description;
    if (token.$type === "color") {
      if (token.path[0] !== "color") fail(token.path, 'color tokens must live under "color"');
      colors.push({
        path: token.path.slice(1),
        ...(description ? { description } : {}),
        light: parseColor(token.path, token.$value),
        dark: parseColor(token.path, counterpart.$value),
      });
    } else {
      values.push({
        path: token.path,
        ...(description ? { description } : {}),
        value: parseValue(token),
      });
    }
  }
  for (const token of darkByPath.values()) {
    fail(token.path, "is defined for dark mode but not for light mode");
  }
  return { colors, values };
}
