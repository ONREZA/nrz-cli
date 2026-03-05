# nrz-cli: Framework Detection Roadmap

> Дата: 2026-03-05
> Контекст: увеличить объём, надёжность и качество детекта фреймворков

---

## Текущее состояние

36 пресетов, 9 SSR-анализаторов (с улучшенным парсингом конфигов), auto-manifest для Next.js standalone + Nuxt + SvelteKit + Remix + React Router v7 + Astro SSR + generic static/compute.

| Tier | Фреймворки | SSR-анализ | Auto-manifest |
|------|-----------|------------|---------------|
| 0 (wrappers) | Blitz.js, Keystone | нет (always PROCESS) | generic compute |
| 1 (core) | Next.js, Nuxt, SvelteKit, React Router v7, Remix, Gatsby | да (6 шт) | **Next.js standalone, Nuxt, SvelteKit, Remix, RR v7, Astro SSR** |
| 1b (next-gen) | SolidStart, Qwik City, Analog | да (3 шт) | — |
| 2 (CLI) | CRA, Vue CLI, Angular, Preact CLI | нет | static fallback |
| 3a (SSG) | Astro, Docusaurus, VitePress, Eleventy, Hexo, Parcel, Stencil | Astro only | static fallback / **Astro SSR** |
| 3b (Server) | Hono, Elysia, Express, Fastify, NestJS, Koa, AdonisJS, H3, Nitro | нет (always PROCESS) | generic compute |
| 3c (Full-stack/CMS) | RedwoodJS, Payload CMS, Strapi | нет (always PROCESS) | generic compute |
| 4 (catch-all) | Vite, Other, Static HTML | нет | static/compute generic |

---

## P0 — Серверные фреймворки: неправильная категоризация ✅ DONE

> Реализовано в `29809e7` — Express, Fastify, NestJS, Koa, AdonisJS, H3, Nitro.
> Пресеты, `is_server_framework()`, entry points, output dirs, тесты.

---

## P1 — Мета-фреймворки с SSR ✅ DONE

> Реализовано: SolidStart, Qwik City, Analog.
> Пресеты (приоритеты 7-9), SSR-анализаторы, `is_ssr_framework()`,
> `DETECTION_CONTENT_FILES`, `detect_config_files()`, `framework_entry_point()`,
> `framework_output_dirs()`. 20+ тестов.

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

### P3.4 — Улучшение SSR-анализа ✅ DONE

> - `contains_value`: поддержка backtick-кавычек (`` `value` ``)
> - `contains_value`: обработка env variable fallbacks (`process.env.X || 'value'`, `?? 'value'`)
> - `analyze_sveltekit`: добавлена поддержка `svelte.config.ts`
> - `analyze_remix`: legacy `remix.config.js` для Remix v1 (`ssr: false`)
> - `detect_config_files`, `DETECTION_CONTENT_FILES` обновлены
> - 8 новых тестов

### P3.5 — Дополнительные пресеты ✅ DONE

> 5 новых пресетов (36 итого):
> - Blitz.js (`blitz`, priority 1, `.next`) — перед Next.js т.к. оборачивает его
> - Keystone (`@keystone-6/core`, priority 1, `.keystone`) — перед Next.js т.к. часто идёт с ним
> - RedwoodJS (`@redwoodjs/core`, priority 39, `api/dist`)
> - Payload CMS (`payload`, priority 40, `dist`)
> - Strapi (`@strapi/strapi`, priority 41, `dist`)
> - Все 5 добавлены в `is_server_framework()` (всегда PROCESS)
> - `framework_output_dirs`, `framework_entry_point` обновлены
> - 15+ новых тестов

---

## Что НЕ делать

- Не добавлять адаптеры (STRATEGY_2026_Q2: вся логика сборки в CLI)
- Не усложнять runtime SDK
- Не строить полноценный JS/TS парсер для конфигов — string matching достаточно
- Не добавлять Python/Deno пока нет product decision
