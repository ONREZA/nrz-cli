#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

const token = process.env.GITHUB_TOKEN;
const repository = process.env.GITHUB_REPOSITORY || "ONREZA/nrz-cli";
const version = process.env.NRZ_RELEASE_VERSION;
const tag = process.env.NRZ_RELEASE_TAG;
const channel = process.env.NRZ_RELEASE_CHANNEL || "stable";
const prerelease = channel !== "stable";
const apiBase = "https://api.github.com";
const uploadBase = "https://uploads.github.com";
const artifactRoot = "/dist";
const requiredAssets = [
  "nrz-linux-x64.tar.gz",
  "nrz-darwin-x64.tar.gz",
  "nrz-darwin-arm64.tar.gz",
  "nrz-win32-x64.tar.gz",
];

interface GitHubAsset {
  id: number;
  name: string;
}

export interface GitHubRelease {
  id: number;
  tag_name: string;
  draft: boolean;
  html_url: string;
  assets?: GitHubAsset[];
}

type Requester = <T>(method: string, path: string, body?: unknown) => Promise<T | null>;

function walk(dir: string): string[] {
  const entries: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      entries.push(...walk(path));
    } else if (stat.isFile()) {
      entries.push(path);
    }
  }
  return entries;
}

function findAsset(name: string): string | undefined {
  return walk(artifactRoot).find((path) => basename(path) === name);
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T | null> {
  const response = await fetch(`${apiBase}${path}`, {
    method,
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      "X-GitHub-Api-Version": "2022-11-28",
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (response.status === 404) {
    return null;
  }
  const text = await response.text();
  const data = (text ? JSON.parse(text) : {}) as T;
  if (!response.ok) {
    throw new Error(`${method} ${path} failed: ${response.status} ${text}`);
  }
  return data;
}

export async function findReleaseByTag(
  repositoryName: string,
  releaseTag: string,
  requester: Requester = request,
): Promise<GitHubRelease | null> {
  const published = await requester<GitHubRelease>(
    "GET",
    `/repos/${repositoryName}/releases/tags/${releaseTag}`,
  );
  if (published) {
    return published;
  }

  const releases = await requester<GitHubRelease[]>("GET", `/repos/${repositoryName}/releases?per_page=100`);
  return releases?.find((release) => release.tag_name === releaseTag) ?? null;
}

async function uploadAsset(releaseId: number, filePath: string, name: string): Promise<void> {
  const response = await fetch(
    `${uploadBase}/repos/${repository}/releases/${releaseId}/assets?name=${encodeURIComponent(name)}`,
    {
      method: "POST",
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${token}`,
        "Content-Type": name.endsWith(".txt") ? "text/plain" : "application/gzip",
        "X-GitHub-Api-Version": "2022-11-28",
      },
      body: readFileSync(filePath),
    },
  );
  const text = await response.text();
  if (!response.ok) {
    throw new Error(`Upload ${name} failed: ${response.status} ${text}`);
  }
}

function releaseNotes(): string {
  const changelog = readFileSync("CHANGELOG.md", "utf8");
  const startMarker = `## [${version}]`;
  const start = changelog.indexOf(startMarker);
  if (start === -1) {
    return `nrz ${tag}`;
  }
  const next = changelog.indexOf("\n## [", start + startMarker.length);
  return changelog.slice(start, next === -1 ? undefined : next).trim();
}

function createChecksums(paths: string[]): string {
  const lines = paths.map((path) => {
    const digest = createHash("sha256").update(readFileSync(path)).digest("hex");
    return `${digest}  ${basename(path)}`;
  });
  const checksumPath = "/tmp/checksums-sha256.txt";
  writeFileSync(checksumPath, `${lines.join("\n")}\n`);
  return checksumPath;
}

function requireAsset(name: string): string {
  const assetPath = findAsset(name);
  if (!assetPath) {
    throw new Error(`Missing required release asset: ${name}`);
  }
  return assetPath;
}

async function main(): Promise<void> {
  if (!token || !version || !tag) {
    throw new Error("GITHUB_TOKEN, NRZ_RELEASE_VERSION, and NRZ_RELEASE_TAG are required");
  }

  const assetPaths = requiredAssets.map((asset) => requireAsset(asset));
  const checksumPath = createChecksums(assetPaths);
  const allUploads = [...assetPaths, checksumPath];
  const allNames = [...requiredAssets, "checksums-sha256.txt"];

  let release = await findReleaseByTag(repository, tag);
  if (release && !release.draft) {
    const existingNames = new Set((release.assets || []).map((asset) => asset.name));
    const complete = allNames.every((name) => existingNames.has(name));
    if (complete) {
      process.stdout.write(
        `${JSON.stringify({ tag, version, channel, releaseUrl: release.html_url, status: "already-published" }, null, 2)}\n`,
      );
      return;
    }
    throw new Error(`Release ${tag} already exists and is published but does not have a complete asset set`);
  }

  if (!release) {
    release = await request<GitHubRelease>("POST", `/repos/${repository}/releases`, {
      tag_name: tag,
      name: `nrz ${tag}`,
      body: releaseNotes(),
      draft: true,
      prerelease,
      generate_release_notes: false,
    });
    if (!release) {
      throw new Error(`Could not create release ${tag}`);
    }
  }

  for (const asset of release.assets || []) {
    if (allNames.includes(asset.name)) {
      await request<Record<string, never>>("DELETE", `/repos/${repository}/releases/assets/${asset.id}`);
    }
  }

  for (let index = 0; index < allUploads.length; index += 1) {
    const filePath = allUploads[index];
    if (!existsSync(filePath)) {
      throw new Error(`Upload path does not exist: ${filePath}`);
    }
    await uploadAsset(release.id, filePath, allNames[index]);
  }

  release = await request<GitHubRelease>("GET", `/repos/${repository}/releases/${release.id}`);
  if (!release) {
    throw new Error(`Release ${tag} disappeared after upload`);
  }
  const uploadedNames = new Set((release.assets || []).map((asset) => asset.name));
  const missing = allNames.filter((name) => !uploadedNames.has(name));
  if (missing.length > 0) {
    throw new Error(`Release ${tag} is missing uploaded assets: ${missing.join(", ")}`);
  }

  const finalizeBody: { draft: boolean; prerelease: boolean; make_latest?: "true" } = {
    draft: false,
    prerelease,
  };
  if (!prerelease) {
    finalizeBody.make_latest = "true";
  }

  release = await request<GitHubRelease>("PATCH", `/repos/${repository}/releases/${release.id}`, finalizeBody);
  if (!release) {
    throw new Error(`Release ${tag} disappeared after finalize`);
  }
  process.stdout.write(
    `${JSON.stringify({ tag, version, channel, releaseUrl: release.html_url, status: "published" }, null, 2)}\n`,
  );
}

if (import.meta.main) {
  await main();
}
