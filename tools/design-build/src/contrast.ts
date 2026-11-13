// WCAG 2 contrast checks, run on every token build so an unreadable pairing cannot be committed.

import { MODES, type Mode, type Model, type Rgba } from "./model.ts";

type Pair = {
  foreground: string;
  background: string;
  /** Minimum contrast ratio: 4.5 for body text, 3 for large text and interface parts. */
  minimum: number;
  /**
   * Opaque colors to composite a translucent background over. The overlay sits on arbitrary
   * screen content, so it has to hold up over both extremes.
   */
  over?: Rgba[];
};

const WHITE: Rgba = { r: 255, g: 255, b: 255, a: 1 };
const BLACK: Rgba = { r: 0, g: 0, b: 0, a: 1 };
const ANY_BACKDROP = [WHITE, BLACK];

const PAIRS: Pair[] = [
  { foreground: "text.primary", background: "background.base", minimum: 4.5 },
  { foreground: "text.primary", background: "background.surface", minimum: 4.5 },
  { foreground: "text.secondary", background: "background.base", minimum: 4.5 },
  { foreground: "text.secondary", background: "background.surface", minimum: 4.5 },
  { foreground: "text.on-accent", background: "accent.default", minimum: 4.5 },
  { foreground: "accent.default", background: "background.base", minimum: 3 },
  { foreground: "status.danger", background: "background.base", minimum: 4.5 },
  { foreground: "status.success", background: "background.base", minimum: 4.5 },
  { foreground: "status.warning", background: "background.base", minimum: 4.5 },
  { foreground: "data-path.standard", background: "background.base", minimum: 4.5 },
  { foreground: "data-path.zero-retention", background: "background.base", minimum: 4.5 },
  { foreground: "data-path.confidential", background: "background.base", minimum: 4.5 },
  {
    foreground: "overlay.text-primary",
    background: "overlay.surface",
    minimum: 4.5,
    over: ANY_BACKDROP,
  },
  {
    foreground: "overlay.text-uncertain",
    background: "overlay.surface",
    minimum: 4.5,
    over: ANY_BACKDROP,
  },
  // Draft captions are deliberately dimmer than settled ones, and are set in large type.
  {
    foreground: "overlay.text-draft",
    background: "overlay.surface",
    minimum: 3,
    over: ANY_BACKDROP,
  },
];

function composite(top: Rgba, bottom: Rgba): Rgba {
  const mix = (a: number, b: number) => a * top.a + b * (1 - top.a);
  return { r: mix(top.r, bottom.r), g: mix(top.g, bottom.g), b: mix(top.b, bottom.b), a: 1 };
}

function luminance({ r, g, b }: Rgba): number {
  const linear = (channel: number) => {
    const c = channel / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

export function contrastRatio(foreground: Rgba, background: Rgba): number {
  const [lighter, darker] = [luminance(foreground), luminance(background)].sort(
    (a, b) => b - a,
  ) as [number, number];
  return (lighter + 0.05) / (darker + 0.05);
}

/** Returns one message per failing pair. An empty list means every pair passes. */
export function checkContrast(model: Model): string[] {
  const byPath = new Map(model.colors.map((token) => [token.path.join("."), token]));
  const color = (path: string, mode: Mode): Rgba => {
    const token = byPath.get(path);
    if (!token) throw new Error(`contrast check refers to the unknown color "${path}"`);
    return token[mode];
  };

  const failures: string[] = [];
  for (const pair of PAIRS) {
    for (const mode of MODES) {
      for (const backdrop of pair.over ?? [WHITE]) {
        const background = composite(color(pair.background, mode), backdrop);
        const foreground = composite(color(pair.foreground, mode), background);
        const ratio = contrastRatio(foreground, background);
        if (ratio < pair.minimum) {
          const where = pair.over ? ` over rgb(${backdrop.r} ${backdrop.g} ${backdrop.b})` : "";
          failures.push(
            `${pair.foreground} on ${pair.background}${where} (${mode}): ` +
              `${ratio.toFixed(2)}:1, needs ${pair.minimum}:1`,
          );
        }
      }
    }
  }
  return failures;
}
