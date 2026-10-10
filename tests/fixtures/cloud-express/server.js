import { createRequire } from "node:module";
import express from "express";

const require = createRequire(import.meta.url);
const app = express();

app.get("/", (_request, response) => {
  response.json({
    marker: "onreza-cloud-express",
    node: process.versions.node,
    express: require("express/package.json").version,
  });
});

app.get("/items/:id", (request, response) => {
  response.json({ item_id: Number(request.params.id), q: request.query.q ?? null });
});

app.listen(Number(process.env.PORT ?? "3000"), "0.0.0.0");
