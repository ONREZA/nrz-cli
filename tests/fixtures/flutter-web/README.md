# Flutter Web qualification fixture

The fixture uses Flutter 3.47.6 with its pinned engine and Dart 3.13.5.
`pub get --enforce-lockfile` followed by `build web --release --no-pub` produces
`build/web`. The ignored CLI fixture test verifies the real build, unchanged Pub
lock, bootstrap, JavaScript entry, and asset manifest. This does not qualify a
browser session or the complete Edge serving lifecycle.

Flutter's default URL strategy uses fragments: `/#/account` requests `/` from the
server. This fixture keeps that default. An application that selects
`usePathUrlStrategy()` needs an authored SPA rewrite. Existing EdgeIngress maps
`/account` to `account/index.html` and does not implicitly fall back to the root
index. Copy `edge-rules.example.toml` to the project root as `onreza.rules.toml`
to use the existing internal rewrite with file priority (`if_no_file = true`).
Existing assets retain their own paths. Apply this rule only to the intended SPA
route scope when a deployment also has other applications or API routes.

The CLI supplies `Cache-Control: no-cache` through the existing STATIC manifest
route when no case-insensitive Cache-Control header is authored. The policy is
preserved by SOURCE_BUNDLE verification; explicitly authored route headers and
USER Edge Rules retain their existing authority. The optional example also shows
an explicit Edge Rules override. Flutter emits unversioned
`flutter_bootstrap.js` and `main.dart.js`; existing EdgeIngress defaults assign
one-year immutable caching to JavaScript. At a stable domain, a later deployment
could otherwise reuse old JavaScript. The existing `set_headers` action sets
`cache-control = "no-cache"` so responses revalidate; this example conservatively
covers all GET/HEAD assets.

The default recipe builds for the domain root, with `<base href="/">`.
A subpath application requires an explicit build command with the appropriate
`--base-href` (ending in `/`) and matching authored routing. This fixture does not
claim subpath or PathUrlStrategy serving qualification.
