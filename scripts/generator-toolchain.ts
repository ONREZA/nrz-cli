import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rename, rm } from "node:fs/promises";
import { resolve } from "node:path";

export async function runTool(command: string[], cwd: string): Promise<void> {
  const child = Bun.spawn(command, { cwd, stdout: "inherit", stderr: "inherit" });
  if (await child.exited !== 0) throw new Error(`Tool failed: ${command[0]}`);
}

/** Build an immutable upstream revision plus the reviewed generator-source patch. */
export async function generatorBinary(root: string): Promise<string> {
  const localSource = process.env.NRZ_OPENAPI_GENERATOR_SOURCE;
  if (localSource) {
    const source = resolve(localSource);
    const target = resolve(root, ".cache/tools/oas3-gen-local");
    await runTool(["cargo", "build", "--manifest-path", resolve(source, "Cargo.toml"),
      "--target-dir", target, "--locked", "-p", "oas3-gen", "-j", "2"], root);
    return resolve(target, "debug/oas3-gen");
  }
  const lock = JSON.parse(await readFile(resolve(root, "api/generator.lock.json"), "utf8")) as {
    repository: string; revision: string; patch: string; patchSha256: string;
  };
  const patch = resolve(root, lock.patch);
  const digest = createHash("sha256").update(await readFile(patch)).digest("hex");
  if (digest !== lock.patchSha256) throw new Error("Generator source patch checksum mismatch");
  const cacheRoot = resolve(root, ".cache/tools/oas3-gen");
  const cache = resolve(cacheRoot, `${lock.revision}-${digest}`);
  const binary = resolve(cache, "target/debug/oas3-gen");
  if (await Bun.file(binary).exists()) return binary;
  await mkdir(cacheRoot, { recursive: true });
  const temporary = await mkdtemp(resolve(cacheRoot, "build-"));
  try {
    await runTool(["git", "init", "--quiet", temporary], root);
    await runTool(["git", "fetch", "--depth=1", lock.repository, lock.revision], temporary);
    await runTool(["git", "checkout", "--detach", "FETCH_HEAD"], temporary);
    await runTool(["git", "apply", "--check", patch], temporary);
    await runTool(["git", "apply", patch], temporary);
    await runTool(["cargo", "build", "--locked", "-p", "oas3-gen", "-j", "2"], temporary);
    await rename(temporary, cache);
    return binary;
  } catch (error) {
    await rm(temporary, { recursive: true, force: true });
    throw error;
  }
}
