#!/usr/bin/env bun
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

const repoUrl = "https://github.com/ONREZA/nrz-cli";
const metadataPath = ".nrz-release/metadata.json";
const defaultChannel = "stable";
const releaseCommitPrefix = "chore(release):";
const vendoredCrateDependencies = [
  { name: "nrz-contract", path: "vendor/onreza-crates/nrz-contract" },
  { name: "nrz-fn-policy", path: "vendor/onreza-crates/nrz-fn-policy" },
  { name: "nrz-source-bundle", path: "vendor/onreza-crates/nrz-source-bundle" },
] as const;

type Bump = "major" | "minor" | "patch";

interface ParsedVersion {
  major: number;
  minor: number;
  patch: number;
  prerelease: string;
}

interface RawGitCommit {
  hash: string;
  subject: string;
  body?: string;
}

export interface ReleaseCommit {
  hash: string;
  shortHash: string;
  subject: string;
  type: string;
  scope: string;
  text: string;
  breaking: boolean;
}

interface GitMetadata {
  previousTag?: string | null;
  tags?: string[];
  commits?: RawGitCommit[];
}

interface ResolveOptions {
  channel?: string;
  explicitVersion?: string;
  bumpInput?: string;
  tags?: string[];
}

const typeHeadings: Record<string, string> = {
  feat: "\u2728 Features",
  fix: "\u{1f41b} Bug Fixes",
  perf: "\u26a1 Performance",
  docs: "\u{1f4da} Documentation",
  refactor: "\u267b\ufe0f Changed",
  style: "\u{1f3a8} Changed",
  chore: "\u{1f527} Changed",
  ci: "\u{1f477} CI/CD",
  test: "\u2705 Testing",
};

function run(command: string, args: string[]): string {
  return execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function maybeRun(command: string, args: string[]): string {
  try {
    return run(command, args);
  } catch {
    return "";
  }
}

function readVersion(): string {
  const cargo = readFileSync("Cargo.toml", "utf8");
  const match = cargo.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error("Could not read package version from Cargo.toml");
  }
  return match[1];
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function parseVersion(version: string): ParsedVersion {
  const normalized = version.replace(/^v/, "");
  const match = normalized.match(/^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/);
  if (!match) {
    throw new Error(`Invalid semver version: ${version}`);
  }
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] || "",
  };
}

export function formatVersion(version: ParsedVersion): string {
  const base = `${version.major}.${version.minor}.${version.patch}`;
  return version.prerelease ? `${base}-${version.prerelease}` : base;
}

export function bumpVersion(current: string, bump: Bump): string {
  const next = { ...parseVersion(current), prerelease: "" };
  if (bump === "major") {
    next.major += 1;
    next.minor = 0;
    next.patch = 0;
  } else if (bump === "minor") {
    next.minor += 1;
    next.patch = 0;
  } else if (bump === "patch") {
    next.patch += 1;
  } else {
    throw new Error(`Unsupported bump: ${bump}`);
  }
  return formatVersion(next);
}

export function normalizeBump(currentVersion: string, commits: ReleaseCommit[], requestedBump = "auto"): Bump {
  if (["major", "minor", "patch"].includes(requestedBump)) {
    return requestedBump as Bump;
  }
  if (requestedBump !== "auto") {
    throw new Error(`Unsupported bump: ${requestedBump}`);
  }
  if (commits.some((commit) => commit.breaking)) {
    return parseVersion(currentVersion).major === 0 ? "minor" : "major";
  }
  if (commits.some((commit) => commit.type === "feat")) {
    return "minor";
  }
  return "patch";
}

function readGitMetadata(): GitMetadata | undefined {
  if (!existsSync(".nrz-release/git.json")) {
    return undefined;
  }
  return JSON.parse(readFileSync(".nrz-release/git.json", "utf8")) as GitMetadata;
}

function latestStableTag(metadata = readGitMetadata()): string | undefined {
  if (metadata) {
    return metadata.previousTag || undefined;
  }
  const tags = maybeRun("git", ["tag", "--list", "v[0-9]*", "--sort=-v:refname"]);
  return tags
    .split("\n")
    .map((tag) => tag.trim())
    .filter(Boolean)
    .find((tag) => !tag.includes("-"));
}

function commitsSince(tag: string | undefined, metadata = readGitMetadata()): ReleaseCommit[] {
  if (metadata) {
    return filterReleaseCommits(
      (metadata.commits || []).map((commit) => parseCommit(commit.hash, commit.subject, commit.body || "")),
    );
  }
  const range = tag ? `${tag}..HEAD` : "HEAD";
  const raw = maybeRun("git", ["log", "--pretty=format:%H%x01%s%x01%b%x02", range]);
  if (!raw) {
    return [];
  }
  return raw
    .split("\x02")
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => {
      const [hash, subject, body = ""] = entry.split("\x01");
      return parseCommit(hash, subject, body);
    })
    .filter((commit) => !isReleaseCommit(commit));
}

function parseCommit(hash: string, subject: string, body: string): ReleaseCommit {
  const match = subject.match(/^(\w+)(?:\(([^)]+)\))?(!)?:\s+(.+)$/);
  const type = match?.[1] || "chore";
  const scope = match?.[2] || "";
  const text = match?.[4] || subject;
  const breaking = Boolean(match?.[3]) || body.includes("BREAKING CHANGE") || body.includes("BREAKING-CHANGE");
  return { hash, shortHash: hash.slice(0, 7), subject, type, scope, text, breaking };
}

export function isReleaseCommit(commit: ReleaseCommit): boolean {
  return commit.subject.startsWith(releaseCommitPrefix);
}

export function filterReleaseCommits(commits: ReleaseCommit[]): ReleaseCommit[] {
  return commits.filter((commit) => !isReleaseCommit(commit));
}

function nextPrereleaseNumber(baseVersion: string, channelName: string, tagsFromMetadata?: string[]): number {
  const prefix = `v${baseVersion}-${channelName}.`;
  if (tagsFromMetadata) {
    const latestFromMetadata = tagsFromMetadata
      .map((tag) => String(tag).trim())
      .filter((tag) => tag.startsWith(prefix))
      .sort((a, b) => b.localeCompare(a, undefined, { numeric: true }))
      .at(0);
    if (!latestFromMetadata) {
      return 0;
    }
    const number = Number(latestFromMetadata.slice(prefix.length));
    return Number.isFinite(number) ? number + 1 : 0;
  }
  const tags = maybeRun("git", ["tag", "--list", `${prefix}*`, "--sort=-v:refname"]);
  const latest = tags
    .split("\n")
    .map((tag) => tag.trim())
    .filter(Boolean)
    .find((tag) => tag.startsWith(prefix));
  if (!latest) {
    return 0;
  }
  const number = Number(latest.slice(prefix.length));
  return Number.isFinite(number) ? number + 1 : 0;
}

function stableBase(version: string): string {
  return formatVersion({ ...parseVersion(version), prerelease: "" });
}

function prereleaseChannel(version: string): string {
  const prerelease = parseVersion(version).prerelease;
  return prerelease ? prerelease.split(".")[0] : "";
}

function releaseTrainBase(currentVersion: string, commits: ReleaseCommit[], requestedBump: string): string {
  const current = parseVersion(currentVersion);
  if (current.prerelease && requestedBump === "auto") {
    return stableBase(currentVersion);
  }
  if (commits.length === 0) {
    throw new Error("Cannot derive a release version without changes; pass --version to force an explicit version");
  }
  return bumpVersion(currentVersion, normalizeBump(currentVersion, commits, requestedBump));
}

export function resolveVersion(currentVersion: string, commits: ReleaseCommit[], options: ResolveOptions = {}): string {
  const channel = options.channel || defaultChannel;
  const explicitVersion = (options.explicitVersion || "").trim();
  const bumpInput = options.bumpInput || "auto";

  if (explicitVersion) {
    const normalized = explicitVersion.replace(/^v/, "");
    const parsed = parseVersion(normalized);
    if (channel === "stable") {
      if (parsed.prerelease) {
        throw new Error("Stable releases cannot use a prerelease version");
      }
      return normalized;
    }
    if (parsed.prerelease) {
      const versionChannel = prereleaseChannel(normalized);
      if (versionChannel !== channel) {
        throw new Error(
          `Explicit prerelease version ${normalized} belongs to channel ${versionChannel}, not ${channel}`,
        );
      }
      return normalized;
    }
    return `${normalized}-${channel}.0`;
  }

  const baseVersion = releaseTrainBase(currentVersion, commits, bumpInput);
  if (channel === "stable") {
    return baseVersion;
  }
  return `${baseVersion}-${channel}.${nextPrereleaseNumber(baseVersion, channel, options.tags)}`;
}

function groupCommits(commits: ReleaseCommit[]): Map<string, ReleaseCommit[]> {
  const groups = new Map<string, ReleaseCommit[]>();
  for (const commit of commits) {
    const heading = typeHeadings[commit.type] || typeHeadings.chore;
    const entries = groups.get(heading) || [];
    entries.push(commit);
    groups.set(heading, entries);
  }
  return groups;
}

function changelogEntry(version: string, commits: ReleaseCommit[]): string {
  const date = new Date().toISOString().slice(0, 10);
  const lines = [`## [${version}] - ${date}`, ""];
  const groups = groupCommits(commits);
  if (groups.size === 0) {
    lines.push("### Changed", "", "- Release metadata update", "");
    return lines.join("\n");
  }

  for (const [heading, entries] of groups) {
    lines.push(`### ${heading}`, "");
    for (const commit of entries) {
      const scope = commit.scope ? `**${commit.scope}:** ` : "";
      lines.push(`- ${scope}${commit.text} ([${commit.shortHash}](${repoUrl}/commit/${commit.hash}))`);
    }
    lines.push("");
  }
  return lines.join("\n");
}

export function releaseCargoToml(cargo: string, version: string): string {
  let updated = cargo.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
  for (const crate of vendoredCrateDependencies) {
    const re = new RegExp(`^${escapeRegExp(crate.name)}\\s*=.*$`, "m");
    if (!re.test(updated)) {
      throw new Error(`Could not update ${crate.name} dependency in Cargo.toml`);
    }
    const next = updated.replace(re, `${crate.name} = { path = "${crate.path}" }`);
    updated = next;
  }
  return updated;
}

function updateLockPackageVersion(lock: string, packageName: string, version: string): string {
  const re = new RegExp(`(\\[\\[package\\]\\]\\nname = "${escapeRegExp(packageName)}"\\nversion = ")[^"]+(")`);
  const updated = lock.replace(re, `$1${version}$2`);
  if (updated === lock) {
    throw new Error(`Could not update ${packageName} version in Cargo.lock`);
  }
  return updated;
}

export function releaseCargoLock(lock: string, version: string): string {
  return updateLockPackageVersion(lock, "nrz", version);
}

function updateCargoVersion(version: string): void {
  writeFileSync("Cargo.toml", releaseCargoToml(readFileSync("Cargo.toml", "utf8"), version));
  for (const crate of vendoredCrateDependencies) {
    if (!existsSync(join(crate.path, "Cargo.toml"))) {
      throw new Error(`Missing sanitized nrz-cli crate snapshot: ${join(crate.path, "Cargo.toml")}`);
    }
  }
  writeFileSync("Cargo.lock", releaseCargoLock(readFileSync("Cargo.lock", "utf8"), version));
}

function updatePackageVersion(version: string): void {
  const packageJson = JSON.parse(readFileSync("package.json", "utf8")) as { version: string };
  packageJson.version = version;
  writeFileSync("package.json", `${JSON.stringify(packageJson, null, 2)}\n`);

  const packageLock = JSON.parse(readFileSync("package-lock.json", "utf8")) as {
    version: string;
    packages?: Record<string, { version?: string }>;
  };
  packageLock.version = version;
  if (packageLock.packages?.[""]) {
    packageLock.packages[""].version = version;
  }
  writeFileSync("package-lock.json", `${JSON.stringify(packageLock, null, 2)}\n`);
}

function updateChangelog(version: string, commits: ReleaseCommit[]): void {
  const changelog = readFileSync("CHANGELOG.md", "utf8");
  const entry = changelogEntry(version, commits);
  const marker = "All notable changes to this project will be documented in this file.\n\n";
  if (!changelog.includes(marker)) {
    throw new Error("Unexpected CHANGELOG.md format");
  }
  writeFileSync("CHANGELOG.md", changelog.replace(marker, `${marker}${entry}\n`));
}

function writeMetadata(metadata: object): void {
  mkdirSync(dirname(metadataPath), { recursive: true });
  writeFileSync(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`);
}

function main(): void {
  const write = process.argv.includes("--write");
  const channel = process.env.NRZ_RELEASE_CHANNEL || defaultChannel;
  const explicitVersion = process.env.NRZ_RELEASE_VERSION || "";
  const bumpInput = process.env.NRZ_RELEASE_BUMP || "auto";
  const gitMetadata = readGitMetadata();
  const currentVersion = readVersion();
  const previousTag = latestStableTag(gitMetadata);
  const commits = commitsSince(previousTag, gitMetadata);
  const nextVersion = resolveVersion(currentVersion, commits, {
    channel,
    explicitVersion,
    bumpInput,
    tags: gitMetadata?.tags,
  });
  const tag = `v${nextVersion}`;
  const metadata = {
    version: nextVersion,
    tag,
    channel,
    prerelease: channel !== "stable",
    npmDistTag: channel === "stable" ? "latest" : channel,
    currentVersion,
    previousTag: previousTag || null,
    commitCount: commits.length,
  };

  if (write) {
    updateCargoVersion(nextVersion);
    updatePackageVersion(nextVersion);
    updateChangelog(nextVersion, commits);
    writeMetadata(metadata);
  }

  process.stdout.write(`${JSON.stringify(metadata, null, 2)}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
