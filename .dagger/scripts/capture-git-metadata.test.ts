import { test, expect } from "bun:test";
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { captureGitMetadata } from "./capture-git-metadata";

test("metadata follows the selected worktree and reachable release history", () => {
  const root = mkdtempSync(join(tmpdir(), "nrz-release-worktree-"));
  try {
    const git = (...args: string[]) => execFileSync("git", [
      "-c", "core.hooksPath=/dev/null", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.test", ...args,
    ], {
      cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env, GIT_CONFIG_GLOBAL: "/dev/null", GIT_CONFIG_NOSYSTEM: "1", GIT_TERMINAL_PROMPT: "0" },
    }).trim();
    git("init", "-b", "main");
    writeFileSync(join(root, "source"), "first");
    git("add", "source");
    git("commit", "-m", "feat(cli): first");
    git("tag", "v0.1.0");
    const base = git("rev-parse", "HEAD");
    git("switch", "-c", "future");
    writeFileSync(join(root, "source"), "future");
    git("commit", "-am", "feat(cli): future");
    git("tag", "v9.0.0");
    git("switch", "main");
    writeFileSync(join(root, "source"), "second");
    git("commit", "-am", "fix(cli): second", "-m", "body with old delimiters \x01 and \x02");
    const metadata = captureGitMetadata(root);
    expect(metadata.previousTag).toBe("v0.1.0");
    expect(metadata.tags).toContain("v9.0.0");
    expect(metadata.commits).toHaveLength(1);
    expect(metadata.commits[0].body).toContain("\x01 and \x02");
    expect(metadata.dirty).toBe(false);
    const worktree = join(root, "isolated");
    git("worktree", "add", "--detach", worktree, base);
    const selected = captureGitMetadata(worktree);
    expect(selected.head).toBe(base);
    expect(selected.head).not.toBe(metadata.head);
    expect(selected.commits).toEqual([]);
    expect(selected.dirty).toBe(false);
    writeFileSync(join(worktree, "source"), "uncommitted");
    expect(captureGitMetadata(worktree).dirty).toBe(true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("metadata collection fails outside Git instead of inventing empty history", () => {
  const root = mkdtempSync(join(tmpdir(), "nrz-release-no-git-"));
  try {
    expect(() => captureGitMetadata(root)).toThrow();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
