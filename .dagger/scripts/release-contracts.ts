import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";

export const CONTRACT_FILES = [
  "api/openapi.json",
  "api/operations.json",
  "api/generator.lock.json",
  "api/generator/oas3-gen.patch",
  "api/client/src/generated/mod.rs",
  "api/client/src/generated/client.rs",
  "api/client/src/generated/types.rs",
  "crates/nrz-contract/schemas/onreza-rules-v1.schema.json",
  "crates/nrz-contract/schemas/manifest-v1.schema.json",
  "crates/nrz-contract/schemas/runtime-artifact-graph-v2.schema.json",
  "crates/nrz-contract/src/generated.rs",
  "assets/functions-runtime.lock.json",
  "assets/functions-runtime-signing-public.pem",
  "assets/functions-inspector.mjs",
] as const;

/** Hash the exact release inputs, without depending on a server or another checkout. */
export function releaseContracts(root: string) {
  const read = (path: string) => readFileSync(join(root, path));
  const sha256 = (path: string) => createHash("sha256").update(read(path)).digest("hex");
  const generator = JSON.parse(read("api/generator.lock.json").toString());
  if (sha256(generator.patch) !== generator.patchSha256) {
    throw new Error("SDK generator patch does not match its lockfile");
  }
  return {
    sha256: Object.fromEntries(CONTRACT_FILES.map(path => [path, sha256(path)])),
    generator,
    functionsRuntime: JSON.parse(read("assets/functions-runtime.lock.json").toString()),
  };
}

export function verifyReleaseProvenance(
  root: string,
  provenance: unknown,
  expected: { version: string; tag: string; channel: string },
): void {
  if (!provenance || typeof provenance !== "object") {
    throw new Error("Missing release provenance");
  }
  const metadata = provenance as Record<string, unknown>;
  if (metadata.version !== expected.version || metadata.tag !== expected.tag ||
      metadata.channel !== expected.channel || metadata.sourceDirty !== false ||
      typeof metadata.sourceRevision !== "string" || !/^[a-f0-9]{40}$/.test(metadata.sourceRevision) ||
      JSON.stringify(metadata.contracts) !== JSON.stringify(releaseContracts(root))) {
    throw new Error("Release provenance does not match the selected release source and contracts");
  }
}
