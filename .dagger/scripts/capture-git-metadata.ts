#!/usr/bin/env bun
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const outputPath = process.argv[2] || ".nrz-release/git.json";

interface GitCommitMetadata {
  hash: string;
  subject: string;
  body: string;
}

function run(args: string[]): string {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function maybeRun(args: string[]): string {
  try {
    return run(args);
  } catch {
    return "";
  }
}

function tags(): string[] {
  return maybeRun(["tag", "--list", "v[0-9]*", "--sort=-v:refname"])
    .split("\n")
    .map((tag) => tag.trim())
    .filter(Boolean);
}

function commitsSince(tag: string | null): GitCommitMetadata[] {
  const range = tag ? `${tag}..HEAD` : "HEAD";
  const raw = maybeRun(["log", "--pretty=format:%H%x01%s%x01%b%x02", range]);
  if (!raw) {
    return [];
  }
  return raw
    .split("\x02")
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => {
      const [hash = "", subject = "", body = ""] = entry.split("\x01");
      return { hash, subject, body };
    });
}

const allTags = tags();
const previousTag = allTags.find((tag) => !tag.includes("-")) || null;
const metadata = {
  previousTag,
  tags: allTags,
  commits: commitsSince(previousTag),
};

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(metadata, null, 2)}\n`);
