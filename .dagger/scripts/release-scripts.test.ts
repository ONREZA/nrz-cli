import * as assert from "node:assert/strict";
import { test } from "bun:test";

import { createPackageJson, createPostinstall, releaseAssets } from "./create-npm-package";
import {
  filterReleaseCommits,
  releaseCargoToml,
  releaseVendorCargoToml,
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

test("postinstall fails when no binary exists for the host platform", () => {
  const script = createPostinstall({
    assets: releaseAssets("v0.33.0-beta.1"),
    version: "0.33.0-beta.1",
  });

  assert.match(script, /No binary available for/);
  assert.match(script, /process\.exit\(1\);/);
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
    '',
  ].join("\n");

  const updated = releaseCargoToml(cargo, "0.33.0-beta.0");

  assert.match(updated, /^version = "0\.33\.0-beta\.0"$/m);
  assert.match(updated, /^nrz-contract = \{ path = "vendor\/onreza-crates\/nrz-contract" \}$/m);
  assert.match(updated, /^nrz-fn-policy = \{ path = "vendor\/onreza-crates\/nrz-fn-policy" \}$/m);
  assert.doesNotMatch(updated, /\.\.\/deployment/);
});

test("release vendored crate manifests follow the CLI version", () => {
  const cargo = [
    '[package]',
    'name = "nrz-contract"',
    'version = "0.0.0"',
    'edition = "2024"',
    '',
  ].join("\n");

  const updated = releaseVendorCargoToml(cargo, "0.33.0-beta.0");

  assert.match(updated, /^version = "0\.33\.0-beta\.0"$/m);
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
        explicitVersion: "0.33.0-rc.0",
        tags: [],
      }),
    /belongs to channel rc, not beta/,
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
