const fs = require('fs')
const path = require('path')

const ADAPTER_NAME = '@onreza/nrz-next-adapter'
const OUTPUT_RELATIVE_PATH =
  process.env.ONREZA_NEXT_ADAPTER_OUTPUT || '.onreza/next-adapter-output.json'
const ONREZA_IMAGE_OPTIMIZER_PATH = '/_onreza/image'
const ONREZA_IMAGE_LOADER_RELATIVE_PATH =
  './.onreza/cache/next-adapter/onreza-image-loader.mjs'
const STANDALONE_SERVER_TRACE_FILES = Object.freeze([
  'next-server.js.nft.json',
  'next-minimal-server.js.nft.json',
])
const MAX_REMOTE_IMAGE_SOURCES = 128
const MAX_REMOTE_IMAGE_PATHNAME_LENGTH = 1024
const MAX_REMOTE_IMAGE_SEARCH_LENGTH = 2048
const REMOTE_IMAGE_DOMAIN_PATTERN =
  /^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z](?:[a-z0-9-]{0,61}[a-z0-9])?$/
const NEXT_IMAGE_RUNTIME_POLICY_DEFAULTS = Object.freeze({
  minimumCacheTTL: 14_400,
  maximumDiskCacheSize: undefined,
  maximumRedirects: 3,
  maximumResponseBody: 50_000_000,
  contentSecurityPolicy: "script-src 'none'; frame-src 'none'; sandbox;",
  contentDispositionType: 'attachment',
  customCacheHandler: false,
})
const installedImageSourcesByProject = new Map()

function jsonReplacer(_key, value) {
  if (typeof value === 'function' || typeof value === 'symbol') {
    return undefined
  }
  if (typeof value === 'bigint') {
    return value.toString()
  }
  return value
}

function hasItems(value) {
  return Array.isArray(value) && value.length > 0
}

function hasNonEmptyString(value) {
  return typeof value === 'string' && value.length > 0
}

function normalizePathname(value) {
  if (typeof value !== 'string' || value.length === 0) {
    return null
  }
  if (value.length > 1 && value.endsWith('/')) {
    return value.slice(0, -1)
  }
  return value
}

function defaultNextImagePath(config) {
  const basePath =
    typeof config.basePath === 'string' && config.basePath !== '/'
      ? config.basePath
      : ''
  return `${basePath}/_next/image`
}

function isDefaultNextImagePath(config, value) {
  const pathname = normalizePathname(value)
  if (pathname == null) {
    return true
  }
  return pathname === '/_next/image' || pathname === defaultNextImagePath(config)
}

function isDefaultLocalPatterns(value) {
  if (!Array.isArray(value) || value.length === 0) {
    return true
  }
  return (
    value.length === 1 &&
    value[0] &&
    value[0].pathname === '**' &&
    value[0].search === ''
  )
}

function isDefaultImageQualities(value) {
  return (
    value == null ||
    (Array.isArray(value) && value.length === 1 && value[0] === 75)
  )
}

function firstNonDefaultImageRuntimePolicy(images) {
  for (const [name, defaultValue] of Object.entries(
    NEXT_IMAGE_RUNTIME_POLICY_DEFAULTS,
  )) {
    if (images[name] !== undefined && images[name] !== defaultValue) {
      return name
    }
  }
  return null
}

function hasUserAssetPrefix(config) {
  if (!hasNonEmptyString(config.assetPrefix)) {
    return false
  }
  return config.assetPrefix !== config.basePath
}

function normalizeRemoteHostname(value, allowWildcard) {
  if (typeof value !== 'string' || value.length === 0) {
    return null
  }
  const normalized = value.trim().toLowerCase()
  const wildcard = allowWildcard
    ? normalized.match(/^(?:\*\*|\*)\./)?.[0] || ''
    : ''
  const exact = normalized.slice(wildcard.length)
  if (
    exact.length === 0 ||
    exact.endsWith('.') ||
    exact.includes('*') ||
    /[%\s/:@?#]/.test(exact)
  ) {
    return null
  }
  let hostname
  try {
    hostname = new URL(`https://${exact}/`).hostname
  } catch (_error) {
    return null
  }
  if (hostname.length > 253 || !REMOTE_IMAGE_DOMAIN_PATTERN.test(hostname)) {
    return null
  }
  return `${wildcard}${hostname}`
}

function normalizeRemotePathname(value) {
  if (
    typeof value !== 'string' ||
    value.length === 0 ||
    value.length > MAX_REMOTE_IMAGE_PATHNAME_LENGTH ||
    !value.startsWith('/') ||
    /[\s?#]/.test(value) ||
    /%(?:2e|2f|5c|25)/i.test(value)
  ) {
    return null
  }
  const segments = value.split('/')
  const supported = segments.every((segment, index) => {
    if (!segment.includes('*')) {
      return true
    }
    return segment === '*' || (segment === '**' && index === segments.length - 1)
  })
  return supported ? value : null
}

function normalizeRemoteSearch(value, exactByDefault) {
  if (value == null) {
    return exactByDefault ? '' : undefined
  }
  if (
    typeof value !== 'string' ||
    value.length > MAX_REMOTE_IMAGE_SEARCH_LENGTH ||
    (value !== '' && (!value.startsWith('?') || value.length === 1)) ||
    /[\s#]/.test(value)
  ) {
    return null
  }
  return value
}

function normalizeRemotePattern(pattern, index) {
  if (!pattern || (typeof pattern !== 'object' && typeof pattern !== 'string')) {
    return null
  }
  let value = pattern
  let exactSearchByDefault = false
  if (typeof pattern === 'string') {
    try {
      value = new URL(pattern)
      exactSearchByDefault = true
    } catch (_error) {
      return null
    }
  }
  const protocol = value.protocol
  const port = value.port
  const hostname = normalizeRemoteHostname(value.hostname, true)
  const pathname = normalizeRemotePathname(value.pathname ?? '/**')
  const search = normalizeRemoteSearch(value.search, exactSearchByDefault)
  if (
    (protocol !== 'https' && protocol !== 'https:') ||
    port !== '' ||
    hostname == null ||
    (!(value instanceof URL) && hostname !== value.hostname) ||
    pathname == null ||
    search === null ||
    value.username ||
    value.password ||
    value.hash
  ) {
    return null
  }
  const source = {
    id: `next.images.remote-pattern.${index}`,
    protocol: 'https',
    hostname,
    pathname,
  }
  if (search !== undefined) {
    source.search = search
  }
  return source
}

function buildRemoteImageSources(images) {
  if (images.domains != null && !Array.isArray(images.domains)) {
    return null
  }
  if (images.remotePatterns != null && !Array.isArray(images.remotePatterns)) {
    return null
  }
  const domains = images.domains || []
  const remotePatterns = images.remotePatterns || []
  // Next's deprecated domains contract matches hostname only, including HTTP and
  // arbitrary ports. Lowering it into HTTPS:443 Edge Rules would silently narrow
  // user behavior, so it remains on the correctness-preserving Compute path.
  if (domains.length > 0) {
    return null
  }
  if (domains.length + remotePatterns.length > MAX_REMOTE_IMAGE_SOURCES) {
    return null
  }

  const sources = []
  for (const [index, pattern] of remotePatterns.entries()) {
    const source = normalizeRemotePattern(pattern, index)
    if (source == null) {
      return null
    }
    sources.push(source)
  }
  return sources
}

function imageOptimizerDecision(config) {
  const images =
    config.images && typeof config.images === 'object' ? config.images : {}

  if (images.unoptimized === true) {
    return {
      status: 'disabled',
      primitive: 'Next.js image config',
      reason: 'images.unoptimized is enabled',
    }
  }

  if (images.loader != null && images.loader !== 'default') {
    return {
      status: 'user_configured',
      primitive: 'user image loader',
      reason: 'images.loader is user-configured',
    }
  }

  if (hasNonEmptyString(images.loaderFile)) {
    return {
      status: 'user_configured',
      primitive: 'user image loader',
      reason: 'images.loaderFile is user-configured',
    }
  }

  if (!isDefaultNextImagePath(config, images.path)) {
    return {
      status: 'user_configured',
      primitive: 'user image path',
      reason: 'images.path is user-configured',
    }
  }

  if (hasUserAssetPrefix(config)) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: 'assetPrefix requires Next.js image URL semantics',
    }
  }

  const customRuntimePolicy = firstNonDefaultImageRuntimePolicy(images)
  if (customRuntimePolicy != null) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: `images.${customRuntimePolicy} requires Next.js image optimizer semantics`,
    }
  }

  if (!isDefaultImageQualities(images.qualities)) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: 'custom image qualities require Next.js quality validation',
    }
  }

  const remoteImageSources = buildRemoteImageSources(images)
  if (remoteImageSources == null) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: 'remote image config cannot be represented by ONREZA Edge Rules',
    }
  }

  if (!isDefaultLocalPatterns(images.localPatterns)) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: 'localPatterns require Next.js local image validation',
    }
  }

  if (
    hasItems(images.formats) &&
    images.formats.some((format) => format !== 'image/webp')
  ) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: 'custom image formats require Next.js content negotiation',
    }
  }

  if (images.dangerouslyAllowSVG === true) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: 'SVG optimizer security policy is user-configured',
    }
  }

  if (images.dangerouslyAllowLocalIP === true) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: 'local-IP image fetching is incompatible with ONREZA SSRF policy',
    }
  }

  return {
    status: 'onreza_optimizer',
    mode: 'custom_loader',
    path: ONREZA_IMAGE_OPTIMIZER_PATH,
    primitive: 'ONREZA image optimizer',
    reason: null,
    remoteImageSources,
  }
}

function buildImageOptimizerHint(config, installedRemoteImageSources) {
  if (installedRemoteImageSources) {
    return {
      status: 'onreza_optimizer',
      mode: 'custom_loader',
      path: ONREZA_IMAGE_OPTIMIZER_PATH,
      primitive: 'ONREZA image optimizer',
      reason: null,
      remoteImageSources: installedRemoteImageSources,
    }
  }
  const { remoteImageSources: _remoteImageSources, ...decision } =
    imageOptimizerDecision(config)
  return decision
}

function imageLoaderSource(config) {
  const basePath =
    typeof config.basePath === 'string' && config.basePath !== '/'
      ? config.basePath
      : ''
  return `'use client'

const BASE_PATH = ${JSON.stringify(basePath)}

function normalizeSource(src) {
  if (typeof src !== 'string') {
    return ''
  }
  if (BASE_PATH && src.startsWith(BASE_PATH + '/')) {
    return src.slice(BASE_PATH.length)
  }
  return src
}

function sourceBundlePath(src) {
  const normalized = normalizeSource(src)
  if (normalized.slice(0, 8).toLowerCase() === 'https://') {
    return normalized
  }
  if (normalized.startsWith('/_static/') || normalized.startsWith('/public/')) {
    return normalized
  }
  if (normalized.startsWith('/_next/static/')) {
    return '/_static' + BASE_PATH + normalized
  }
  if (normalized.startsWith('/')) {
    return '/public' + normalized
  }
  return '/public/' + normalized
}

export default function onrezaImageLoader({ src, width, quality }) {
  const params = new URLSearchParams()
  params.set('url', sourceBundlePath(src))
  params.set('w', String(width))
  params.set('q', String(quality || 75))
  return '/_onreza/image?' + params.toString()
}
`
}

function writeImageLoader(projectDir, config) {
  const loaderPath = path.join(projectDir, ONREZA_IMAGE_LOADER_RELATIVE_PATH)
  fs.mkdirSync(path.dirname(loaderPath), { recursive: true })
  fs.writeFileSync(loaderPath, imageLoaderSource(config))
}

function pathnameMayMatchMiddleware(middleware, pathname) {
  if (!middleware) {
    return false
  }

  const matchers = middleware.config && middleware.config.matchers
  if (!Array.isArray(matchers) || matchers.length === 0) {
    return true
  }

  return matchers.some((matcher) => {
    if (!matcher || typeof matcher.sourceRegex !== 'string') {
      return true
    }
    try {
      return new RegExp(matcher.sourceRegex).test(pathname)
    } catch (_error) {
      return true
    }
  })
}

function collectPublicPathnames(projectDir) {
  const publicDir = path.join(projectDir, 'public')
  if (!fs.existsSync(publicDir)) {
    return []
  }

  const pathnames = []
  const stack = [publicDir]
  while (stack.length > 0) {
    const dir = stack.pop()
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const fullPath = path.join(dir, entry.name)
      if (entry.isSymbolicLink()) {
        continue
      }
      if (entry.isDirectory()) {
        stack.push(fullPath)
        continue
      }
      if (!entry.isFile()) {
        continue
      }
      const rel = path.relative(publicDir, fullPath).split(path.sep).join('/')
      pathnames.push(`/${rel}`)
    }
  }
  return pathnames.sort()
}

function partitionPathnamesByMiddleware(middleware, pathnames) {
  const safeForStaticLayer = []
  const requiresCompute = []

  for (const pathname of pathnames) {
    if (pathnameMayMatchMiddleware(middleware, pathname)) {
      requiresCompute.push(pathname)
    } else {
      safeForStaticLayer.push(pathname)
    }
  }

  return { safeForStaticLayer, requiresCompute }
}

function buildDeploymentHints(ctx) {
  const middleware = ctx.outputs && ctx.outputs.middleware
  const staticPathnames = Array.isArray(ctx.outputs && ctx.outputs.staticFiles)
    ? ctx.outputs.staticFiles
        .map((file) => file && file.pathname)
        .filter((pathname) => typeof pathname === 'string')
    : []

  const projectKey = path.resolve(ctx.projectDir || process.cwd())
  return {
    imageOptimizer: buildImageOptimizerHint(
      ctx.config,
      installedImageSourcesByProject.get(projectKey),
    ),
    middleware: {
      staticFiles: partitionPathnamesByMiddleware(middleware, staticPathnames),
      publicFiles: partitionPathnamesByMiddleware(
        middleware,
        collectPublicPathnames(ctx.projectDir),
      ),
    },
  }
}

function writeDescriptor(ctx) {
  const outputPath = path.join(ctx.projectDir, OUTPUT_RELATIVE_PATH)
  const payload = {
    version: 1,
    adapter: {
      name: ADAPTER_NAME,
      version: process.env.ONREZA_NEXT_ADAPTER_VERSION || null,
    },
    nextVersion: ctx.nextVersion,
    buildId: ctx.buildId,
    projectDir: ctx.projectDir,
    repoRoot: ctx.repoRoot,
    distDir: ctx.distDir,
    config: ctx.config,
    routing: ctx.routing,
    outputs: ctx.outputs,
    deploymentHints: buildDeploymentHints(ctx),
  }

  fs.mkdirSync(path.dirname(outputPath), { recursive: true })
  fs.writeFileSync(outputPath, `${JSON.stringify(payload, jsonReplacer, 2)}\n`)
}

function ensureStandaloneServerTraces(ctx) {
  if (!ctx.config || ctx.config.output !== 'standalone') {
    return
  }

  const distDir = path.isAbsolute(ctx.distDir)
    ? ctx.distDir
    : path.resolve(ctx.projectDir, ctx.distDir)
  fs.mkdirSync(distDir, { recursive: true })

  for (const fileName of STANDALONE_SERVER_TRACE_FILES) {
    const tracePath = path.join(distDir, fileName)
    if (fs.existsSync(tracePath)) {
      continue
    }

    // Next.js adapter endpoints already carry the additional traced modules,
    // but affected Next 16 releases suppress these whole-app files while the
    // standalone copier still reads them unconditionally (vercel/next.js#96646).
    // Materialize only the missing compatibility inputs and never replace a
    // trace emitted by Next itself.
    try {
      fs.writeFileSync(tracePath, '{"version":1,"files":[]}\n', {
        flag: 'wx',
      })
    } catch (error) {
      if (!error || error.code !== 'EEXIST') {
        throw error
      }
    }
  }
}

/** @type {import('next').NextAdapter} */
const adapter = {
  name: ADAPTER_NAME,

  modifyConfig(config, { phase, projectDir }) {
    if (phase !== 'phase-production-build') {
      return config
    }

    const nextConfig = {
      ...config,
    }

    if (nextConfig.output == null) {
      nextConfig.output = 'standalone'
    }

    const projectKey = path.resolve(projectDir || process.cwd())
    installedImageSourcesByProject.delete(projectKey)
    const imageDecision = imageOptimizerDecision(nextConfig)
    if (imageDecision.status === 'onreza_optimizer') {
      installedImageSourcesByProject.set(
        projectKey,
        imageDecision.remoteImageSources,
      )
      writeImageLoader(projectKey, nextConfig)
      nextConfig.images = {
        ...(nextConfig.images || {}),
        loader: 'custom',
        loaderFile: ONREZA_IMAGE_LOADER_RELATIVE_PATH,
        path: ONREZA_IMAGE_OPTIMIZER_PATH,
      }
    }

    return nextConfig
  },

  onBuildComplete(ctx) {
    ensureStandaloneServerTraces(ctx)
    writeDescriptor(ctx)
  },
}

module.exports = adapter
module.exports.__test = {
  buildRemoteImageSources,
  ensureStandaloneServerTraces,
  imageLoaderSource,
  imageOptimizerDecision,
}
