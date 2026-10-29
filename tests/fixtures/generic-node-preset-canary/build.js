// DEPRECATED: Incident-only built Node canary; remove after built-output Git deployment acceptance.
const { mkdirSync, writeFileSync } = require("node:fs");

mkdirSync("dist/app", { recursive: true });
writeFileSync("dist/app/cloud.mjs", `import http from "node:http";
http.createServer((_request, response) => {
  response.writeHead(200, { "content-type": "application/json" });
  response.end(JSON.stringify({ ok: true, fixture: "generic-node-preset-canary", buildScript: true, outputDirectory: "dist" }));
}).listen(Number(process.env.PORT || 3000), "0.0.0.0");
`);
