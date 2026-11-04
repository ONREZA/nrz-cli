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
const original = { ok: true, fixture: "generic-node-preset-canary", buildScript: true, outputDirectory: "dist" };
const marker = { ok: true, fixture: "runtime-write-canary", mkdir: true, overwrite: true };
http.createServer((request, response) => {
  response.writeHead(200, { "content-type": "application/json", "cache-control": "no-store" });
  response.end(JSON.stringify((request.url || "/").split("?")[0] === "/__runtime-write-canary" ? marker : original));
}).listen(Number(process.env.PORT || 3000), "0.0.0.0");
`);
