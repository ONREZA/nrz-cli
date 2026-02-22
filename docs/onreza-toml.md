# onreza.toml — справочник полей

Файл `onreza.toml` — единый конфиг проекта для ONREZA платформы. Коммитится в git (публичен), не содержит секретов.

Создаётся автоматически при `nrz init`. Для IDE-подсказок подключите JSON-схему (если редактор поддерживает TOML + JSON Schema):

```toml
# $schema = "./onreza.schema.json"  # пока не поддерживается стандартно
```

---

## [project]

Идентификация проекта на платформе.

| Поле | Тип | Обязателен | Описание |
|------|-----|-----------|---------|
| `id` | string | да (после init) | ID проекта на платформе вида `proj_abc123`. Прописывается автоматически при `nrz init` / `nrz link`. Используется во всех API-вызовах. |
| `name` | string | нет | Отображаемое имя проекта. Заполняется при `nrz init`. |
| `workspace` | string | нет | Slug воркспейса (организации). Заполняется при `nrz init`. |
| `framework` | string | нет | Slug обнаруженного фреймворка (например `next`, `nuxt`, `sveltekit`, `astro`, `vite`, `remix`). **Заполняется автоматически** командой `nrz detect --save` и при каждом `nrz deploy`. Не редактируйте вручную. |

**Пример:**
```toml
[project]
id = "proj_abc123"
name = "My App"
workspace = "myteam"
framework = "next"
```

---

## [dev]

Настройки локального dev-сервера (`nrz dev`).

| Поле | Тип | По умолчанию | Описание |
|------|-----|-------------|---------|
| `command` | string | авто | Команда запуска dev-сервера фреймворка. Если не задана, `nrz dev` пытается определить её автоматически из `package.json`. Пример: `"npm run dev"`, `"bun dev"`. |
| `port` | integer | `4321` | Порт HTTP-сервера эмулятора ONREZA (KV, D1 endpoints). **Не** порт вашего фреймворка — фреймворк запускается на своём порту. |
| `host` | string | `"127.0.0.1"` | Bind-адрес для эмулятора. Изменяйте только если эмулятор нужен снаружи (например в Docker). |
| `data_dir` | string | `".onreza/data"` | Папка для локальных данных (SQLite файл D1, KV персистенция). Создаётся автоматически. |
| `db_name` | string | `"dev.db"` | Имя SQLite-файла внутри `data_dir` для D1-эмуляции. |
| `aliases` | object | `{}` | Именованные профили команд для `nrz dev --alias <name>`. Ключ — имя алиаса, значение — команда. |

**Пример:**
```toml
[dev]
command = "npm run dev"
port = 4321

[dev.aliases]
worker = "node src/worker.js"
debug = "node --inspect src/index.js"
```

**Как работает `nrz dev`:**
1. Поднимает эмулятор (HTTP-сервер на `host:port`)
2. Генерирует JS bootstrap-скрипт, прокидывающий `globalThis.ONREZA`
3. Запускает `command` с `NODE_OPTIONS=--import <bootstrap>`
4. Ctrl+C → graceful shutdown всего

---

## [build]

Настройки сборки проекта.

| Поле | Тип | По умолчанию | Описание |
|------|-----|-------------|---------|
| `command` | string | нет | Команда сборки, которая выполняется автоматически перед `nrz deploy` (если не передан `--skip-build`). Пример: `"npm run build"`. |
| `output_dirs` | string[] | см. ниже | Список директорий, в которых CLI ищет build output. Порядок важен — берётся первая существующая. |

**Дефолтный `output_dirs`:**
```toml
output_dirs = ["dist", ".output", "build", "out", "_site", "www", ".vitepress/dist"]
```

Для фреймворков с нестандартными путями (`.next`, `.svelte-kit`) CLI использует дополнительную логику на основе детекции — поэтому ручное переопределение нужно редко.

**Пример:**
```toml
[build]
command = "npm run build"
output_dirs = ["dist"]
```

---

## [deploy]

Настройки деплоя на платформу (`nrz deploy`).

| Поле | Тип | По умолчанию | Описание |
|------|-----|-------------|---------|
| `skip_migrations` | boolean | `false` | Пропустить применение D1-миграций при деплое. Полезно если миграции применяются отдельным CI-шагом. |
| `compute` | string | авто | Принудительно задать compute type вместо авто-определения. Значения: `"static"`, `"isolate"`, `"process"`. Используйте только если авто-определение даёт неверный результат. |
| `entry` | string | авто | Точка входа для `PROCESS`-деплоев (Node.js/Bun сервер). Должна быть относительным путём без `..`. Пример: `"server.ts"`, `"dist/index.js"`. Если не задана, CLI определяет автоматически. |

**Compute types:**

| Тип | Когда использовать |
|-----|--------------------|
| `static` | Статические сайты без серверного кода (Vite, CRA, Astro static) |
| `isolate` | Edge-функции через @onreza адаптер (Next.js Edge, Nuxt с адаптером) |
| `process` | Полноценный Node.js/Bun сервер (Next.js standalone, Hono, Elysia, кастомный сервер) |

`.onreza/manifest.json` поддерживается только для `isolate`:
- `compute = "isolate"` требует manifest в build output
- `compute = "process"` и `compute = "static"` выполняются без manifest

**Приоритет entry point для PROCESS:**
`[deploy] entry` > авто-определение по фреймворку > `package.json "main"/"module"` > `scripts.start/serve/...` > `index.*` (Bun default) > heuristic scan по build output

Если найдено несколько одинаково подходящих кандидатов, деплой завершится ошибкой и попросит явно задать `[deploy] entry`.

**Пример:**
```toml
[deploy]
skip_migrations = false
compute = "process"
entry = "dist/server.js"
```

---

## [migrations]

Настройки D1 миграций.

| Поле | Тип | По умолчанию | Описание |
|------|-----|-------------|---------|
| `dir` | string | `"migrations"` | Папка с SQL-файлами миграций. CLI сканирует её по паттерну `NNNN_*.sql`. |

**Пример:**
```toml
[migrations]
dir = "db/migrations"
```

---

## [db]

Настройки команд работы с базой данных.

| Поле | Тип | По умолчанию | Описание |
|------|-----|-------------|---------|
| `default_env` | string | нет | Environment по умолчанию для remote DB команд (`nrz db shell --remote`, `nrz db push`). Значения: `"production"`, `"preview"`, `"development"`. Если не задан, CLI спрашивает интерактивно. |

**Пример:**
```toml
[db]
default_env = "development"
```

---

## [env]

Декларация переменных окружения проекта.

### [env] — настройки

| Поле | Тип | По умолчанию | Описание |
|------|-----|-------------|---------|
| `strict` | boolean | `false` | Режим строгой загрузки: `nrz env push` загружает только переменные, объявленные в `[env.declarations]`. Аналог флага `--declared-only`. |

### [env.declarations] — переменные

Объявляет какие env vars нужны проекту, их видимость и обязательность. Используется в:
- `nrz env push` — определяет `is_secret` без эвристики по имени
- `nrz env validate` — проверяет что все required переменные заданы
- `nrz deploy` — pre-flight проверка перед деплоем (можно отключить `--skip-env-check`)

**Два формата объявления:**

```toml
[env.declarations]
# Shorthand — переменная обязательна, задаётся тип видимости
DATABASE_URL = "sensitive"   # зашифруется на платформе
PUBLIC_API_URL = "plain"     # хранится открыто

# Full form — для необязательных переменных
OPTIONAL_FEATURE_FLAG = { visibility = "plain", required = false }
ANALYTICS_KEY = { visibility = "sensitive", required = false }
```

| Поле объявления | Значения | Описание |
|-----------------|----------|---------|
| shorthand | `"sensitive"` / `"plain"` | Обязательная переменная с заданной видимостью |
| `visibility` | `"sensitive"` / `"plain"` | Тип хранения на платформе |
| `required` | `true` / `false` | Нужна ли переменная перед деплоем (default: `true`) |

**Visibility:**
- `sensitive` — значение шифруется на платформе, не отображается в UI (для паролей, токенов, ключей)
- `plain` — хранится открыто, видно в UI (для публичных URL, флагов, несекретных настроек)

**Пример полного [env]:**
```toml
[env]
strict = true

[env.declarations]
DATABASE_URL = "sensitive"
JWT_SECRET = "sensitive"
PUBLIC_API_URL = "plain"
NEXT_PUBLIC_APP_NAME = "plain"
SENTRY_DSN = { visibility = "sensitive", required = false }
```

---

## Полный пример onreza.toml

```toml
[project]
id = "proj_abc123"
name = "My Next.js App"
workspace = "myteam"
# framework = "next"  # заполняется автоматически

[dev]
command = "npm run dev"
port = 4321

[dev.aliases]
debug = "node --inspect node_modules/.bin/next dev"

[build]
command = "npm run build"

[deploy]
skip_migrations = false
# compute = "process"  # только если авто-определение неверно
# entry = "dist/server.js"

[migrations]
dir = "migrations"

[db]
default_env = "development"

[env]
strict = true

[env.declarations]
DATABASE_URL = "sensitive"
NEXTAUTH_SECRET = "sensitive"
NEXTAUTH_URL = "plain"
NEXT_PUBLIC_API_URL = "plain"
SENTRY_DSN = { visibility = "sensitive", required = false }
```

---

## Что заполняется автоматически

| Поле | Команда |
|------|---------|
| `project.id` | `nrz init`, `nrz link` |
| `project.name` | `nrz init` |
| `project.workspace` | `nrz init` |
| `project.framework` | `nrz detect --save`, `nrz deploy` |

## Приоритет настроек

```
CLI flag > env var (NRZ_*) > onreza.toml > hardcoded default
```

Например, `--env production` переопределяет `[db] default_env`, `NRZ_TOKEN` переопределяет любой сохранённый токен.

## Локальные файлы (.onreza/)

Не путайте `onreza.toml` (в git) с `.onreza/` (gitignored):

| Файл | Назначение |
|------|-----------|
| `.onreza/data/dev.db` | SQLite для D1 эмуляции |
| `.onreza/data/kv.json` | Персистенция KV store |
| `.onreza/environment.json` | Личный выбор environment разработчика |
