import { argument, dag, type Directory, type File, func, object, type Secret } from "@dagger.io/dagger";

import {
  RELEASE_PLATFORMS,
  type ReleasePlatform,
  releaseAssetName,
} from "../scripts/release-assets";

const RUST_IMAGE = "rust:1.97-bookworm";
const BUN_IMAGE = "oven/bun:1.3.14-debian";
const ALPINE_IMAGE = "alpine:3.20";
const RELEASE_GIT_METADATA_SCRIPT = ".dagger/scripts/capture-git-metadata.ts";
const CLI_CRATES_VENDOR_DIR = "vendor/onreza-crates";
const REQUIRED_CLI_CRATES = [
  "nrz-contract",
  "nrz-fn-source",
  "nrz-source-bundle",
  "nrz-runtime-artifact",
  "nrz-source-publisher",
] as const;

const PLATFORMS = new Set<string>(RELEASE_PLATFORMS);
const CHANNELS = new Set(["stable", "beta"]);

function requirePlatform(platform: string): asserts platform is ReleasePlatform {
  if (!PLATFORMS.has(platform)) {
    throw new Error(`Unsupported platform: ${platform}`);
  }
}

function requireChannel(channel: string): void {
  if (!CHANNELS.has(channel)) {
    throw new Error(`Unsupported release channel: ${channel}`);
  }
}

function binName(platform: ReleasePlatform): string {
  return platform === "win32-x64" ? "nrz.exe" : "nrz";
}

function rustContainer(source: Directory) {
  return dag
    .container()
    .from(RUST_IMAGE)
    .withMountedCache("/usr/local/cargo/registry", dag.cacheVolume("nrz-cargo-registry"))
    .withMountedCache("/usr/local/cargo/git", dag.cacheVolume("nrz-cargo-git"))
    .withMountedCache("/work/target", dag.cacheVolume("nrz-cargo-target"))
    .withDirectory("/work", source)
    .withWorkdir("/work")
    .withEnvVariable("CARGO_TERM_COLOR", "always");
}

function bunContainer(source: Directory) {
  return dag.container().from(BUN_IMAGE).withDirectory("/work", source).withWorkdir("/work");
}

function bunDevContainer(source: Directory) {
  return bunContainer(source)
    .withMountedCache("/root/.bun/install/cache", dag.cacheVolume("nrz-bun-install-cache"))
    .withExec(["sh", "-ceu", "cd .dagger && bun install --frozen-lockfile"]);
}

function bunContainerWithGit(source: Directory) {
  return bunContainer(source)
    .withExec([
      "sh",
      "-ceu",
      [
        "apt-get update",
        "apt-get install -y --no-install-recommends git ca-certificates",
        "rm -rf /var/lib/apt/lists/*",
      ].join("\n"),
    ])
    .withExec(["git", "config", "--global", "--add", "safe.directory", "/work"]);
}

function sourceWithReleaseGitMetadata(source: Directory): Directory {
  const sourceWithoutReleaseState = source.withoutDirectory(".nrz-release");
  const sourceWithGit = sourceWithoutReleaseState.withDirectory(
    ".git",
    dag.currentWorkspace().directory(".git", { gitignore: false }),
  );
  const metadata = bunContainerWithGit(sourceWithGit)
    .withExec(["bun", RELEASE_GIT_METADATA_SCRIPT])
    .file("/work/.nrz-release/git.json");

  return sourceWithoutReleaseState.withFile(".nrz-release/git.json", metadata);
}

@object()
export class NrzCli {
  /**
   * Run the Rust CI checks used by the GitHub CI workflow.
   */
  @func()
  async ci(
    @argument({
      ignore: [
        "target",
        "**/target",
        "node_modules",
        "**/node_modules",
        "dist",
        "dist-archive",
        ".dagger/sdk",
        ".dagger/node_modules",
        ".nrz-release",
        ".env",
        ".env.*",
      ],
    })
    source: Directory,
  ): Promise<string> {
    await bunDevContainer(source)
      .withExec(["sh", "-ceu", "cd .dagger && bun run typecheck && bun test scripts/release-scripts.test.ts"])
      .sync();

    let ctr = rustContainer(source);
    ctr = ctr.withExec([
      "sh",
      "-ceu",
      [
        "apt-get update",
        "apt-get install -y --no-install-recommends nodejs",
        "rm -rf /var/lib/apt/lists/*",
      ].join("\n"),
    ]);
    ctr = ctr.withExec(["rustup", "component", "add", "rustfmt", "clippy"]);
    ctr = ctr.withExec(["cargo", "test", "--all"]);
    ctr = ctr.withExec(["cargo", "fmt", "--", "--check"]);
    ctr = ctr.withExec(["cargo", "clippy", "--", "-D", "warnings"]);
    await ctr.sync();
    return "CI checks passed";
  }

  /**
   * Calculate release metadata without changing source files.
   */
  @func()
  async releaseMetadata(
    @argument({
      ignore: [
        "target",
        "**/target",
        "node_modules",
        "**/node_modules",
        "dist",
        "dist-archive",
        ".dagger/sdk",
        ".dagger/node_modules",
        ".nrz-release",
        ".env",
        ".env.*",
      ],
    })
    source: Directory,
    /** Release channel: stable or beta. */
    channel = "stable",
    /** Optional explicit version. Accepts 1.2.3, v1.2.3, or full prerelease versions. */
    version = "",
    /** Version bump when version is not provided: auto, patch, minor, or major. */
    bump = "auto",
  ): Promise<string> {
    requireChannel(channel);
    const releaseSource = sourceWithReleaseGitMetadata(source);
    return bunContainer(releaseSource)
      .withEnvVariable("NRZ_RELEASE_CHANNEL", channel)
      .withEnvVariable("NRZ_RELEASE_VERSION", version)
      .withEnvVariable("NRZ_RELEASE_BUMP", bump)
      .withExec(["bun", ".dagger/scripts/release-plan.ts"])
      .stdout();
  }

  /**
   * Apply release metadata to Cargo/package versions and CHANGELOG.md.
   */
  @func()
  async prepareRelease(
    @argument({
      ignore: [
        "target",
        "**/target",
        "node_modules",
        "**/node_modules",
        "dist",
        "dist-archive",
        ".dagger/sdk",
        ".dagger/node_modules",
        ".nrz-release",
        ".env",
        ".env.*",
      ],
    })
    source: Directory,
    /** Release channel: stable or beta. */
    channel = "stable",
    /** Optional explicit version. Accepts 1.2.3, v1.2.3, or full prerelease versions. */
    version = "",
    /** Version bump when version is not provided: auto, patch, minor, or major. */
    bump = "auto",
  ): Promise<Directory> {
    requireChannel(channel);
    const releaseSource = sourceWithReleaseGitMetadata(source);
    const requiredCrateChecks = REQUIRED_CLI_CRATES.map(
      (crate) => `test -f ${CLI_CRATES_VENDOR_DIR}/${crate}/Cargo.toml`,
    ).join(" && ");
    const releaseDir = bunContainer(releaseSource)
      .withExec([
        "sh",
        "-ceu",
        [
          `if ! ${requiredCrateChecks}; then`,
          "  echo 'prepare-release requires vendor/onreza-crates from deployment scripts/sync-nrz-cli-crates.ts' >&2",
          "  exit 1",
          "fi",
        ].join("\n"),
      ])
      .withEnvVariable("NRZ_RELEASE_CHANNEL", channel)
      .withEnvVariable("NRZ_RELEASE_VERSION", version)
      .withEnvVariable("NRZ_RELEASE_BUMP", bump)
      .withExec(["bun", ".dagger/scripts/release-plan.ts", "--write"])
      .directory("/work");

    return rustContainer(releaseDir)
      .withExec(["cargo", "metadata", "--locked", "--format-version", "1"])
      .directory("/work")
      .withoutDirectory(".git")
      .withoutDirectory("target")
      .withoutDirectory("node_modules")
      .withoutDirectory("dist")
      .withoutDirectory("dist-archive");
  }

  /**
   * Package a native nrz binary into the release archive contract.
   */
  @func()
  packagePlatform(binary: File, platform: string): File {
    requirePlatform(platform);
    const executable = binName(platform);
    const mode = platform === "win32-x64" ? "0644" : "0755";
    return dag
      .container()
      .from(ALPINE_IMAGE)
      .withMountedFile(`/input/${executable}`, binary)
      .withExec([
        "sh",
        "-ceu",
        [
          "set -o pipefail",
          `mkdir -p /out/archive/${platform}`,
          `cp /input/${executable} /out/archive/${platform}/${executable}`,
          `chmod ${mode} /out/archive/${platform}/${executable}`,
          `find /out/archive/${platform} -exec touch -h -d @0 {} +`,
          `tar --numeric-owner -cf - -C /out/archive ${platform} | gzip -c > /out/${releaseAssetName(platform)}`,
        ].join("\n"),
      ])
      .file(`/out/${releaseAssetName(platform)}`);
  }

  /**
   * Package all native build artifacts downloaded from GitHub Actions.
   */
  @func()
  packageReleaseArtifacts(binaries: Directory): Directory {
    return dag
      .container()
      .from(ALPINE_IMAGE)
      .withDirectory("/input", binaries)
      .withExec([
        "sh",
        "-ceu",
        [
          "set -o pipefail",
          `for platform in ${RELEASE_PLATFORMS.join(" ")}; do`,
          "  binary=nrz",
          "  mode=0755",
          "  if [ \"$platform\" = \"win32-x64\" ]; then binary=nrz.exe; mode=0644; fi",
          "  src=\"$(find /input -type f -name \"$binary\" -path \"*/nrz-bin-$platform/*\" | head -n 1)\"",
          "  if [ -z \"$src\" ]; then echo \"missing binary for $platform\" >&2; exit 1; fi",
          "  mkdir -p \"/out/archive/$platform\"",
          "  cp \"$src\" \"/out/archive/$platform/$binary\"",
          "  chmod \"$mode\" \"/out/archive/$platform/$binary\"",
          "  find \"/out/archive/$platform\" -exec touch -h -d @0 {} +",
          "  tar --numeric-owner -cf - -C /out/archive \"$platform\" | gzip -c > \"/out/nrz-$platform.tar.gz\"",
          "done",
          "rm -rf /out/archive",
        ].join("\n"),
      ])
      .directory("/out");
  }

  /**
   * Create a complete npm package directory for the already prepared release.
   */
  @func()
  npmPackage(
    @argument({
      ignore: [
        "target",
        "**/target",
        "node_modules",
        "**/node_modules",
        "dist",
        "dist-archive",
        ".dagger/sdk",
        ".dagger/node_modules",
        ".git",
        ".nrz-release",
        ".env",
        ".env.*",
      ],
    })
    source: Directory,
    artifacts: Directory,
    version: string,
    tag: string,
    channel = "stable",
  ): Directory {
    requireChannel(channel);
    return bunContainer(source)
      .withDirectory("/dist", artifacts)
      .withEnvVariable("NRZ_RELEASE_VERSION", version)
      .withEnvVariable("NRZ_RELEASE_TAG", tag)
      .withEnvVariable("NRZ_RELEASE_CHANNEL", channel)
      .withExec(["bun", ".dagger/scripts/create-npm-package.ts"])
      .directory("/out/npm");
  }

  /**
   * Publish GitHub release assets through a draft release and finalize it only after validation.
   */
  @func()
  async publishGithubRelease(
    @argument({
      ignore: [
        "target",
        "**/target",
        "node_modules",
        "**/node_modules",
        ".dagger/sdk",
        ".dagger/node_modules",
        ".git",
        ".nrz-release",
        ".env",
        ".env.*",
      ],
    })
    source: Directory,
    artifacts: Directory,
    githubToken: Secret,
    version: string,
    tag: string,
    channel = "stable",
    repository = "ONREZA/nrz-cli",
  ): Promise<string> {
    requireChannel(channel);
    return bunContainer(source)
      .withDirectory("/dist", artifacts)
      .withSecretVariable("GITHUB_TOKEN", githubToken)
      .withEnvVariable("GITHUB_REPOSITORY", repository)
      .withEnvVariable("NRZ_RELEASE_VERSION", version)
      .withEnvVariable("NRZ_RELEASE_TAG", tag)
      .withEnvVariable("NRZ_RELEASE_CHANNEL", channel)
      .withExec(["bun", ".dagger/scripts/publish-github-release.ts"])
      .stdout();
  }
}
