import { expect, test } from "bun:test";
import { selectOperations } from "./api-contract";

test("selection preserves wire schemas, path parameters, and recursive discriminator references", () => {
  const operation = {
    operationId: "getItem",
    responses: { "200": { content: { "application/json": { schema: { $ref: "#/components/schemas/Item" } } } } },
  };
  const item = {
    type: "object",
    properties: { next: { $ref: "#/components/schemas/Item" }, value: { type: ["string", "null"] } },
    discriminator: { propertyName: "kind", mapping: { leaf: "#/components/schemas/Leaf" } },
  };
  const document = {
    openapi: "3.1.2",
    jsonSchemaDialect: "https://json-schema.org/draft/2020-12/schema",
    security: [{ ApiKey: [] }],
    info: { title: "test", version: "1" },
    paths: {
      "/items/{id}": { get: operation, parameters: [{ name: "id", in: "path", required: true }] },
      "/private": { get: { operationId: "private" } },
    },
    components: { schemas: { Item: item, Leaf: { type: "string" }, Unused: { type: "integer" } } },
  };
  const result = selectOperations(document, ["getItem"]);
  expect(result.jsonSchemaDialect).toBe(document.jsonSchemaDialect);
  expect(result.security).toEqual(document.security);
  expect(result.paths).toEqual({ "/items/{id}": document.paths["/items/{id}"] });
  expect(result.components.schemas).toEqual({ Item: item, Leaf: { type: "string" } });
  expect(() => selectOperations(document, ["missing"])).toThrow("Missing server operations");
  expect(() => selectOperations(document, ["getItem", "getItem"])).toThrow("Duplicate requested");
});


test("selection fails on missing referenced definitions and ambiguous operation IDs", () => {
  const operation = { operationId: "item", responses: { "200": { $ref: "#/components/schemas/Missing" } } };
  expect(() => selectOperations({ paths: { "/item": { get: operation } } }, ["item"]))
    .toThrow("Unresolved contract reference");
  expect(() => selectOperations({ paths: { "/item": { get: operation, post: operation } } }, ["item"]))
    .toThrow("Duplicate operation");
});


test("selection rejects operations that document only errors", () => {
  const operation = { operationId: "incomplete", responses: { "400": { description: "Bad request" } } };
  expect(() => selectOperations({ paths: { "/incomplete": { get: operation } } }, ["incomplete"]))
    .toThrow("Missing successful response contract: incomplete");
  const noContent = { operationId: "remove", responses: { "204": { description: "Deleted" } } };
  expect(selectOperations({ paths: { "/item": { delete: noContent } } }, ["remove"]).paths)
    .toEqual({ "/item": { delete: noContent } });
});
