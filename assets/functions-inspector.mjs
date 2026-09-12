const HANDLER_NAMES = ["fetch", "request", "response", "observe", "manual", "queue", "scheduled"];

function jsonValue(value, ancestors = new Set(), depth = 0) {
  if (depth > 32) throw new Error("Function config exceeds the maximum nesting depth");
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value !== "object") throw new Error("Function config must contain JSON values only");
  if (ancestors.has(value)) throw new Error("Function config must not contain cycles");
  const prototype = Object.getPrototypeOf(value);
  if (!Array.isArray(value) && prototype !== Object.prototype && prototype !== null) {
    throw new Error("Function config must contain plain objects and arrays only");
  }
  if (Object.getOwnPropertySymbols(value).length > 0) throw new Error("Function config must not contain symbol keys");
  ancestors.add(value);
  try {
    if (Array.isArray(value)) return Array.from(value, item => jsonValue(item, ancestors, depth + 1));
    return Object.fromEntries(Object.keys(value).map(key => [key, jsonValue(value[key], ancestors, depth + 1)]));
  } finally {
    ancestors.delete(value);
  }
}

async function inspectModule(entrypointUrl) {
    const module = await import(entrypointUrl);
    const candidate = module.default;
    let handlers;
    if (typeof candidate === "function") {
      handlers = [...HANDLER_NAMES];
    } else if (candidate !== null && typeof candidate === "object" && !Array.isArray(candidate)) {
      handlers = HANDLER_NAMES.filter(name => {
        const handler = candidate[name];
        if (handler === undefined) return false;
        if (typeof handler !== "function") throw new Error(`Function handler '${name}' must be callable`);
        return true;
      });
    } else {
      throw new Error("Function must export a default function or handler object");
    }
    if (handlers.length === 0) throw new Error("Function must export at least one supported handler");
    const config = jsonValue(module.config === undefined ? {} : module.config);
    if (config === null || typeof config !== "object" || Array.isArray(config)) {
      throw new Error("Function config must be an object");
    }
    if (Array.isArray(config.triggers)) {
      for (const trigger of config.triggers) {
        if (trigger && ["manual", "queue", "scheduled"].includes(trigger.type) && !handlers.includes(trigger.type)) {
          throw new Error(`Function trigger '${trigger.name}' requires a '${trigger.type}' handler`);
        }
      }
    }
    return { config, handlers };
}

export default {
  async manual(event, ctx) {
    try {
      ctx.locals.inspection = await inspectModule(event.entrypointUrl);
    } catch (error) {
      let message = "Function initialization or declaration is invalid";
      try {
        message = String(error?.message ?? error).slice(0, 4096);
      } catch {}
      ctx.locals.inspectionError = { message };
    }
  },
};
