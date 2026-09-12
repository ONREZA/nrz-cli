type JsonObject = { [key: string]: JsonValue };
type JsonValue = null | boolean | number | string | JsonValue[] | JsonObject;

function object(value: unknown, location: string): JsonObject {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`Expected an object at ${location}`);
  }
  return value as JsonObject;
}

const METHODS = ["get", "post", "put", "patch", "delete", "head", "options", "trace"];

/** Select operations without rewriting any request or response schema. */
export function selectOperations(input: unknown, requested: readonly string[]) {
  const document = object(input, "document");
  const components = object(document.components ?? {}, "components");
  const sourceSchemas = object(components.schemas ?? {}, "components.schemas");
  const pending = new Set(requested);
  const wanted = new Set(requested);
  if (pending.size !== requested.length) throw new Error("Duplicate requested operation IDs");
  const paths: JsonObject = {};
  for (const [path, value] of Object.entries(object(document.paths, "paths"))) {
    const item = object(value, `paths.${path}`);
    const selected: JsonObject = {};
    for (const method of METHODS) {
      if (!item[method]) continue;
      const operation = object(item[method], `paths.${path}.${method}`);
      if (typeof operation.operationId !== "string" || !wanted.has(operation.operationId)) continue;
      if (!pending.delete(operation.operationId)) throw new Error(`Duplicate operation: ${operation.operationId}`);
      const responses = object(operation.responses ?? {}, `${method.toUpperCase()} ${path} responses`);
      if (!Object.keys(responses).some((status) => /^2(?:[0-9]{2}|XX)$/.test(status))) {
        throw new Error(`Missing successful response contract: ${operation.operationId} (${method.toUpperCase()} ${path})`);
      }
      selected[method] = operation;
    }
    if (Object.keys(selected).length) {
      for (const [key, value] of Object.entries(item)) {
        if (!METHODS.includes(key)) selected[key] = value;
      }
      paths[path] = selected;
    }
  }
  if (pending.size) throw new Error(`Missing server operations: ${[...pending].join(", ")}`);

  const schemas: JsonObject = {};
  const visited = new Set<string>();
  function visit(value: JsonValue): void {
    if (!value || typeof value !== "object") return;
    if (Array.isArray(value)) {
      value.forEach(visit);
      return;
    }
    if (typeof value.$ref === "string") {
      const ref = value.$ref;
      if (!ref.startsWith("#/components/schemas/")) throw new Error(`Unsupported contract reference: ${ref}`);
      if (!visited.has(ref)) {
        visited.add(ref);
        const key = ref.slice("#/components/schemas/".length).replaceAll("~1", "/").replaceAll("~0", "~");
        const schema = sourceSchemas[key];
        if (schema === undefined) throw new Error(`Unresolved contract reference: ${ref}`);
        schemas[key] = schema;
        visit(schema);
      }
    }
    if (value.discriminator) {
      const discriminator = object(value.discriminator, "discriminator");
      if (discriminator.mapping) {
        for (const ref of Object.values(object(discriminator.mapping, "discriminator.mapping"))) visit({ $ref: ref });
      }
    }
    for (const child of Object.values(value)) visit(child);
  }
  visit(paths);
  return {
    openapi: document.openapi,
    info: document.info,
    ...(document.jsonSchemaDialect ? { jsonSchemaDialect: document.jsonSchemaDialect } : {}),
    ...(document.servers ? { servers: document.servers } : {}),
    paths,
    components: { schemas, securitySchemes: components.securitySchemes ?? {} },
    ...(document.security ? { security: document.security } : {}),
  };
}
