#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { RELEASE_REPOSITORY, REQUIRED_RELEASE_ASSETS } from "./release-assets";

const token = process.env.GITHUB_TOKEN;
const repository = process.env.GITHUB_REPOSITORY || RELEASE_REPOSITORY;
const version = process.env.NRZ_RELEASE_VERSION;
const tag = process.env.NRZ_RELEASE_TAG;
const channel = process.env.NRZ_RELEASE_CHANNEL || "stable";
const prerelease = channel !== "stable";
const apiBase = "https://api.github.com";
const uploadBase = "https://uploads.github.com";
const artifactRoot = "/dist";
export interface GitHubAsset {
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
type AssetDownloader = (assetId: number) => Promise<Uint8Array>;

export interface ChecksumArtifact {
  path: string;
  text: string;
}

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

async function downloadAsset(assetId: number): Promise<Uint8Array> {
  const response = await fetch(`${apiBase}/repos/${repository}/releases/assets/${assetId}`, {
    headers: {
      Accept: "application/octet-stream",
      Authorization: `Bearer ${token}`,
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });
  if (!response.ok) {
    throw new Error(`Download release asset ${assetId} failed: ${response.status} ${await response.text()}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

export async function verifyPublishedAssets(
  release: GitHubRelease,
  expectedPaths: Readonly<Record<string, string>>,
  downloader: AssetDownloader = downloadAsset,
): Promise<void> {
  const assets = new Map((release.assets || []).map((asset) => [asset.name, asset]));

  for (const [name, expectedPath] of Object.entries(expectedPaths)) {
    const asset = assets.get(name);
    if (!asset) {
      throw new Error(`Release ${release.tag_name} is missing published asset: ${name}`);
    }

    const expectedDigest = createHash("sha256").update(readFileSync(expectedPath)).digest("hex");
    const actualDigest = createHash("sha256").update(await downloader(asset.id)).digest("hex");
    if (actualDigest !== expectedDigest) {
      throw new Error(`Published release asset ${name} does not match the CI-built artifact`);
    }
  }
}

export function appendChecksumsToReleaseNotes(notes: string, checksumText: string): string {
  const checksums = checksumText.trim();
  if (!checksums) {
    return notes.trim();
  }

  return `${notes.trim()}\n\n### Checksums (SHA-256)\n\n\`\`\`text\n${checksums}\n\`\`\``;
}

function releaseNotes(checksumText: string): string {
  const changelog = readFileSync("CHANGELOG.md", "utf8");
  const startMarker = `## [${version}]`;
  const start = changelog.indexOf(startMarker);
  let notes: string;
  if (start === -1) {
    notes = `nrz ${tag}`;
  } else {
    const next = changelog.indexOf("\n## [", start + startMarker.length);
    notes = changelog.slice(start, next === -1 ? undefined : next).trim();
  }
  return appendChecksumsToReleaseNotes(notes, checksumText);
}

export function createChecksums(paths: string[]): ChecksumArtifact {
  const lines = paths.map((path) => {
    const digest = createHash("sha256").update(readFileSync(path)).digest("hex");
    return `${digest}  ${basename(path)}`;
  });
  const checksumPath = "/tmp/checksums-sha256.txt";
  const text = `${lines.join("\n")}\n`;
  writeFileSync(checksumPath, text);
  return { path: checksumPath, text };
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

  const assetPaths = REQUIRED_RELEASE_ASSETS.map((asset) => requireAsset(asset));
  const checksum = createChecksums(assetPaths);
  const releaseBody = releaseNotes(checksum.text);
  const allUploads = [...assetPaths, checksum.path];
  const allNames = [...REQUIRED_RELEASE_ASSETS, "checksums-sha256.txt"];
  const expectedPaths = Object.fromEntries(allNames.map((name, index) => [name, allUploads[index]]));

  let release = await findReleaseByTag(repository, tag);
  if (release && !release.draft) {
    await verifyPublishedAssets(release, expectedPaths);
    process.stdout.write(
      `${JSON.stringify({ tag, version, channel, releaseUrl: release.html_url, status: "already-published" }, null, 2)}\n`,
    );
    return;
  }

  if (!release) {
    release = await request<GitHubRelease>("POST", `/repos/${repository}/releases`, {
      tag_name: tag,
      name: `nrz ${tag}`,
      body: releaseBody,
      draft: true,
      prerelease,
      generate_release_notes: false,
    });
    if (!release) {
      throw new Error(`Could not create release ${tag}`);
    }
  } else {
    release = await request<GitHubRelease>("PATCH", `/repos/${repository}/releases/${release.id}`, {
      name: `nrz ${tag}`,
      body: releaseBody,
      prerelease,
    });
    if (!release) {
      throw new Error(`Release ${tag} disappeared while updating release notes`);
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
  await verifyPublishedAssets(release, expectedPaths);

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
