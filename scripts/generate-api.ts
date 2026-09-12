#!/usr/bin/env bun
import { cp, mkdir, mkdtemp, readFile, rename, rm } from "node:fs/promises";
import { resolve } from "node:path";
import { selectOperations } from "./api-contract";
import { generatorBinary, runTool } from "./generator-toolchain";

const ROOT = resolve(import.meta.dir, "..");
const CONTRACT = resolve(ROOT, "api/openapi.json");
const OUTPUT = resolve(ROOT, "api/client/src/generated");
const args = process.argv.slice(2);
let source = CONTRACT;
let check = false;
for (let index = 0; index < args.length; index++) {
  if (args[index] === "--check") check = true;
  else if (args[index] === "--from" && args[index + 1]) source = resolve(args[++index]!);
  else throw new Error(`Unknown or incomplete argument: ${args[index]}`);
}

const document = selectOperations(
  JSON.parse(await readFile(source, "utf8")),
  JSON.parse(await readFile(resolve(ROOT, "api/operations.json"), "utf8")),
);
const generator = await generatorBinary(ROOT);
await mkdir(resolve(ROOT, ".cache"), { recursive: true });
const temporary = await mkdtemp(resolve(ROOT, ".cache/api-generation-"));
try {
  // Stable relative input path makes generated headers independent of checkout/cache paths.
  const input = resolve(temporary, "api/openapi.json");
  const output = resolve(temporary, "generated");
  await Bun.write(input, `${JSON.stringify(document, null, 2)}\n`);
  await runTool([
    generator, "generate", "client-mod", "--input", "api/openapi.json", "--output", "generated",
    "--api-name", "PlatformClient", "--enum-mode", "preserve", "--no-ordered-collections",
    "--bidirectional-serde",
  ], temporary);
  await runTool(["rustfmt", "--edition", "2024", resolve(output, "mod.rs")], ROOT);
  if (check) {
    await runTool(["diff", "-u", CONTRACT, input], ROOT);
    await runTool(["diff", "-ruN", OUTPUT, output], ROOT);
  } else {
    const previous = resolve(temporary, "previous");
    const exists = await Bun.file(resolve(OUTPUT, "mod.rs")).exists();
    if (exists) await rename(OUTPUT, previous);
    try {
      await rename(output, OUTPUT);
    } catch (error) {
      if (exists) await rename(previous, OUTPUT);
      throw error;
    }
    await cp(input, CONTRACT);
  }
} finally {
  await rm(temporary, { recursive: true, force: true });
}
