# nrz-cli: Framework Detection Roadmap

> Дата: 2026-03-05
> Контекст: увеличить объём, надёжность и качество детекта фреймворков

---

## Текущее состояние

27 пресетов, 6 SSR-анализаторов, auto-manifest для Next.js standalone + Nuxt + SvelteKit + Remix + React Router v7 + Astro SSR + generic static/compute.

| Tier | Фреймворки | SSR-анализ | Auto-manifest |
|------|-----------|------------|---------------|
| 1 (core) | Next.js, Nuxt, SvelteKit, React Router v7, Remix, Gatsby | да (6 шт) | **Next.js standalone, Nuxt, SvelteKit, Remix, RR v7, Astro SSR** |
| 2 (CLI) | CRA, Vue CLI, Angular, Preact CLI | нет | static fallback |
| 3a (SSG) | Astro, Docusaurus, VitePress, Eleventy, Hexo, Parcel, Stencil | Astro only | static fallback / **Astro SSR** |
| 3b (Server) | Hono, Elysia, Express, Fastify, NestJS, Koa, AdonisJS, H3, Nitro | нет (always PROCESS) | generic compute |
| 4 (catch-all) | Vite, Other, Static HTML | нет | static/compute generic |

---

## P0 — Серверные фреймворки: неправильная категоризация ✅ DONE

> Реализовано в `29809e7` — Express, Fastify, NestJS, Koa, AdonisJS, H3, Nitro.
> Пресеты, `is_server_framework()`, entry points, output dirs, тесты.

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

## P2 — Auto-manifest для SSR-фреймворков ✅ DONE

> Реализовано в `9b8a034`.
>
> Генераторы: `generate_nuxt_manifest`, `generate_sveltekit_manifest`, `generate_remix_manifest`, `generate_astro_ssr_manifest`.
> Общая логика вынесена в `SsrManifestConfig` + `generate_ssr_manifest()`.
> Интеграция через `try_generate_ssr_manifest()` в `run_with_hint()`.
> Также: `compute_aware_output_dirs` для Nuxt static (.output/public) и Remix/RR SSR (build root).
> Tracing при missing entry point. 8 validate-тестов + 17 integration-тестов.

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
