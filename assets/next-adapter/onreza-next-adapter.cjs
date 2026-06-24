const fs = require('fs')
const path = require('path')

const ADAPTER_NAME = '@onreza/nrz-next-adapter'
const OUTPUT_RELATIVE_PATH =
  process.env.ONREZA_NEXT_ADAPTER_OUTPUT || '.onreza/next-adapter-output.json'
const ONREZA_IMAGE_OPTIMIZER_PATH = '/_onreza/image'
const ONREZA_IMAGE_LOADER_RELATIVE_PATH =
  './.onreza/cache/next-adapter/onreza-image-loader.js'

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

function hasUserAssetPrefix(config) {
  if (!hasNonEmptyString(config.assetPrefix)) {
    return false
  }
  return config.assetPrefix !== config.basePath
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

  if (hasItems(images.domains) || hasItems(images.remotePatterns)) {
    return {
      status: 'compute_fallback',
      primitive: 'COMPUTE layer',
      reason: 'remote images require Next.js remote image validation/fetching',
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

  return {
    status: 'onreza_optimizer',
    mode: 'custom_loader',
    path: ONREZA_IMAGE_OPTIMIZER_PATH,
    primitive: 'ONREZA image optimizer',
    reason: null,
  }
}

function isOnrezaImageOptimizerConfig(config) {
  const images =
    config.images && typeof config.images === 'object' ? config.images : {}
  return (
    images.loader === 'custom' &&
    images.loaderFile === ONREZA_IMAGE_LOADER_RELATIVE_PATH
  )
}

function buildImageOptimizerHint(config) {
  if (isOnrezaImageOptimizerConfig(config)) {
    return {
      status: 'onreza_optimizer',
      mode: 'custom_loader',
      path: ONREZA_IMAGE_OPTIMIZER_PATH,
      primitive: 'ONREZA image optimizer',
      reason: null,
    }
  }
  return imageOptimizerDecision(config)
}

function imageLoaderSource(config) {
  const basePath =
    typeof config.basePath === 'string' && config.basePath !== '/'
      ? config.basePath
      : ''
  return `'use strict'

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

function onrezaImageLoader({ src, width, quality }) {
  const params = new URLSearchParams()
  params.set('url', sourceBundlePath(src))
  params.set('w', String(width))
  params.set('q', String(quality || 75))
  return '/_onreza/image?' + params.toString()
}

module.exports = onrezaImageLoader
module.exports.default = onrezaImageLoader
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

  return {
    imageOptimizer: buildImageOptimizerHint(ctx.config),
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

    const imageDecision = imageOptimizerDecision(nextConfig)
    if (imageDecision.status === 'onreza_optimizer') {
      writeImageLoader(projectDir || process.cwd(), nextConfig)
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
    writeDescriptor(ctx)
  },
}

module.exports = adapter
