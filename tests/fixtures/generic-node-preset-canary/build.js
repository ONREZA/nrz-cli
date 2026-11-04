// DEPRECATED: Final Corepack incident canary; remove by exact forward restore after acceptance.
(() => {
  const assert = require("node:assert/strict");
  const fs = require("node:fs");
  const path = require("node:path");
  const { execFileSync } = require("node:child_process");
  const tmp = fs.realpathSync(process.env.TMPDIR || "");
  assert.ok(tmp.startsWith("/workspace/"), "Corepack canary requires the execution disk workspace TMPDIR");
  assert.notEqual(fs.statfsSync(tmp, { bigint: true }).type, 0x01021994n, "TMPDIR must not use tmpfs");
  assert.equal(process.env.COREPACK_ENABLE_NETWORK, "0", "Corepack must remain offline");
  const root = fs.mkdtempSync(path.join(tmp, "onreza-corepack-final-"));
  try {
    const dependency = path.join(root, "dependency");
    fs.mkdirSync(dependency);
    fs.writeFileSync(path.join(dependency, "package.json"), JSON.stringify({
      name: "onreza-corepack-local-dependency", version: "1.0.0", main: "index.cjs",
      bin: { "onreza-corepack-local-bin": "bin.cjs" },
      scripts: { postinstall: "node -e \"require('node:fs').writeFileSync('ready.json', 'true')\"" },
    }));
    fs.writeFileSync(path.join(dependency, "index.cjs"), "module.exports = 'local-dependency-ok';\n");
    fs.writeFileSync(path.join(dependency, "bin.cjs"), "#!/usr/bin/env node\nprocess.stdout.write('local-bin-ok');\n");
    fs.chmodSync(path.join(dependency, "bin.cjs"), 0o755);
    fs.writeFileSync(path.join(root, "package.json"), JSON.stringify({
      name: "onreza-corepack-final-canary", version: "1.0.0", private: true,
      packageManager: "pnpm@11.25.0",
      dependencies: { "onreza-corepack-local-dependency": "file:./dependency" },
      scripts: { build: "node build.cjs" },
    }));
    fs.writeFileSync(path.join(root, "build.cjs"), String.raw`
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { execFileSync } = require('node:child_process');
assert.equal(require('onreza-corepack-local-dependency'), 'local-dependency-ok');
assert.equal(require('onreza-corepack-local-dependency/ready.json'), true);
assert.equal(execFileSync('onreza-corepack-local-bin', { encoding: 'utf8' }), 'local-bin-ok');
fs.writeFileSync('build-proof.json', JSON.stringify({ dependency: true, postinstall: true, bin: true, build: true }));
`);
    const run = (command, args) => execFileSync(command, args, {
      cwd: root, env: process.env, encoding: "utf8", timeout: 45000, maxBuffer: 1024 * 1024,
      stdio: ["ignore", "pipe", "pipe"],
    });
    assert.equal(run("pnpm", ["--version"]).trim(), "11.28.8", "Platform pnpm must use its bundled version");
    assert.ok(run("pnpx", ["--help"]).trim().length > 0, "pnpx help must work without downloading a package");
    run("pnpm", ["install", "--offline"]);
    const lock = fs.readFileSync(path.join(root, "pnpm-lock.yaml"));
    run("pnpm", ["install", "--offline", "--frozen-lockfile"]);
    assert.deepEqual(fs.readFileSync(path.join(root, "pnpm-lock.yaml")), lock, "Frozen install changed its lockfile");
    run("pnpm", ["run", "build"]);
    assert.deepEqual(JSON.parse(fs.readFileSync(path.join(root, "build-proof.json"), "utf8")), {
      dependency: true, postinstall: true, bin: true, build: true,
    });
    process.stdout.write("ONREZA_COREPACK_CANARY_PASS pnpm=11.28.8 pnpx=help localDependency=ok postinstall=ok bin=ok frozenLock=unchanged build=ok\n");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
})();
// End of the reversible Corepack canary block.

// DEPRECATED: Incident-only runtime artifact COW canary; remove after runtime DAC acceptance.
const { mkdirSync, writeFileSync } = require("node:fs");

mkdirSync(".output/server", { recursive: true });
mkdirSync(".output/public/nested", { recursive: true });
mkdirSync(".output/.onreza", { recursive: true });
writeFileSync(".output/public/nested/status.txt", "artifact-baseline\n");
const manifest = {
  "version": 2,
  "layers": [
    {
      "name": "public-assets",
      "target": "STATIC",
      "directory": "public"
    },
    {
      "name": "server",
      "target": "COMPUTE",
      "directory": ".",
      "entry": "server/index.mjs",
      "runtime": {
        "network": {
          "schemaVersion": "COMPUTE_NETWORK_V1",
          "exposure": "PUBLIC",
          "ports": [
            {
              "name": "http",
              "protocol": "http",
              "port": 3000,
              "binding": "primary"
            }
          ],
          "readiness": {
            "protocol": "tcp",
            "port": "http"
          },
          "allowFrom": []
        }
      }
    }
  ],
  "routes": [
    {
      "pattern": "^/nested/.*$",
      "layer": "public-assets",
      "priority": 100
    },
    {
      "pattern": "^/.*$",
      "layer": "server",
      "priority": 0
    }
  ]
};
writeFileSync(".output/.onreza/manifest.json", JSON.stringify(manifest, null, 2) + "\n");
writeFileSync(".output/server/index.mjs", String.raw`import http from "node:http";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
await import("express");
const nested = fileURLToPath(new URL("../public/nested/", import.meta.url));
const status = join(nested, "status.txt");
let stage = "baseline";
try {
  const before = readFileSync(status, "utf8");
  if (!["artifact-baseline\n", "runtime-cow-ok\n"].includes(before)) throw new Error("unexpected baseline");
  stage = "mkdir";
  const created = join(nested, "runtime-created");
  mkdirSync(created, { recursive: true });
  writeFileSync(join(created, "created.txt"), "runtime-created\n");
  stage = "overwrite";
  writeFileSync(status, "runtime-cow-ok\n");
  if (readFileSync(status, "utf8") !== "runtime-cow-ok\n" || readFileSync(join(created, "created.txt"), "utf8") !== "runtime-created\n") throw new Error("write verification failed");
} catch (error) {
  console.error(JSON.stringify({ fixture: "runtime-write-canary", stage, errorCode: typeof error.code === "string" ? error.code : "CANARY_FAILED" }));
  process.exit(1);
}
const original = { ok: true, fixture: "generic-node-preset-canary", buildScript: true, outputDirectory: "dist", publicationCanary: "20261101-corepack-final-release" };
const marker = { ok: true, fixture: "runtime-write-canary", mkdir: true, overwrite: true };
http.createServer((request, response) => {
  response.writeHead(200, { "content-type": "application/json", "cache-control": "no-store" });
  response.end(JSON.stringify((request.url || "/").split("?")[0] === "/__runtime-write-canary" ? marker : original));
}).listen(Number(process.env.PORT || 3000), "0.0.0.0");
`);
