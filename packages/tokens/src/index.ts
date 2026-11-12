// Malmoon design tokens for TypeScript. The values come from design/tokens; see
// tools/design-build. Web apps style with the CSS exports instead:
//
//   @import "@malmoon/tokens/theme.css";     custom properties
//   @import "@malmoon/tokens/tailwind.css";  Tailwind theme that points at them

import type { colors } from "./generated/tokens.ts";

export * from "./generated/tokens.ts";

export type ColorMode = keyof typeof colors;
export type ColorName = keyof (typeof colors)["light"];
