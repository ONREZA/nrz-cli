#!/usr/bin/env bun
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { pathToFileURL } from "node:url";

export interface GitCommitMetadata {
  hash: string;
  subject: string;
  body: string;
}

export interface ReleaseGitMetadata {
  head: string;
  dirty: boolean;
  previousTag: string | null;
  tags: string[];
  commits: GitCommitMetadata[];
}

export function captureGitMetadata(cwd: string): ReleaseGitMetadata {
  const run = (args: string[]) => execFileSync("git", args, {
    cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"],
  });
  const tags = (args: string[]) => run(["tag", "--list", "v[0-9]*", "--sort=-v:refname", ...args])
    .trim().split("\n").filter(Boolean);
  const head = run(["rev-parse", "--verify", "HEAD"]).trim();
  const previousTag = tags(["--merged", head]).find(tag => !tag.includes("-")) ?? null;
  const raw = run(["log", "-z", "--format=%H%x00%s%x00%b", previousTag ? `${previousTag}..${head}` : head]);
  const fields = raw.split("\0");
  if (fields.at(-1) === "") fields.pop();
  if (fields.length % 3 !== 0) throw new Error("Invalid Git log metadata");
  const commits: GitCommitMetadata[] = [];
  for (let index = 0; index < fields.length; index += 3) {
    commits.push({ hash: fields[index], subject: fields[index + 1], body: fields[index + 2] });
  }
  return {
    head,
    dirty: run(["status", "--porcelain", "--untracked-files=normal"]).length > 0,
    previousTag,
    tags: tags([]),
    commits,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const outputPath = process.argv[2] || ".nrz-release/git.json";
  const metadata = captureGitMetadata(process.cwd());
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(metadata, null, 2)}\n`);
}
