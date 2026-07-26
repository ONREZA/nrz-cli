import * as assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "bun:test";

import { createPackageJson, createPostinstall, releaseAssets } from "./create-npm-package";
import {
  appendChecksumsToReleaseNotes,
  createChecksums,
  findReleaseByTag,
  type GitHubRelease,
} from "./publish-github-release";
import {
  filterReleaseCommits,
  releaseCargoLock,
  releaseCargoToml,
  resolveVersion,
  type ReleaseCommit,
} from "./release-plan";

const fixCommit: ReleaseCommit = {
  hash: "1111111111111111111111111111111111111111",
  shortHash: "1111111",
  subject: "fix(cli): resolve issue",
  type: "fix",
  scope: "cli",
  text: "resolve issue",
  breaking: false,
};

const featureCommit: ReleaseCommit = {
  hash: "2222222222222222222222222222222222222222",
  shortHash: "2222222",
  subject: "feat(upgrade): add channel",
  type: "feat",
  scope: "upgrade",
  text: "add channel",
  breaking: false,
};

const breakingCommit: ReleaseCommit = {
  hash: "4444444444444444444444444444444444444444",
  shortHash: "4444444",
  subject: "feat(api)!: change contract",
  type: "feat",
  scope: "api",
  text: "change contract",
  breaking: true,
};

test("generated npm package declares ESM for generated .js scripts", () => {
  const packageJson = createPackageJson({ version: "0.33.0-beta.1", channel: "beta" });

  assert.equal(packageJson.type, "module");
  assert.equal(packageJson.scripts.postinstall, "node scripts/postinstall.js");
  assert.equal(packageJson.bin.nrz, "bin/nrz.js");
  assert.equal(packageJson.publishConfig.tag, "beta");
});

test("release assets are tied to the selected GitHub release tag", () => {
  assert.equal(
    releaseAssets("v0.33.0-beta.1")["linux-x64"],
    "https://github.com/ONREZA/nrz-cli/releases/download/v0.33.0-beta.1/nrz-linux-x64.tar.gz",
  );
});

test("GitHub release lookup includes draft releases by tag", async () => {
  const draft: GitHubRelease = {
    id: 123,
    tag_name: "v0.33.0-beta.1",
    draft: true,
    html_url: "https://github.com/ONREZA/nrz-cli/releases/tag/untagged-draft",
    assets: [],
  };
  const calls: string[] = [];

  const release = await findReleaseByTag("ONREZA/nrz-cli", "v0.33.0-beta.1", async <T>(
    method: string,
    path: string,
  ): Promise<T | null> => {
    calls.push(`${method} ${path}`);
    if (path.endsWith("/releases/tags/v0.33.0-beta.1")) {
      return null;
    }
    return [draft] as T;
  });

  assert.equal(release?.id, draft.id);
  assert.deepEqual(calls, [
    "GET /repos/ONREZA/nrz-cli/releases/tags/v0.33.0-beta.1",
    "GET /repos/ONREZA/nrz-cli/releases?per_page=100",
  ]);
});

test("GitHub release notes include the same checksum text as the uploaded checksum file", () => {
  const dir = mkdtempSync(join(tmpdir(), "nrz-release-"));
  try {
    const asset = join(dir, "nrz-linux-x64.tar.gz");
    writeFileSync(asset, "binary");

    const checksums = createChecksums([asset]);
    const notes = appendChecksumsToReleaseNotes("## [0.33.0-beta.2]\n\n- Add ONREZA Functions", checksums.text);

    assert.equal(readFileSync(checksums.path, "utf8"), checksums.text);
    assert.match(checksums.text, /^[a-f0-9]{64}  nrz-linux-x64\.tar\.gz\n$/);
    assert.equal(
      notes,
      `## [0.33.0-beta.2]\n\n- Add ONREZA Functions\n\n### Checksums (SHA-256)\n\n\`\`\`text\n${checksums.text.trim()}\n\`\`\``,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("postinstall fails when no binary exists for the host platform", () => {
  const script = createPostinstall({
    assets: releaseAssets("v0.33.0-beta.1"),
    checksums: {
      "linux-x64": "0".repeat(64),
      "darwin-x64": "1".repeat(64),
      "darwin-arm64": "2".repeat(64),
      "win32-x64": "3".repeat(64),
    },
    version: "0.33.0-beta.1",
  });

  assert.match(script, /No binary available for/);
  assert.match(script, /SHA-256 mismatch/);
  assert.match(script, /createHash\("sha256"\)/);
  assert.match(script, /Archive did not contain/);
  assert.match(script, /mkdtempSync\(join\(binDir, "\.nrz-install-"\)\)/);
  assert.match(script, /tar\.extract\(\{ cwd: stagingDir, strip: 1 \}\)/);
  assert.match(script, /replaceInstalledBinary\(sourcePath, targetPath, backupPath\)/);
  assert.match(script, /previous binary was restored/);
  assert.match(script, /process\.exitCode = 1/);
  assert.match(script, /rmSync\(stagingDir, \{ recursive: true, force: true \}\)/);
  assert.doesNotThrow(() => new Bun.Transpiler({ loader: "js" }).transformSync(script));
});

test("release cargo manifest points nrz crates at sanitized vendor snapshot", () => {
  const cargo = [
    '[package]',
    'name = "nrz"',
    'version = "0.32.4"',
    '',
    '[dependencies]',
    'nrz-contract = { version = "0.1", path = "../deployment/crates/nrz-contract" }',
    'nrz-fn-policy = { version = "0.1", path = "../deployment/crates/nrz-fn-policy" }',
    'nrz-source-bundle = { version = "0.1", path = "../deployment/crates/nrz-source-bundle" }',
    '',
  ].join("\n");

  const updated = releaseCargoToml(cargo, "0.33.0-beta.0");

  assert.match(updated, /^version = "0\.33\.0-beta\.0"$/m);
  assert.match(updated, /^nrz-contract = \{ path = "vendor\/onreza-crates\/nrz-contract" \}$/m);
  assert.match(updated, /^nrz-fn-policy = \{ path = "vendor\/onreza-crates\/nrz-fn-policy" \}$/m);
  assert.match(updated, /^nrz-source-bundle = \{ path = "vendor\/onreza-crates\/nrz-source-bundle" \}$/m);
  assert.doesNotMatch(updated, /\.\.\/deployment/);
});

test("release cargo manifest accepts already sanitized nrz crate dependencies", () => {
  const cargo = [
    '[package]',
    'name = "nrz"',
    'version = "0.32.4"',
    '',
    '[dependencies]',
    'nrz-contract = { path = "vendor/onreza-crates/nrz-contract" }',
    'nrz-fn-policy = { path = "vendor/onreza-crates/nrz-fn-policy" }',
    'nrz-source-bundle = { path = "vendor/onreza-crates/nrz-source-bundle" }',
    '',
  ].join("\n");

  const updated = releaseCargoToml(cargo, "0.33.0-beta.0");

  assert.match(updated, /^version = "0\.33\.0-beta\.0"$/m);
  assert.match(updated, /^nrz-contract = \{ path = "vendor\/onreza-crates\/nrz-contract" \}$/m);
  assert.match(updated, /^nrz-fn-policy = \{ path = "vendor\/onreza-crates\/nrz-fn-policy" \}$/m);
  assert.match(updated, /^nrz-source-bundle = \{ path = "vendor\/onreza-crates\/nrz-source-bundle" \}$/m);
});

test("release cargo lock leaves vendored crate versions stable", () => {
  const lock = [
    "[[package]]",
    'name = "nrz"',
    'version = "0.32.4"',
    "",
    "[[package]]",
    'name = "nrz-contract"',
    'version = "0.1.0"',
    "",
    "[[package]]",
    'name = "nrz-fn-policy"',
    'version = "0.1.0"',
    "",
    "[[package]]",
    'name = "nrz-source-bundle"',
    'version = "0.1.0"',
    "",
  ].join("\n");

  const updated = releaseCargoLock(lock, "0.33.0-beta.0");

  assert.match(updated, /\[\[package\]\]\nname = "nrz"\nversion = "0\.33\.0-beta\.0"/);
  assert.match(updated, /\[\[package\]\]\nname = "nrz-contract"\nversion = "0\.1\.0"/);
  assert.match(updated, /\[\[package\]\]\nname = "nrz-fn-policy"\nversion = "0\.1\.0"/);
  assert.match(updated, /\[\[package\]\]\nname = "nrz-source-bundle"\nversion = "0\.1\.0"/);
});

test("stable auto release promotes an active prerelease train without another bump", () => {
  assert.equal(
    resolveVersion("0.33.0-beta.0", [featureCommit], {
      channel: "stable",
      bumpInput: "auto",
      tags: ["v0.33.0-beta.0"],
    }),
    "0.33.0",
  );
});

test("auto breaking change before 1.0 bumps minor, not major", () => {
  assert.equal(
    resolveVersion("0.32.4", [breakingCommit], {
      channel: "beta",
      bumpInput: "auto",
      tags: [],
    }),
    "0.33.0-beta.0",
  );
});

test("auto breaking change after 1.0 bumps major", () => {
  assert.equal(
    resolveVersion("1.2.3", [breakingCommit], {
      channel: "stable",
      bumpInput: "auto",
      tags: [],
    }),
    "2.0.0",
  );
});

test("next prerelease continues the active release train", () => {
  assert.equal(
    resolveVersion("0.33.0-beta.0", [featureCommit], {
      channel: "beta",
      bumpInput: "auto",
      tags: ["v0.33.0-beta.0"],
    }),
    "0.33.0-beta.1",
  );
});

test("explicit bump can open a new prerelease train", () => {
  assert.equal(
    resolveVersion("0.33.0-beta.0", [fixCommit], {
      channel: "beta",
      bumpInput: "minor",
      tags: ["v0.33.0-beta.0"],
    }),
    "0.34.0-beta.0",
  );
});

test("commit-derived release plan requires changes", () => {
  assert.throws(
    () =>
      resolveVersion("0.32.4", [], {
        channel: "stable",
        bumpInput: "auto",
        tags: [],
      }),
    /Cannot derive a release version without changes/,
  );
});

test("explicit version can force an empty release plan", () => {
  assert.equal(
    resolveVersion("0.32.4", [], {
      channel: "stable",
      bumpInput: "auto",
      explicitVersion: "0.32.5",
      tags: [],
    }),
    "0.32.5",
  );
});

test("explicit prerelease version must match the selected channel", () => {
  assert.throws(
    () =>
      resolveVersion("0.32.4", [fixCommit], {
        channel: "beta",
        bumpInput: "auto",
        explicitVersion: "0.33.0-alpha.0",
        tags: [],
      }),
    /belongs to channel alpha, not beta/,
  );
});

test("generated release commits are excluded before changelog and bump planning", () => {
  const commits: ReleaseCommit[] = [
    {
      ...fixCommit,
      hash: "3333333333333333333333333333333333333333",
      shortHash: "3333333",
      subject: "chore(release): v0.33.0-beta.0",
    },
    featureCommit,
  ];

  assert.deepEqual(
    filterReleaseCommits(commits).map((commit) => commit.subject),
    ["feat(upgrade): add channel"],
  );
});
