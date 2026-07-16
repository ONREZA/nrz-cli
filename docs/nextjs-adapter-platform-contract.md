# Next.js Adapter Platform Contract

Status: superseded as a design source
Updated: 2026-07-16

The platform contract moved to the deployment repository so generic platform capabilities and
Next.js-specific translation have separate owners:

- [`Framework Adapter Platform`](../../deployment/docs/rfc/framework-adapter-platform/INDEX.md)
  owns program goals, economics, rollout, observability, and completion levels.
- [`Platform Capabilities`](../../deployment/docs/rfc/framework-adapter-platform/platform-capabilities.md)
  owns framework-neutral routing, cache, deferred response, bundle, media, and conformance
  contracts.
- [`Next.js Mapping`](../../deployment/docs/rfc/framework-adapter-platform/nextjs-mapping.md)
  owns the mapping from Next.js outputs, routing buckets, and runtime hooks to ONREZA primitives.

This file remains only to preserve existing links. New platform design must not be added here.

## Current Implementation Projection

The implementation-facing status remains in
[`nextjs-adapter-support.md`](./nextjs-adapter-support.md). The important ownership boundaries are:

- full standalone Compute is the correctness fallback;
- generated Edge Rule contributions are already implemented and producer-owned;
- ISR/PPR cache metadata is report-only and does not activate a runtime cache;
- the existing flat Edge Rule runtime does not preserve every Next.js routing phase;
- ONREZA Functions v1 is a self-contained source contract and is not a framework bundle runtime.

## Historical Contract Status

The previous version of this document combined two independent decisions: the future runtime
cache substrate and generated Edge Rule contribution ownership. Generated contributions are now
implemented and documented by the deployment repository's ONREZA Functions Edge Rules RFC.
Runtime cache, phased routing, PPR, executable bundles, and media fetch remain draft capabilities
in the new RFC family.
