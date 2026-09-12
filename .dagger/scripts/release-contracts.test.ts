import { expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { CONTRACT_FILES, releaseContracts, verifyReleaseProvenance } from "./release-contracts";

test("release provenance binds the exact selected contract inputs", () => {
  const root = mkdtempSync(join(tmpdir(), "nrz-contract-provenance-"));
  try {
    const source = resolve(import.meta.dir, "../..");
    for (const file of CONTRACT_FILES) {
      mkdirSync(dirname(join(root, file)), { recursive: true });
      writeFileSync(join(root, file), readFileSync(join(source, file)));
    }
    const original = releaseContracts(root);
    expect(releaseContracts(root)).toEqual(original);
    const changed = "{\"openapi\":\"changed fixture\"}\n";
    writeFileSync(join(root, "api/openapi.json"), changed);
    const next = releaseContracts(root);
    expect(next.sha256["api/openapi.json"]).toBe(createHash("sha256").update(changed).digest("hex"));
    expect(next.sha256["api/openapi.json"]).not.toBe(original.sha256["api/openapi.json"]);
    expect(next.functionsRuntime).toEqual(original.functionsRuntime);
    writeFileSync(join(root, "api/generator/oas3-gen.patch"), "different patch");
    expect(() => releaseContracts(root)).toThrow("does not match its lockfile");
    rmSync(join(root, "api/generator/oas3-gen.patch"));
    expect(() => releaseContracts(root)).toThrow();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("publishing refuses provenance from another release or modified contracts", () => {
  const root = resolve(import.meta.dir, "../..");
  const expected = { version: "0.41.0-beta.0", tag: "v0.41.0-beta.0", channel: "beta" };
  const metadata = {
    ...expected, sourceDirty: false, sourceRevision: "1".repeat(40), contracts: releaseContracts(root),
  };
  expect(() => verifyReleaseProvenance(root, metadata, expected)).not.toThrow();
  for (const change of [
    { version: "0.40.2" }, { tag: "v0.40.2" }, { channel: "stable" },
    { sourceDirty: true }, { sourceRevision: "unknown" }, { contracts: {} },
  ]) {
    expect(() => verifyReleaseProvenance(root, { ...metadata, ...change }, expected)).toThrow("does not match");
  }
  expect(() => verifyReleaseProvenance(root, null, expected)).toThrow("Missing");
});
