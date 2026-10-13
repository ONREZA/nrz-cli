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
    for (const path of ["api/openapi.json", "crates/nrz-contract/schemas/manifest-v2.schema.json"] as const) {
      const changed = "{\"contract\":\"changed fixture\"}\n";
      writeFileSync(join(root, path), changed);
      const next = releaseContracts(root);
      expect(next.sha256[path]).toBe(createHash("sha256").update(changed).digest("hex"));
      expect(next.sha256[path]).not.toBe(original.sha256[path]);
      expect(next.functionsRuntime).toEqual(original.functionsRuntime);
      const expected = { version: "0.46.0", tag: "v0.46.0", channel: "stable" };
      expect(() => verifyReleaseProvenance(root, {
        ...expected, sourceDirty: false, sourceRevision: "1".repeat(40), contracts: original,
      }, expected)).toThrow("does not match");
      writeFileSync(join(root, path), readFileSync(join(source, path)));
    }
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


test("held provenance binds the release commit and its original source, retained metadata stays readable", () => {
  const root = resolve(import.meta.dir, "../..");
  const expected = { version: "0.46.0", tag: "v0.46.0", channel: "stable" };
  const metadata = { ...expected, sourceDirty: false, sourceRevision: "1".repeat(40), contracts: releaseContracts(root) };
  expect(() => verifyReleaseProvenance(root, metadata, expected)).not.toThrow();
  const identity = { ...expected, releaseRevision: "2".repeat(40), sourceRevision: metadata.sourceRevision };
  expect(() => verifyReleaseProvenance(root, metadata, identity)).toThrow("does not match");
  const held = { ...metadata, releaseRevision: identity.releaseRevision };
  expect(() => verifyReleaseProvenance(root, held, identity)).not.toThrow();
  for (const change of [{ releaseRevision: "3".repeat(40) }, { sourceRevision: "4".repeat(40) }]) {
    expect(() => verifyReleaseProvenance(root, { ...held, ...change }, identity)).toThrow("does not match");
  }
});
