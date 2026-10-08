import { argument, dag, type Directory, type File, func, object, type Secret } from "@dagger.io/dagger";

import {
  RELEASE_PLATFORMS,
  type ReleasePlatform,
  releaseAssetName,
} from "../scripts/release-assets";

const ALPINE_IMAGE = "alpine:3.20";
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

async function toolVersion(source: Directory, tool: "rust" | "bun"): Promise<string> {
  const pins = Bun.TOML.parse(await source.file(".prototools").contents()) as Record<string, unknown>;
  const version = pins[tool];
  if (typeof version !== "string" || !/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(`.prototools must pin an exact ${tool} version`);
  }
  return version;
}

async function rustContainer(source: Directory) {
  return dag
    .container()
    .from(`rust:${await toolVersion(source, "rust")}-bookworm`)
    .withMountedCache("/usr/local/cargo/registry", dag.cacheVolume("nrz-cargo-registry"))
    .withMountedCache("/usr/local/cargo/git", dag.cacheVolume("nrz-cargo-git"))
    .withMountedCache("/work/target", dag.cacheVolume("nrz-cargo-target"))
    .withDirectory("/work", source)
    .withWorkdir("/work")
    .withEnvVariable("CARGO_BUILD_JOBS", "2")
    .withEnvVariable("RUST_TEST_THREADS", "4")
    .withEnvVariable("CARGO_TERM_COLOR", "always");
}

async function bunContainer(source: Directory) {
  return dag.container()
    .from(`oven/bun:${await toolVersion(source, "bun")}-debian`)
    .withDirectory("/work", source)
    .withWorkdir("/work");
}

async function bunDevContainer(source: Directory) {
  return (await bunContainer(source))
    .withExec(["sh", "-ceu", "apt-get update && apt-get install -y --no-install-recommends git && rm -rf /var/lib/apt/lists/*"])
    .withMountedCache("/root/.bun/install/cache", dag.cacheVolume("nrz-bun-install-cache"))
    .withExec(["sh", "-ceu", "cd .dagger && bun install --frozen-lockfile"]);
}

function sourceWithReleaseGitMetadata(source: Directory, gitMetadata: File): Directory {
  return source.withoutDirectory(".nrz-release").withFile(".nrz-release/git.json", gitMetadata);
}

@object()
export class NrzCli {
  /** Verify generated HTTP and artifact models against their committed inputs. */
  @func()
  async contracts(
    @argument({
      ignore: [
        ".cache", ".moon/cache", ".jscpd", "mutants.out", "mutants.out.old",
        "target", "**/target", "node_modules", "**/node_modules",
        "dist", "dist-archive", ".dagger/sdk", ".nrz-release", ".env", ".env.*",
      ],
    })
    source: Directory,
  ): Promise<string> {
    await (await rustContainer(source))
      .withFile("/usr/local/bin/bun", (await bunContainer(source)).file("/usr/local/bin/bun"))
      .withMountedCache("/work/.cache/tools/oas3-gen", dag.cacheVolume("nrz-oas3-generator"))
      .withExec(["rustup", "component", "add", "rustfmt"])
      .withExec(["bun", "test", "scripts"])
      .withExec(["bun", "scripts/generate-api.ts", "--check"])
      .withExec(["cargo", "run", "--locked", "-p", "nrz-contract", "--features", "codegen", "--bin", "gen-contract", "--", "--check"])
      .sync();
    return "Generated contracts match committed inputs";
  }

  /**
   * Run the containerized Rust and contract checks. GitHub quality gates use Moon.
   */
  @func()
  async ci(
    @argument({
      ignore: [
        ".cache",
        ".moon/cache",
        ".jscpd",
        "mutants.out",
        "mutants.out.old",
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
    await this.contracts(source);
    await (await bunDevContainer(source))
      .withExec(["sh", "-ceu", "cd .dagger && bun run typecheck && bun test scripts"])
      .sync();

    const moonToolchains = Bun.YAML.parse(await source.file(".moon/toolchains.yml").contents()) as {
      rust?: { bins?: string[] };
    };
    const cargoDenyVersion = moonToolchains.rust?.bins?.find((entry) => entry.startsWith("cargo-deny@"))?.match(/^cargo-deny@(\d+\.\d+\.\d+)$/)?.[1];
    if (typeof cargoDenyVersion !== "string" || !/^\d+\.\d+\.\d+$/.test(cargoDenyVersion)) {
      throw new Error(".moon/toolchains.yml must pin an exact cargo-deny version in rust.bins");
    }
    const cargoDeny = dag.container().from(ALPINE_IMAGE)
      .withExec(["apk", "add", "--no-cache", "curl"])
      .withEnvVariable("CARGO_DENY_VERSION", cargoDenyVersion)
      .withExec([
        "sh", "-ceu",
        [
          'arch="$(uname -m)"',
          'case "$arch" in x86_64|aarch64) ;; *) echo "unsupported cargo-deny architecture: $arch" >&2; exit 1;; esac',
          'archive="cargo-deny-$CARGO_DENY_VERSION-$arch-unknown-linux-musl.tar.gz"',
          'url="https://github.com/EmbarkStudios/cargo-deny/releases/download/$CARGO_DENY_VERSION/$archive"',
          "mkdir -p /tmp/cargo-deny /out",
          "cd /tmp/cargo-deny",
          'curl -fsSL "$url" -o "$archive"',
          'curl -fsSL "$url.sha256" -o "$archive.sha256"',
          'printf "%s  %s\\n" "$(cat "$archive.sha256")" "$archive" | sha256sum -c -',
          'tar -xzf "$archive" --strip-components=1 -C /out',
          "chmod 0755 /out/cargo-deny",
          'test "$(/out/cargo-deny --version)" = "cargo-deny $CARGO_DENY_VERSION"',
        ].join("\n"),
      ])
      .file("/out/cargo-deny");
    const toolchains = JSON.parse(
      await source.file("crates/nrz-source-bundle/assets/runtime-toolchains.json").contents(),
    );
    const nodeMajors: unknown = toolchains.node?.supported;
    // Platform handoff fixtures freeze Node 24.
    const nodeMajor = 24;
    if (!Array.isArray(nodeMajors) || !nodeMajors.includes(nodeMajor)) {
      throw new Error(`Runtime toolchain catalog does not qualify CI Node ${nodeMajor}`);
    }
    let ctr = (await rustContainer(source))
      .withFile("/usr/local/bin/cargo-deny", cargoDeny)
      .withFile("/usr/local/bin/bun", (await bunContainer(source)).file("/usr/local/bin/bun"))
      .withMountedDirectory(
        "/opt/node",
        dag.container().from(`node:${nodeMajor}-bookworm`).directory("/usr/local"),
      )
      .withEnvVariable("PATH", "/opt/node/bin:$PATH", { expand: true });
    ctr = ctr.withExec(["rustup", "component", "add", "rustfmt", "clippy"]);
    ctr = ctr.withExec(["cargo", "deny", "check", "licenses"]);
    ctr = ctr.withExec(["cargo", "test", "--locked", "--workspace", "--features", "nrz-contract/codegen"]);
    ctr = ctr.withExec(["cargo", "fmt", "--all", "--check"]);
    ctr = ctr.withExec(["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--features", "nrz-contract/codegen", "--no-deps", "--", "-D", "warnings"]);
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
        ".cache",
        ".moon/cache",
        ".jscpd",
        "mutants.out",
        "mutants.out.old",
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
    /** Git metadata captured from the exact host checkout, including worktrees. */
    gitMetadata: File,
    /** Release channel: stable or beta. */
    channel = "stable",
    /** Optional explicit version. Accepts 1.2.3, v1.2.3, or full prerelease versions. */
    version = "",
    /** Version bump when version is not provided: auto, patch, minor, or major. */
    bump = "auto",
  ): Promise<string> {
    requireChannel(channel);
    const releaseSource = sourceWithReleaseGitMetadata(source, gitMetadata);
    return (await bunContainer(releaseSource))
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
        ".cache",
        ".moon/cache",
        ".jscpd",
        "mutants.out",
        "mutants.out.old",
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
    /** Git metadata captured from the exact host checkout, including worktrees. */
    gitMetadata: File,
    /** Release channel: stable or beta. */
    channel = "stable",
    /** Optional explicit version. Accepts 1.2.3, v1.2.3, or full prerelease versions. */
    version = "",
    /** Version bump when version is not provided: auto, patch, minor, or major. */
    bump = "auto",
  ): Promise<Directory> {
    requireChannel(channel);
    const releaseSource = sourceWithReleaseGitMetadata(source, gitMetadata);
    const releaseDir = (await bunContainer(releaseSource))
      .withEnvVariable("NRZ_RELEASE_CHANNEL", channel)
      .withEnvVariable("NRZ_RELEASE_VERSION", version)
      .withEnvVariable("NRZ_RELEASE_BUMP", bump)
      .withExec(["bun", ".dagger/scripts/release-plan.ts", "--write"])
      .directory("/work");

    return (await rustContainer(releaseDir))
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
  packagePlatform(binary: File, platform: string, notices: File, license: File): File {
    requirePlatform(platform);
    const executable = binName(platform);
    const mode = platform === "win32-x64" ? "0644" : "0755";
    return dag
      .container()
      .from(ALPINE_IMAGE)
      .withMountedFile(`/input/${executable}`, binary)
      .withMountedFile("/input/THIRD_PARTY_NOTICES", notices)
      .withMountedFile("/input/LICENSE", license)
      .withExec([
        "sh",
        "-ceu",
        [
          "set -o pipefail",
          `mkdir -p /out/archive/${platform}`,
          `cp /input/${executable} /out/archive/${platform}/${executable}`,
          `chmod ${mode} /out/archive/${platform}/${executable}`,
          `cp /input/THIRD_PARTY_NOTICES /out/archive/${platform}/THIRD_PARTY_NOTICES`,
          `chmod 0644 /out/archive/${platform}/THIRD_PARTY_NOTICES`,
          `cp /input/LICENSE /out/archive/${platform}/LICENSE`,
          `chmod 0644 /out/archive/${platform}/LICENSE`,
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
  packageReleaseArtifacts(binaries: Directory, notices: File, license: File): Directory {
    return dag
      .container()
      .from(ALPINE_IMAGE)
      .withDirectory("/input", binaries)
      .withMountedFile("/input/THIRD_PARTY_NOTICES", notices)
      .withMountedFile("/input/LICENSE", license)
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
          '  cp /input/THIRD_PARTY_NOTICES "/out/archive/$platform/THIRD_PARTY_NOTICES"',
          '  chmod 0644 "/out/archive/$platform/THIRD_PARTY_NOTICES"',
          '  cp /input/LICENSE "/out/archive/$platform/LICENSE"',
          '  chmod 0644 "/out/archive/$platform/LICENSE"',
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
  async npmPackage(
    @argument({
      ignore: [
        ".cache",
        ".moon/cache",
        ".jscpd",
        "mutants.out",
        "mutants.out.old",
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
  ): Promise<Directory> {
    requireChannel(channel);
    return (await bunContainer(source))
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
        ".cache",
        ".moon/cache",
        ".jscpd",
        "mutants.out",
        "mutants.out.old",
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
    return (await bunContainer(source))
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
