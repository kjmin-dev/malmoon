// Generates the platform token files from design/tokens.
//
//   node src/build.ts           write the generated files
//   node src/build.ts --check   fail if the committed files are out of date (used by CI)

import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { checkContrast } from "./contrast.ts";
import { emitCss, emitRust, emitTailwind, emitTypeScript } from "./emit.ts";
import { loadModel, TokenError } from "./model.ts";

const root = join(import.meta.dirname, "../../..");
const check = process.argv.includes("--check");

async function main(): Promise<number> {
  const model = await loadModel(join(root, "design/tokens"));

  const failures = checkContrast(model);
  if (failures.length > 0) {
    console.error("Contrast is too low:");
    for (const failure of failures) console.error(`  ${failure}`);
    return 1;
  }

  const outputs: [path: string, content: string][] = [
    ["packages/tokens/src/generated/theme.css", emitCss(model)],
    ["packages/tokens/src/generated/tailwind.css", emitTailwind(model)],
    ["packages/tokens/src/generated/tokens.ts", emitTypeScript(model)],
    ["crates/malmoon-tokens/src/generated.rs", emitRust(model)],
  ];

  const stale: string[] = [];
  for (const [path, content] of outputs) {
    const file = join(root, path);
    const current = await readFile(file, "utf8").catch(() => undefined);
    if (current === content) continue;
    stale.push(path);
    if (!check) {
      await mkdir(dirname(file), { recursive: true });
      await writeFile(file, content);
    }
  }

  const summary = `${model.colors.length} colors, ${model.values.length} other tokens`;
  if (check && stale.length > 0) {
    console.error(`Generated token files are out of date (${summary}):`);
    for (const path of stale) console.error(`  ${path}`);
    console.error("Run `mise run //:tokens` and commit the result.");
    return 1;
  }
  const verb = check
    ? "up to date"
    : stale.length > 0
      ? `wrote ${stale.length} files`
      : "no changes";
  console.log(`tokens: ${summary}, ${verb}`);
  return 0;
}

try {
  process.exitCode = await main();
} catch (error) {
  if (!(error instanceof TokenError)) throw error;
  console.error(`Invalid token in ${relative(process.cwd(), join(root, "design/tokens"))}:`);
  console.error(`  ${error.message}`);
  process.exitCode = 1;
}
