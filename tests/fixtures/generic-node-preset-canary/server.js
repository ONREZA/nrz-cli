// DEPRECATED: Incident-only no-build Node canary; remove after Git deployment acceptance.
const http = require("node:http");

http.createServer((_request, response) => {
  response.writeHead(200, { "content-type": "application/json" });
  response.end(JSON.stringify({ ok: true, fixture: "generic-node-preset-canary", buildScript: false }));
}).listen(Number(process.env.PORT || 3000), "0.0.0.0");
