# Next.js Adapter Support Matrix

This document is an implementation-facing support matrix for the ONREZA
Next.js adapter. Keep it current when adapter behavior changes; the contents can
later be moved into user-facing documentation.

Canonical platform decisions are tracked in the deployment repository:

- [`Framework Adapter Platform`](../../deployment/docs/rfc/framework-adapter-platform/INDEX.md)
- [`Platform Capabilities`](../../deployment/docs/rfc/framework-adapter-platform/platform-capabilities.md)
- [`Next.js Mapping`](../../deployment/docs/rfc/framework-adapter-platform/nextjs-mapping.md)

[`nextjs-adapter-platform-contract.md`](./nextjs-adapter-platform-contract.md) is kept only as a
compatibility pointer for older links.

## Version Boundary

| Area | Status | Notes |
| --- | --- | --- |
| Next.js Adapter API | Supported for installed `next >= 16.2.0` | `nrz deploy` writes a bundled adapter and sets `NEXT_ADAPTER_PATH` automatically. The installed `node_modules/next/package.json` version is the source of truth when available; declared dependency ranges are a conservative fallback. Build conformance validated on Next.js 16.2.10. |
| Older Next.js versions | Legacy fallback | `nrz deploy` keeps the `NEXT_PRIVATE_STANDALONE=1` standalone path. Validated on Next.js 14, 15, and 16.1.6 without user `output: 'standalone'` config. |
| User install step | Not required | The adapter is bundled in `nrz-cli`; users do not install a package. |

## Platform Mapping

| Next.js feature/output | ONREZA primitive | Current status | Limitations |
| --- | --- | --- | --- |
| Standalone Node server | Compute | Supported | This remains the correctness fallback for framework runtime behavior. |
| Static export | Generic STATIC deployment | Supported | Next.js 16.2.10 emits an adapter descriptor for `output: 'export'`, but no standalone server; `nrz` therefore follows the generic static build path and does not publish adapter Compute compatibility metadata. |
| `.next/static` files | STATIC + Compute fallthrough | Supported | With middleware/proxy, only matcher-disjoint assets are split into STATIC. |
| `public/` assets | STATIC + Compute fallthrough | Supported | With middleware/proxy, only matcher-disjoint assets are split into STATIC. |
| Fully static prerenders | STATIC + Compute fallthrough | Supported | Requires static HTML fallback, `initialRevalidate=false`, safe pathname, no matching middleware, and no exact Next redirect on the same pathname. Exact redirect conflicts stay in Compute to preserve Next routing semantics. |
| ISR prerenders | Compute fallback, report-only cache candidate classification | Classification supported | `compatibility.platform.nextCache` reports edge-cache candidates and blockers. Regeneration, cache key, bypass, and revalidation ownership are not implemented on ONREZA cache primitives yet. |
| PPR prerenders | Compute fallback, classified | Classification supported | Resume/rendering, partial shell upgrade, and PPR cache semantics stay in the Next.js Compute runtime. |
| `basePath` | Adapter config + STATIC/Compute routing | Supported | Validated with App Router static, dynamic, metadata, public assets, and generated Edge Rules under `/docs`. |
| `trailingSlash` | Edge Rules + Compute fallback | Supported | Canonical redirect rules are generated. Static prerenders that would shadow an exact slash redirect are kept in Compute. |
| `i18n` | Pages Router + STATIC/Compute routing | Supported | Build-validated with locale prerenders and `pages/api` fallback on Next.js 16.2.10; the last stage runtime smoke used 16.2.9. |
| Image optimizer | ONREZA image optimizer or Compute fallback | Partial native support | For Next.js Adapter API builds with default image config, the adapter installs a generated custom loader that emits `/_onreza/image?...` URLs. Representable HTTPS `images.remotePatterns` with exact default port (`port: ""`, including `URL` objects) are published as GENERATED Edge Rules `imageSources`; omitted `search` allows any query while an explicit value matches exactly. Deprecated `images.domains` cannot restrict protocol/port/path, so mapping it to HTTPS:443 would silently narrow behavior and remains on Compute. Custom loaders/paths, `unoptimized`, non-HTTPS or non-representable remote patterns, custom `localPatterns`, non-WebP format negotiation, SVG/local-IP policy overrides, custom redirect limits, and `assetPrefix` stay on the Next Compute fallback or user-configured path. |
| Metadata routes | STATIC public copy + Compute fallback | Supported | Static App Router metadata `.body` outputs such as `robots.txt` and `sitemap.xml` are copied into `public/`; dynamic metadata routes stay in Compute. |
| Redirects, rewrites, headers | Edge Rules + Compute fallback | Phase-safe native subset | Lowers equivalent `beforeMiddleware` redirects/headers, `beforeFiles` rules only without middleware using `ifNoFile=false`, and immutable Next static cache headers. `afterFiles`, `dynamicRoutes`, `fallback`, invalid targets, generic regex, and unsupported conditions remain in Compute; affected STATIC outputs are withheld so they cannot shadow framework routing. |
| Generated adapter Edge Rules | Server-owned Edge Rule contributions | Supported in deploy contract | Adapter rules are published as a generated contribution keyed by producer; they are not merged into `onreza.rules.toml`. |
| Middleware/proxy | Compute fallback | Classified | Matchers are used for safe STATIC splitting, but arbitrary middleware code is not lowered into Edge Rules or Functions. |
| `pages/api` handlers | Compute fallback, classified | Classification supported | Next emits framework artifacts with traced assets/runtime protocol; ONREZA Functions v1 only accepts self-contained source files. |
| `app` route handlers | Compute fallback, classified | Classification supported | Same limitation as `pages/api`; edge-runtime handlers are additionally blocked by edge bundle semantics. |
| Next edge runtime outputs | Compute fallback | Classified | Edge runtime outputs are framework bundles/chunks, not ONREZA Functions v1 source files. |

## ONREZA Functions Boundary

ONREZA Functions v1 is intentionally a self-contained source contract:

- one branded function entry file per function;
- source-size bounded entry payload;
- no `node_modules` graph or traced asset graph in the public publish payload;
- no arbitrary Next.js invocation/cache/request adapter protocol.

Because of that, the Next.js adapter must not split `APP_ROUTE`, `PAGES_API`,
or `MIDDLEWARE` outputs into ONREZA Functions until the platform has a framework
bundle contract. Today the adapter reports why each route handler stays in
Compute.

## Validated Matrix

Last local Next.js 16.2.10 validation: 2026-08-12. Last legacy full matrix and stage runtime
validation: 2026-06-24.

Automated local conformance (`NRZ_NEXTJS_CONFORMANCE=1 cargo test --test
nextjs_adapter_conformance_test`) covers:

- Next.js 16.2.10 Adapter API with App Router `basePath`, `trailingSlash`,
  redirects, rewrites, headers, metadata routes, local and HTTPS remote images through the
  ONREZA image optimizer, generated remote-image Edge Rules, ISR route classification,
  middleware matcher classification, and route handlers.
- Next.js 16.2.10 Pages Router `i18n`, locale static prerenders, and
  `pages/api` Compute fallback.
- Next.js 16.2.10 static export through the generic STATIC deployment path.
- The last full legacy conformance run covered Next.js 14, 15, and 16.1.6 through
  `NEXT_PRIVATE_STANDALONE=1`, with no adapter descriptor emitted.

Historical stage smoke coverage on 2026-06-24 verified:

- `basePath` `/docs`, canonical `/docs -> /docs/` redirect, dynamic redirect,
  header rule, header-gated rewrite, metadata `robots.txt`/`sitemap.xml`,
  public asset, and image optimizer Compute fallback.
- Native ONREZA image optimizer routing for a Next.js 16.2.9 `basePath` app:
  generated HTML emitted `/_onreza/image?url=/public/photo.jpg&w=32&q=75`,
  and stage returned `200 image/webp` with `server: ONREZA`.
- Pages Router `i18n` locale pages, `pages/api` handler, and redirect.
- Legacy Next.js 14, 15, and 16.1.6 deploy/runtime paths using the standalone
  fallback.

Some stage deploys in that historical run emitted a non-fatal detection-sync warning from the
API (`500 INTERNAL_ERROR`). The package-manager enum mapping was fixed after the run; the full
stage matrix has not been refreshed since then.

## Compatibility Report

`nrz build --json` includes `compatibility` for Next.js adapter builds, and
`nrz build` / `nrz deploy` emit a concise report line:

```text
Next.js adapter report: STATIC <n> (...), prerenders <static> static/<isr> ISR/<ppr> PPR (...), ISR cache <candidates> candidates/<blocked> blocked (...), image optimizer <status>, route handlers <n> (...), Edge Rules <generated> generated/<unsupported> unsupported (...), middleware ..., edge runtime outputs <n>
```

Important JSON fields:

| Field | Meaning |
| --- | --- |
| `platform.staticFiles` | STATIC split status and counts. |
| `platform.prerenders` | Aggregate prerender status plus route-level static/ISR/PPR classification. |
| `platform.nextCache` | Report-only route-level ISR/PPR cache substrate classification. Counts eligible ISR cache candidates and blocked ISR/PPR routes, excluding internal Next RSC/data/segment artifacts; it does not enable runtime cache behavior yet. |
| `platform.imageOptimizer` | Whether `next/image` was lowered to `/_onreza/image` via the generated loader, or why it remains on Compute/user config. |
| `platform.routeHandlers` | Route-level `pages/api` and `app` route handler classification for Compute vs Functions readiness, excluding internal Next RSC/data/segment artifacts. |
| `platform.routing` | Generated vs unsupported Edge Rules from Next routing buckets. |
| `platform.middleware` | Middleware/proxy runtime and fallback reason. |
| `platform.edgeRuntime` | Count and fallback reason for edge runtime outputs. |
| `config` | Captured Next config signals currently relevant to mapping (`basePath`, `i18n`, `trailingSlash`). |

## Next Work

1. Add typed routing lifecycle checkpoints before expanding native lowering to `afterFiles`,
   `dynamicRoutes`, or `fallback`.
2. Map the platform deployment identity into Next.js `deploymentId` and add the official Next.js
   deploy/log/cleanup conformance harness.
3. Promote `platform.nextCache` from report-only coverage into the generic durable runtime cache
   contract while keeping Next Compute as the renderer.
4. Add PPR only after runtime cache ownership and request lifetime extension are reliable.
5. Keep generated Edge Rules server-owned and producer-keyed; do not merge them into
   `onreza.rules.toml`.
6. Revisit route-handler/middleware native execution only after the platform supports a
   multi-file executable bundle artifact with assets/WASM and an explicit invocation protocol.
