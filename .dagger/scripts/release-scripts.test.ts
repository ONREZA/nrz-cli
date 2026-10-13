import * as assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "bun:test";

import {
  artifactChecksums,
  createNpmPackage,
  createPackageJson,
  createPostinstall,
  releaseAssets,
} from "./create-npm-package";
import { REQUIRED_RELEASE_ASSETS } from "./release-assets";
import {
  appendChecksumsToReleaseNotes,
  createChecksums,
  findReleaseByTag,
  finalizeRelease,
  resumeRelease,
  validateReleaseMode,
  verifyResumeChecksums,
  type GitHubRelease,
  verifyPublishedAssets,
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

test("generated npm tarball carries third-party notice bytes with its license and launch scripts", () => {
  const dir = mkdtempSync(join(tmpdir(), "nrz-npm-notices-"));
  try {
    const sourceDir = join(dir, "source");
    const distDir = join(dir, "dist");
    const outDir = join(dir, "npm");
    mkdirSync(sourceDir);
    mkdirSync(distDir);
    const notice = Buffer.from("Fixture third-party notice\nSource: https://example.invalid/source\n", "utf8");
    writeFileSync(join(sourceDir, "THIRD_PARTY_NOTICES"), notice);
    writeFileSync(join(sourceDir, "LICENSE"), "Fixture MIT license\n");
    writeFileSync(join(sourceDir, "npm-README.md"), "Fixture README\n");
    for (const asset of REQUIRED_RELEASE_ASSETS) writeFileSync(join(distDir, asset), asset);
    createNpmPackage({
      version: "0.33.0-beta.1",
      tag: "v0.33.0-beta.1",
      channel: "beta",
      sourceDir,
      distDir,
      outDir,
    });
    const packed = Bun.spawnSync(
      [process.execPath, "pm", "pack", "--ignore-scripts", "--filename", join(dir, "fixture.tgz")],
      { cwd: outDir },
    );
    assert.equal(packed.exitCode, 0, packed.stderr.toString());
    for (const member of ["THIRD_PARTY_NOTICES", "LICENSE", "bin/nrz.js", "scripts/postinstall.js"]) {
      const extracted = Bun.spawnSync(["tar", "-xOf", join(dir, "fixture.tgz"), `package/${member}`]);
      assert.equal(extracted.exitCode, 0, `${member}: ${extracted.stderr.toString()}`);
      assert.deepEqual(
        extracted.stdout,
        member === "THIRD_PARTY_NOTICES" ? notice : readFileSync(join(outDir, member)),
      );
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("release assets are tied to the selected GitHub release tag", () => {
  const assets = releaseAssets("v0.33.0-beta.1");

  assert.deepEqual(
    Object.keys(assets).map((platform) => `nrz-${platform}.tar.gz`),
    REQUIRED_RELEASE_ASSETS,
  );
  assert.equal(
    assets["linux-x64"],
    "https://github.com/ONREZA/nrz-cli/releases/download/v0.33.0-beta.1/nrz-linux-x64.tar.gz",
  );
});

test("npm package uses checksums from the CI-built release artifacts", () => {
  const dir = mkdtempSync(join(tmpdir(), "nrz-npm-artifacts-"));
  try {
    for (const asset of REQUIRED_RELEASE_ASSETS) {
      writeFileSync(join(dir, asset), asset);
    }

    const checksums = artifactChecksums(dir);

    assert.equal(
      checksums["linux-x64"],
      "5634f923b26f33338f294ce3822e370c29c8817d7944898552e053701387fd15",
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
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

test("published release retries require the exact CI-built assets", async () => {
  const dir = mkdtempSync(join(tmpdir(), "nrz-release-retry-"));
  try {
    const archive = join(dir, "nrz-linux-x64.tar.gz");
    const checksums = join(dir, "checksums-sha256.txt");
    writeFileSync(archive, "ci archive");
    writeFileSync(checksums, "ci checksums");

    const release: GitHubRelease = {
      id: 123,
      tag_name: "v0.33.0-beta.1",
      draft: false,
      html_url: "https://github.com/ONREZA/nrz-cli/releases/tag/v0.33.0-beta.1",
      assets: [
        { id: 1, name: "nrz-linux-x64.tar.gz" },
        { id: 2, name: "checksums-sha256.txt" },
      ],
    };
    const published = new Map([
      [1, new TextEncoder().encode("ci archive")],
      [2, new TextEncoder().encode("ci checksums")],
    ]);

    await verifyPublishedAssets(
      release,
      {
        "nrz-linux-x64.tar.gz": archive,
        "checksums-sha256.txt": checksums,
      },
      async (assetId) => published.get(assetId)!,
    );

    published.set(1, new TextEncoder().encode("repacked archive"));
    await assert.rejects(
      verifyPublishedAssets(
        release,
        {
          "nrz-linux-x64.tar.gz": archive,
          "checksums-sha256.txt": checksums,
        },
        async (assetId) => published.get(assetId)!,
      ),
      /does not match the CI-built artifact/,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
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

test("release manifest preserves workspace dependencies and changes only the CLI version", () => {
  const cargo = `
[workspace]
members = ["api/client", "crates/*"]
[workspace.package]
version = "0.1.0"
[package]
name = "nrz"
version = "0.40.2"
[dependencies]
nrz-api = { path = "api/client" }
nrz-contract = { path = "crates/nrz-contract", features = ["validation"] }
nrz-source-publisher = { path = "crates/nrz-source-publisher" }
`;
  const manifest = Bun.TOML.parse(cargo) as { package: { version: string } };
  manifest.package.version = "0.41.0-beta.0";
  assert.deepEqual(Bun.TOML.parse(releaseCargoToml(cargo, "0.41.0-beta.0")), manifest);
});

test("release manifest rejects the wrong package and an inherited CLI version", () => {
  assert.throws(() => releaseCargoToml('[package]\nname = "other"\nversion = "1.0.0"', "2.0.0"), /explicit version/);
  assert.throws(() => releaseCargoToml('[package]\nname = "nrz"\nversion.workspace = true', "2.0.0"), /explicit version/);
});

test("release cargo lock leaves public library versions stable", () => {
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
    'name = "nrz-fn-source"',
    'version = "0.1.0"',
    "",
    "[[package]]",
    'name = "nrz-source-bundle"',
    'version = "0.1.0"',
    "",
  ].join("\n");

  const updated = releaseCargoLock(lock, "0.33.0-beta.0");
  assert.equal(releaseCargoLock(updated, "0.33.0-beta.0"), updated);
  assert.throws(() => releaseCargoLock('[[package]]\nname = "other"\nversion = "1.0.0"\n', "0.33.0-beta.0"), /Could not update/);

  assert.match(updated, /\[\[package\]\]\nname = "nrz"\nversion = "0\.33\.0-beta\.0"/);
  assert.match(updated, /\[\[package\]\]\nname = "nrz-contract"\nversion = "0\.1\.0"/);
  assert.match(updated, /\[\[package\]\]\nname = "nrz-fn-source"\nversion = "0\.1\.0"/);
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

test("holding a verified draft does not finalize it or move GitHub latest", async () => {
  const release: GitHubRelease = { id: 7, tag_name: "v0.46.0", draft: true, html_url: "draft" };
  const mutations: unknown[] = [];
  const result = await finalizeRelease("ONREZA/nrz-cli", release, { prerelease: false, holdDraft: true }, async <T>(_method: string, _path: string, body?: unknown) => {
    mutations.push(body);
    return { ...release, draft: false } as T;
  });
  assert.equal(result.draft, true);
  assert.deepEqual(mutations, []);
});


test("resume verifies original assets before the only publication mutation", async () => {
  const dir = mkdtempSync(join(tmpdir(), "nrz-resume-"));
  try {
    const path = join(dir, "asset.tar.gz");
    writeFileSync(path, "original");
    const draft: GitHubRelease = { id: 8, tag_name: "v0.46.0", draft: true, html_url: "draft", assets: [{ id: 9, name: "asset.tar.gz" }] };
    const mutations: string[] = [];
    const requester = async <T>(method: string) => {
      mutations.push(method);
      return { ...draft, draft: false } as T;
    };
    const expected = { "asset.tar.gz": path };
    const download = async () => new TextEncoder().encode("original");
    await assert.rejects(resumeRelease("ONREZA/nrz-cli", null, expected, false, download, requester), /missing release/);
    await assert.rejects(resumeRelease("ONREZA/nrz-cli", { ...draft, assets: [] }, expected, false, download, requester), /missing published asset/);
    await assert.rejects(resumeRelease("ONREZA/nrz-cli", draft, expected, false, async () => new TextEncoder().encode("changed"), requester), /does not match/);
    assert.deepEqual(mutations, []);
    const published = await resumeRelease("ONREZA/nrz-cli", draft, expected, false, download, requester);
    assert.equal(published.draft, false);
    assert.deepEqual(mutations, ["PATCH"]);
    mutations.length = 0;
    await resumeRelease("ONREZA/nrz-cli", published, expected, false, download, requester);
    assert.deepEqual(mutations, []);
    await assert.rejects(finalizeRelease("ONREZA/nrz-cli", published, { prerelease: false, holdDraft: true }, requester), /already published/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("two phase publication requires exact identities and excludes simultaneous hold/resume", () => {
  const revisions = { releaseRevision: "1".repeat(40), sourceRevision: "2".repeat(40), expectedChecksumsSha256: "3".repeat(64) };
  for (const mode of [{ holdDraft: true, resumeExisting: false }, { holdDraft: false, resumeExisting: true }]) {
    assert.throws(() => validateReleaseMode({ ...mode, expectedChecksumsSha256: "3".repeat(64) }), /exact release and source/);
    assert.doesNotThrow(() => validateReleaseMode({ ...mode, ...revisions }));
  }
  assert.throws(() => validateReleaseMode({ holdDraft: true, resumeExisting: true, ...revisions }), /hold and resume/);
  assert.doesNotThrow(() => validateReleaseMode({ holdDraft: false, resumeExisting: false }));
});

test("workflow input validation rejects combinations before release side effects", () => {
  const script = join(import.meta.dir, "../../.github/scripts/validate-release-mode.sh");
  const check = (selection: Record<string, string>) => Bun.spawnSync(["bash", script], { env: { ...process.env,
    RELEASE_HOLD_DRAFT: "false", RELEASE_RESUME_VERSION: "", RELEASE_RETAG_VERSION: "", RELEASE_EXPLICIT_VERSION: "", RELEASE_EXPECTED_REVISION: "", RELEASE_CHANNEL: "stable", ...selection,
  }, stdout: "pipe", stderr: "pipe" }).exitCode;
  assert.equal(check({ RELEASE_HOLD_DRAFT: "true" }), 0);
  const resume = { RELEASE_RESUME_VERSION: "0.46.0", RELEASE_EXPECTED_REVISION: "1".repeat(40), RELEASE_EXPECTED_CHECKSUMS_SHA256: "2".repeat(64) };
  assert.equal(check(resume), 0);
  for (const selection of [
    { ...resume, RELEASE_EXPECTED_CHECKSUMS_SHA256: "" }, { ...resume, RELEASE_HOLD_DRAFT: "true" }, { ...resume, RELEASE_EXPECTED_REVISION: "" },
    { ...resume, RELEASE_RETAG_VERSION: "0.45.1" }, { ...resume, RELEASE_EXPLICIT_VERSION: "0.46.0" },
    { ...resume, RELEASE_CHANNEL: "beta" },
  ]) assert.notEqual(check(selection), 0);
});

test("serialized release workflow never schedules npm on hold or builds on resume", () => {
  const workflow = Bun.YAML.parse(readFileSync(join(import.meta.dir, "../../.github/workflows/release.yml"), "utf8")) as {
    jobs: Record<string, { if?: string; steps: { name?: string; if?: string }[] }>;
  };
  assert.equal(workflow.jobs.prepare.if, "inputs.npm_tag_version == '' && inputs.resume_version == ''");
  const steps = workflow.jobs.publish.steps;
  for (const name of ["Setup Node.js", "Generate npm package", "Publish npm package"]) {
    assert.equal(steps.find(step => step.name === name)?.if, "inputs.publish_npm && !inputs.hold_draft");
  }
  for (const name of ["Download all artifacts", "Package release archives", "Attest release archives"]) {
    assert.equal(steps.find(step => step.name === name)?.if, "inputs.resume_version == ''");
  }
});


test("resume rejects jointly replaced archives and checksums against the recorded hold digest", () => {
  const dir = mkdtempSync(join(tmpdir(), "nrz-resume-identity-"));
  try {
    const path = join(dir, "checksums-sha256.txt");
    const original = "original checksums\n";
    const digest = createHash("sha256").update(original).digest("hex");
    writeFileSync(path, original);
    assert.doesNotThrow(() => verifyResumeChecksums(path, original, digest));
    const changed = "changed archives and matching new checksums\n";
    writeFileSync(path, changed);
    assert.throws(() => verifyResumeChecksums(path, changed, digest), /recorded hold digest/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
