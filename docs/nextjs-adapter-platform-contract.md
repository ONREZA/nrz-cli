# Next.js Adapter Platform Contract

Status: design draft
Date: 2026-06-23

This document defines the platform contract for the next high-ROI Next.js
adapter work:

- ISR and Next server cache ownership on ONREZA primitives.
- Server-owned generated Edge Rule contributions.

It intentionally does not move Next route handlers, middleware, proxy, or edge
runtime bundles into ONREZA Functions v1. Compute remains the correctness
fallback and the Next.js renderer.

## Inputs

The contract is based on three source-of-truth boundaries:

1. Next.js Deployment Adapter API is build-time only. The adapter reads
   `routing`, `outputs`, `config`, `nextVersion`, and `buildId` in
   `onBuildComplete`.
2. Next.js runtime cache behavior is handled by the Next server and cache
   interfaces such as `cacheHandler` and `cacheHandlers`.
3. ONREZA Edge Rules are one ordered runtime ruleset materialized from separate
   USER and GENERATED contributions.

Relevant Next.js docs:

- Adapter build output:
  https://github.com/vercel/next.js/blob/v16.2.9/docs/01-app/03-api-reference/07-adapters/02-creating-an-adapter.mdx
- Runtime integration:
  https://github.com/vercel/next.js/blob/v16.2.9/docs/01-app/03-api-reference/07-adapters/07-runtime-integration.mdx
- Cache handlers:
  https://github.com/vercel/next.js/blob/v16.2.9/docs/01-app/03-api-reference/05-config/01-next-config-js/cacheHandlers.mdx
- ISR semantics:
  https://github.com/vercel/next.js/blob/v16.2.9/docs/01-app/02-guides/incremental-static-regeneration.mdx
- PPR platform flow:
  https://github.com/vercel/next.js/blob/v16.2.9/docs/01-app/03-api-reference/07-adapters/06-implementing-ppr-in-an-adapter.mdx

ONREZA contract references:

- Deployment-origin function/rules payload:
  `../../deployment/docs/rfc/onreza-functions/INDEX.md`
- Edge Rules contribution ownership:
  `../../deployment/docs/rfc/onreza-functions/edge-rules.md`

## Non-goals

- No JavaScript middleware compiler.
- No route handler, middleware, proxy, or edge runtime split into ONREZA
  Functions v1.
- No ISR implemented as a plain Edge Rule TTL rewrite.
- No cache of arbitrary Compute HTML unless the route is identified by the
  Next adapter/cache contract as cacheable.
- No local merge of generated rules into `onreza.rules.toml`.
- No hidden ownership transfer from generated rules to user-authored rules.

## Contract A: ISR and Next Cache Substrate

### Ownership

Next Compute owns rendering. ONREZA owns durable cache storage, invalidation,
singleflight regeneration coordination, and edge delivery of responses that are
proven safe to serve before Compute.

This means the adapter must ship a Next cache handler integration rather than
reimplementing Next request rendering. The handler maps Next cache operations to
an ONREZA internal cache service.

### Cache Namespace

Every cache entry is scoped by:

- workspace id;
- project id;
- environment id;
- deployment id;
- Next `buildId`;
- cache family: `next-server-cache`, `next-isr`, `next-ppr`;
- route/cache key produced by the Next handler integration.

By default, cache entries do not cross deployment boundaries. Reuse across
deployments can be added later only for content-addressed entries with explicit
compatibility metadata.

### Build-Time Manifest

Current `nrz-cli` implementation emits this shape as report-only metadata under
`compatibility.platform.nextCache`. It is intentionally not a deploy-time cache
contract yet: no runtime cache handler, cache storage, or edge ISR rules are
activated from this field. The report is route-level and excludes internal Next
RSC/data/segment artifacts from candidate and blocker counts.

Once the runtime substrate exists, the adapter descriptor should gain an
optional `cache` section with the same route classification shape:

```json
{
  "schemaVersion": "NEXT_CACHE_SUBSTRATE_V1",
  "producer": "nextjs-adapter",
  "nextVersion": "16.2.9",
  "buildId": "<next-build-id>",
  "routes": [
    {
      "id": "<next-output-id>",
      "pathname": "/blog/[slug]",
      "kind": "isr",
      "status": "edge_cache_candidate",
      "initialRevalidateSeconds": 60,
      "fallbackFilePath": ".next/server/app/blog/[slug].html",
      "middlewareSafe": true,
      "reason": null
    }
  ]
}
```

Rules:

- `kind = "isr"` is eligible only when Next marks the prerender as
  revalidating and the adapter has enough metadata to build the initial entry.
- `kind = "ppr"` remains `compute_fallback` until the cache substrate supports
  shell bytes, `postponedState`, streaming resume, and response concatenation.
- `middlewareSafe = false` forces Compute fallback, because Next middleware may
  need to run before the cached route response.
- Unknown or partially understood Next output shapes stay in Compute.

### Runtime Cache Operations

The ONREZA internal cache service should expose these logical operations. The
transport can be HTTP, Unix socket, or platform RPC, but the behavior must be
stable:

| Operation | Purpose |
| --- | --- |
| `get(key, context)` | Read cache entry plus freshness metadata. |
| `set(key, entry, context, compare)` | Store a successfully rendered entry with CAS protection. |
| `acquireRegeneration(key, context)` | Singleflight lock for stale/miss regeneration. |
| `releaseRegeneration(key, result)` | Commit success or release without invalidating old data on failure. |
| `refreshTags(context)` | Sync tag invalidation clocks into the handler. |
| `getExpiration(tags, context)` | Return the latest invalidation timestamp for tags. |
| `updateTags(tags, durations, context)` | Persist `revalidateTag` / cache tag updates. |
| `revalidatePath(path, context)` | Persist path invalidation for `revalidatePath`. |

The Next cache handler maps Next's server cache calls to these operations. It is
platform code, bundled by `nrz-cli`, and wired zero-config by the adapter.

### Edge Read Path

For an edge-cache-eligible ISR route:

1. Edge Rules match the route and ask ONREZA cache for the current entry.
2. Fresh hit: serve the cached response from the edge path.
3. Stale hit: serve the stale response and start one regeneration through
   Compute if `acquireRegeneration` succeeds.
4. Miss: forward to Compute. Compute renders through Next and writes the entry
   through the cache handler.
5. Regeneration failure never deletes or replaces the previously successful
   entry.

If async stale revalidation is unavailable, the route must remain Compute
fallback. A blocking TTL cache is not acceptable ISR support.

### Edge Rule Integration

Generated cache rules are emitted as the `nextjs-adapter` GENERATED Edge Rule
contribution. They must use native `cache`/continue actions that reference the
Next cache namespace. User-authored rules still run before generated rules, so a
user rule may intentionally shadow adapter cache acceleration.

The adapter should report shadowing risk in `nrz build --json` when a known
user rule or middleware matcher makes an ISR route unsafe for edge cache.

### Invariants

- Compute remains able to serve every route without the edge cache.
- Cache writes are accepted only from the deployment's Compute runtime through a
  signed internal credential.
- Cache entries are immutable once committed; replacement writes create a new
  generation and atomically move the pointer.
- Stale data is retained on regeneration error.
- Tag/path invalidations are timestamped tombstones, not best-effort deletes.
- Lock TTLs are bounded so a crashed Compute process cannot permanently block
  regeneration.
- Every cache response records `deploymentId`, `buildId`, `routeId`, `cacheKey`,
  freshness state, and whether regeneration was started.

### Minimal Reliable Milestone

M1 can claim ISR platform support only when all of these are true:

- adapter emits a cache manifest for ISR routes;
- Next Compute uses an ONREZA cache handler;
- edge route can serve a fresh build-time ISR entry;
- stale hit keeps serving old content while one Compute regeneration runs;
- failed regeneration keeps the old entry;
- `revalidatePath` and tag invalidation are reflected in the cache service;
- middleware-matched routes remain Compute fallback;
- conformance tests cover fresh, stale, failed regeneration, path invalidation,
  tag invalidation, middleware fallback, and deployment isolation.

Anything less should stay reported as `compute_fallback_isr`.

## Contract B: Server-Owned Generated Edge Rule Contributions

### Current Wire Shape

The public publish payload already has the correct split:

```json
{
  "origin": "DEPLOYMENT",
  "edgeRules": { "schemaVersion": "EDGE_RULE_SET_V1", "rules": [] },
  "edgeRulesForce": false,
  "generatedEdgeRuleSets": [
    {
      "producer": "nextjs-adapter",
      "version": "16.2.9",
      "edgeRules": { "schemaVersion": "EDGE_RULE_SET_V1", "rules": [] }
    }
  ]
}
```

Contract meaning:

- `edgeRules` is the USER contribution from `onreza.rules.toml`.
- `generatedEdgeRuleSets[]` are GENERATED contributions keyed by `producer`.
- `producer = "user"` is reserved and invalid for generated contributions.
- duplicate generated producers are invalid.
- generated contributions can exist without functions.
- an explicit empty generated ruleset clears stale rules for that producer.

### Server Data Model

The server stores contributions separately:

| Field | Meaning |
| --- | --- |
| `kind` | `USER` or `GENERATED`. |
| `producer` | `user` for USER, adapter name for GENERATED. |
| `producerVersion` | Adapter/framework version for generated rules. |
| `authoringSource` | `BUILD` or `UI` for USER only. |
| `status` | `DRAFT`, `ACTIVE`, or `ARCHIVED`. |
| `rules` | Normalized rules array, not TOML text. |
| `checksum` | Hash of normalized contribution rules. |

Activation materializes one effective `EdgeRuleSet`:

1. choose the DRAFT USER contribution for the deployment if present, otherwise
   the active USER contribution;
2. append GENERATED contributions by sorted producer name, preferring DRAFT for
   the deployment over ACTIVE for the same producer;
3. normalize positions in the final effective ruleset;
4. select the materialized ruleset in the runtime release.

### CLI Behavior

`nrz deploy`:

- always sends the `nextjs-adapter` generated contribution when a Next adapter
  descriptor exists;
- sends an explicit empty generated ruleset if the current build no longer has
  lowerable Next rules;
- sends `edgeRules` only when `onreza.rules.toml` exists;
- never writes generated rules into `onreza.rules.toml`;
- never needs local file-write permission to publish generated rules.

`nrz rules publish`:

- publishes only USER rules from `onreza.rules.toml`;
- does not mutate generated contributions.

`nrz rules pull`:

- must pull USER rules only by default;
- if a future "effective rules" export is added, generated rules must be marked
  read-only or require an explicit take-over/import mode before they can become
  USER rules.

### Conflict Guard

Generated contributions do not participate in USER divergence checks.

Build-origin USER rules conflict only when:

- the deploy includes `edgeRules`;
- the active USER contribution is UI-authored;
- the active USER checksum differs from the local normalized checksum;
- `edgeRulesForce` is false.

In that case the server returns `EDGE_RULES_DIVERGED` and the CLI tells the user
to run `nrz rules pull` or redeploy with `--force-rules`.

### Generic Endpoint Direction

The current transport can remain the deployment/functions publish payload. The
data model is already generic, so a later endpoint should be a thin transport
over the same contribution semantics:

```text
PUT /v1/projects/:projectId/environments/:environmentId/edge-rule-contributions/:producer
```

Required semantics:

- only updates the named GENERATED producer;
- rejects `producer = user`;
- validates the same `EdgeRuleSetAuthoring` contract;
- accepts empty `rules` to clear the producer;
- returns the new contribution checksum and effective ruleset preview;
- does not change USER ownership;
- uses the same activation/staging rules as deployment-origin publishes.

This endpoint is useful for future producers that need rules without a deploy or
without ONREZA Functions, but it must not introduce a second composition model.

### Invariants

- The hot path sees one ordered effective `EdgeRuleSet`.
- USER and GENERATED contributions remain separately auditable.
- A producer can replace only its own generated contribution.
- UI can display generated rules as platform-owned, but cannot edit them as user
  rules.
- Rollback restores the contribution snapshot selected by the runtime release.
- Absence of `onreza.rules.toml` never deletes USER rules.
- Presence of the adapter descriptor should refresh or clear the adapter's
  generated contribution on every deploy.

## How A and B Fit Together

ISR cache acceleration uses both contracts:

1. Adapter emits cache metadata from Next build outputs.
2. Adapter emits generated Edge Rules for edge-cache-eligible ISR paths.
3. Server stores those rules as the `nextjs-adapter` generated contribution.
4. Compute runs the real Next server with the ONREZA cache handler.
5. Edge serves only fresh/stale entries that the ONREZA cache service proves
   belong to the active deployment/build.

The result is fast on ONREZA primitives and still correct when any part is not
eligible: unsupported routes fall through to Compute, not to a partial runtime.

Current implementation stops before step 1 becomes deploy-owned metadata:
`nrz-cli` reports candidate coverage in `compatibility.platform.nextCache` only.
That lets us measure real app eligibility without changing runtime behavior.

## Implementation Order

1. Keep `compatibility.platform.nextCache` report-only until the runtime cache
   substrate is implemented and deployment-owned.
2. Extend the adapter descriptor with `cache` metadata after the deploy contract
   can consume it safely.
3. Add server-side generated-contribution status/preview fields if current APIs
   cannot expose USER vs GENERATED clearly enough.
4. Build the ONREZA Next cache handler and internal cache service contract.
5. Wire Compute environment variables for the cache handler endpoint and token.
6. Emit generated cache Edge Rules only for routes that pass all safety gates.
7. Add conformance tests for ISR freshness, stale regeneration, failed
   regeneration, tag/path invalidation, middleware fallback, and deployment
   isolation.
8. Start PPR only after ISR cache ownership is reliable.
