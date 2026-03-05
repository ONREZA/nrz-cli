# nrz-cli: Framework Detection Roadmap

> Дата: 2026-03-05
> Контекст: увеличить объём, надёжность и качество детекта фреймворков

---

## Текущее состояние

20 пресетов, 6 SSR-анализаторов, auto-manifest только для Next.js standalone + generic static/compute.

| Tier | Фреймворки | SSR-анализ | Auto-manifest |
|------|-----------|------------|---------------|
| 1 (core) | Next.js, Nuxt, SvelteKit, React Router v7, Remix, Gatsby | да (6 шт) | Next.js standalone only |
| 2 (CLI) | CRA, Vue CLI, Angular, Preact CLI | нет | static fallback |
| 3a (SSG) | Astro, Docusaurus, VitePress, Eleventy, Hexo, Parcel, Stencil | Astro only | static fallback |
| 3b (Server) | Hono, Elysia | нет (always PROCESS) | generic compute |
| 4 (catch-all) | Vite, Other, Static HTML | нет | static/compute generic |

---

## P0 — Серверные фреймворки: неправильная категоризация (критично)

**Проблема:** `is_server_framework()` знает только Hono и Elysia. Express/Fastify/NestJS/Koa и другие серверные фреймворки попадают в `other` и получают `ComputeType::Static` (если нет явного `start` скрипта). Пользователь деплоит Express-app — получает STATIC. Это ломает деплой.

### P0.1 — Добавить пресеты серверных фреймворков

Новые пресеты в `presets.rs` (category: `Server`, priority: 30-39):

| slug | name | dependencies | output_directory | runtime |
|------|------|-------------|-----------------|---------|
| `express` | Express | `express` | `.` | Node |
| `fastify` | Fastify | `fastify` | `.` | Node |
| `nestjs` | NestJS | `@nestjs/core` | `dist` | Node |
| `koa` | Koa | `koa` | `.` | Node |
| `adonis` | AdonisJS | `@adonisjs/core` | `build` | Node |
| `h3` | H3 | `h3` | `.` | Node |
| `nitro` | Nitro (standalone) | `nitropack` | `.output` | Node |

**Важно:** `express` имеет priority ниже чем SSR-фреймворки (Next.js, Nuxt и т.д.), которые тоже зависят от express/connect. SSR-фреймворки ловятся раньше по priority.

### P0.2 — Расширить `is_server_framework()`

Добавить все новые серверные slug в `is_server_framework()` — это гарантирует `ComputeType::Process` без SSR-анализа.

### P0.3 — Entry points для серверных фреймворков

Обновить `framework_entry_point()`:

| slug | entry |
|------|-------|
| `nestjs` | `main.js` |
| `adonis` | `server.js` |
| `nitro` | `server/index.mjs` |

Express/Fastify/Koa/H3 — entry определяется через `main`/`module`/scripts (эвристика уже работает).

### P0.4 — Config files и DETECTION_CONTENT_FILES

Ничего не нужно добавлять — серверные фреймворки не имеют специфичных config-файлов для анализа.

### P0.5 — `framework_output_dirs()`

Добавить маппинги для новых фреймворков:

| slug | dirs |
|------|------|
| `express` | `.` |
| `fastify` | `.`, `dist` |
| `nestjs` | `dist` |
| `koa` | `.` |
| `adonis` | `build` |
| `h3` | `.`, `dist` |
| `nitro` | `.output` |

### P0.6 — Тесты

- Юнит-тесты в `presets_tests.rs`: каждый новый пресет правильно матчится
- Тесты в `mod_tests.rs`: серверный фреймворк → `ComputeType::Process`
- Тесты на priority: Express не перебивает Next.js/Nuxt
- Тест: Express без `start` скрипта → всё равно PROCESS (а не STATIC)

---

## P1 — Мета-фреймворки с SSR

**Проблема:** SolidStart, QwikCity, Analog — SSR-фреймворки нового поколения. Растущая аудитория, zero-config деплой на конкурирующих PaaS.

### P1.1 — Новые пресеты

| slug | name | dependencies | output_directory | category | priority |
|------|------|-------------|-----------------|----------|----------|
| `solidstart` | SolidStart | `@solidjs/start` | `.output` | Other | 7 |
| `qwik` | Qwik City | `@builder.io/qwik-city` | `dist`, `server` | Other | 8 |
| `analog` | Analog | `@analogjs/platform` | `.output`, `dist/analog` | Other | 9 |

### P1.2 — SSR-анализ для новых фреймворков

- **SolidStart:** проверка `app.config.ts` на `ssr: false`, наличие `src/routes/api/`
- **QwikCity:** проверка на `@qwik.dev/router` plugin, `src/routes/` структура
- **Analog:** проверка `vite.config.ts` на `analog()` plugin, `src/server/routes/`

### P1.3 — Обновить `is_ssr_framework()`, `DETECTION_CONTENT_FILES`

---

## P2 — Auto-manifest для SSR-фреймворков

**Проблема:** все PROCESS-фреймворки кроме Next.js standalone требуют ручного `.onreza/manifest.json` или полагаются на эвристику entry point. Это ломает zero-config цель.

### P2.1 — `generate_nuxt_manifest()`

Nuxt `.output/` структура:
- STATIC layer: `.output/public/` (static assets, `_nuxt/`)
- COMPUTE layer: `.output/server/` (entry: `index.mjs`)
- Routes: `^/_nuxt/.*$` → static (priority 100), `^/.*$` → server (priority 0)

### P2.2 — `generate_sveltekit_manifest()`

SvelteKit с adapter-node:
- STATIC layer: `build/client/` (immutable assets)
- COMPUTE layer: `build/` (entry: `index.js`)
- Routes: `^/_app/.*$` → static (priority 100), `^/.*$` → server (priority 0)

### P2.3 — `generate_remix_manifest()` / `generate_react_router_manifest()`

Remix/React Router v7:
- STATIC layer: `build/client/` (assets)
- COMPUTE layer: `build/server/` (entry: `index.js`)
- Routes: `^/assets/.*$` → static (priority 100), `^/.*$` → server (priority 0)

### P2.4 — `generate_astro_ssr_manifest()`

Astro SSR (output: 'server' / 'hybrid'):
- STATIC layer: `dist/client/` (static assets)
- COMPUTE layer: `dist/server/` (entry: `entry.mjs`)

### P2.5 — Интеграция в `build/mod.rs`

Добавить ветки в `run_with_hint()` после Next.js standalone:
- Nuxt: `.output/server/index.mjs` exists → auto-manifest
- SvelteKit: `build/index.js` exists + adapter-node → auto-manifest
- Remix/RR: `build/server/index.js` exists → auto-manifest
- Astro SSR: `dist/server/entry.mjs` exists → auto-manifest

---

## P3 — Расширение экосистемы

### P3.1 — Deno-проекты

- Парсинг `deno.json` / `deno.jsonc` как альтернатива `package.json`
- Fresh framework detection (Deno runtime)
- Lume static site generator

### P3.2 — Улучшение monorepo support

- Парсинг `pnpm-workspace.yaml`
- Определение конкретного workspace для деплоя
- Turborepo/Nx awareness (turbo.json, nx.json)
- `--app` / `--filter` CLI флаг

### P3.3 — Python-проекты (exploratory)

- `pyproject.toml` / `requirements.txt` detection
- Django, FastAPI, Flask пресеты
- Определение WSGI/ASGI entry point

### P3.4 — Улучшение SSR-анализа

- Обработка программных конфигов с переменными окружения
- Более точный парсинг template-literal конфигов
- Поддержка `svelte.config.ts` (SvelteKit TS-конфиг)
- `remix.config.js` (legacy Remix v1)

### P3.5 — Дополнительные пресеты

- RedwoodJS (`@redwoodjs/core`)
- Blitz.js (`blitz`)
- Payload CMS (`payload`)
- Keystone (`@keystone-6/core`)
- Strapi (`@strapi/strapi`)

---

## Что НЕ делать

- Не добавлять адаптеры (STRATEGY_2026_Q2: вся логика сборки в CLI)
- Не усложнять runtime SDK
- Не строить полноценный JS/TS парсер для конфигов — string matching достаточно
- Не добавлять Python/Deno пока нет product decision
