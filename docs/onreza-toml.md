# onreza.toml — справочник полей

Файл `onreza.toml` — единый конфиг проекта для ONREZA платформы. Коммитится в git (публичен), не содержит секретов.

Создаётся автоматически при `nrz init`. Для IDE-подсказок `nrz init` добавляет директиву схемы в начало файла (если редактор поддерживает TOML + JSON Schema):

```toml
#:schema https://docs.onreza.ru/schemas/onreza-project-v1.schema.json
```

---

## [project]

Идентификация проекта на платформе.

| Поле | Тип | Обязателен | Описание |
|------|-----|-----------|---------|
| `id` | string | да (после init) | ID проекта на платформе вида `proj_abc123`. Прописывается автоматически при `nrz init` / `nrz link`. Используется во всех API-вызовах. |
| `name` | string | нет | Отображаемое имя проекта. Заполняется при `nrz init`. |
| `workspace` | string | нет | Slug воркспейса (организации). Заполняется при `nrz init`. |
| `framework` | string | нет | Slug фреймворка (например `next`, `nuxt`, `sveltekit`, `astro`, `vite`, `remix`). Заполняется командой `nrz detect --save` и используется как локальный override для build/deploy. |

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
| `port` | integer | `4321` | Порт HTTP-сервера эмулятора ONREZA (KV, DB endpoints). **Не** порт вашего фреймворка — фреймворк запускается на своём порту. |
| `host` | string | `"127.0.0.1"` | Bind-адрес для эмулятора. Изменяйте только если эмулятор нужен снаружи (например в Docker). |
| `data_dir` | string | `".onreza/data"` | Папка для локальных данных (SQLite, KV персистенция). Создаётся автоматически. |
| `db_name` | string | `"dev.db"` | Имя SQLite-файла внутри `data_dir` для локальной эмуляции. |
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
| `install_command` | string | авто | Команда установки зависимостей перед build/deploy. Если не задана, определяется по package manager. Пустая строка означает явный skip. |
| `command` | string | нет | Команда сборки, которая выполняется автоматически перед `nrz deploy` (если не передан `--skip-build`). Пример: `"npm run build"`. |
| `output_directory` | string | авто | Единственная авторитетная директория build output. Если задана в `onreza.toml`, CLI не делает silent fallback в другие директории. Compatibility alias: `output_dir`. |
| `output_dirs` | string[] | см. ниже | Список директорий, в которых CLI ищет build output. Порядок важен — берётся первая существующая. |

**Дефолтный `output_dirs`:**
```toml
output_dirs = ["dist", ".output", "build", "out", "_site", "www", ".vitepress/dist"]
```

Для фреймворков с нестандартными путями (`.next`, `.svelte-kit`) CLI использует дополнительную логику на основе детекции — поэтому ручное переопределение нужно редко.

**Пример:**
```toml
[build]
install_command = "pnpm install"
command = "npm run build"
output_directory = "dist"
output_dirs = ["dist"]
```

---

## [deploy]

Настройки деплоя на платформу (`nrz deploy`).

| Поле | Тип | По умолчанию | Описание |
|------|-----|-------------|---------|
| `compute` | string | авто | Принудительно задать compute type вместо авто-определения. Значения: `"static"`, `"process"`. Используйте только если авто-определение даёт неверный результат. |
| `entry` | string | авто | Точка входа для `PROCESS`-деплоев (Node.js/Bun сервер). Должна быть относительным путём без `..`, не shell-командой. Пример: `"server.ts"`, `"dist/index.js"`. Compatibility alias: `entrypoint`. Если не задана, CLI определяет автоматически. |
| `app` | string | нет | Монорепо: какой workspace/пакет деплоить. Матчится по имени пакета из `package.json`, имени директории, или относительному пути. Эквивалент CLI флага `--app` / `--filter`. |

Для `nrz deploy --app web` CLI сначала выбирает workspace из root config, затем
строит финальный effective config для директории app. Если в app есть свой
`onreza.toml`, его поля переопределяют root config, а root project identity
(`project.id`/`name`/`workspace`) остается fallback. Проверить итоговое решение:
`nrz config explain --app web --json`. По умолчанию `config explain` также
подтягивает server project settings для `project.id`, как `nrz deploy`; для
локального-only просмотра используйте `nrz config explain --local`.

**Compute types:**

| Тип | Когда использовать |
|-----|--------------------|
| `static` | Статические сайты без серверного кода (Vite, CRA, Astro static) |
| `process` | Полноценный Node.js/Bun сервер (Next.js standalone, Hono, Elysia, кастомный сервер) |

Матрица приоритетов `frameworkPreset`/`compute`/`outputDirectory` описана в
[output-directory-contract.md](./output-directory-contract.md). Ключевое правило:
пользовательский `outputDirectory` из server settings является authoritative и
не допускает silent fallback; preset/default значения могут уточняться SSR-анализом.

`compute = "process"` и `compute = "static"` выполняются без `.onreza/manifest.json`.

**Приоритет entry point для PROCESS:**
`[deploy] entry` > авто-определение по фреймворку > `package.json "main"/"module"` > `scripts.start/serve/...` > `index.*` (Bun default) > heuristic scan по build output

Если entry не удалось определить однозначно:
- для strict-фреймворков (`nextjs`, `nuxt`) деплой завершается ошибкой с actionable подсказкой
- для остальных деплой тоже завершается ошибкой с просьбой явно задать `[deploy] entry`

CLI не патчит `package.json` в build output для PROCESS. Резолвленный entry передаётся в deployment metadata (`processEntry`) как явная команда запуска. Если entry не найден или найден неоднозначно, деплой останавливается до отправки runtime metadata.

Для `Next.js` в `compute = "process"` требуется runnable standalone output:
- должен существовать `server.js` в корне выбранного output dir (обычно `.next/standalone/server.js`)
- если standalone output невалиден/отсутствует, деплой завершается ошибкой (без fallback в `.next`)

**Пример:**
```toml
[deploy]
compute = "process"
entry = "dist/server.js"
app = "web"  # для монорепо — какой пакет деплоить
```

---

## [db]

Настройки managed PostgreSQL команд и локального `nrz dev` DB injection.

| Поле | Тип | По умолчанию | Описание |
|------|-----|-------------|---------|
| `database` | string | авто | Managed database ID или name. Если не задано, CLI выбирает auto-inject DB или первую доступную DB проекта. |
| `branch` | string | main | Branch для `nrz dev` DB injection. CLI DB-команды также принимают `--branch`, где это поддерживается. |

**Пример:**
```toml
[db]
database = "primary"
branch = "dev"
```

---

## [env]

Декларация переменных окружения проекта.

### [env.declarations] — переменные

Объявляет какие env vars нужны проекту, их видимость и обязательность. Используется в:
- `nrz env validate` — проверяет один материализованный Environment snapshot
- `nrz deploy` — проверяет материализованный snapshot перед сборкой (можно отключить `--skip-env-check`)

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
# compute = "process"  # только если авто-определение неверно
# entry = "dist/server.js"

[db]
database = "primary"
branch = "dev"

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
| `project.framework` | `nrz detect --save` |

Если нужен только локальный scaffold без создания или линковки platform project,
используйте `nrz init --local`. Это явный путь перед `nrz detect --save`, когда
`onreza.toml` еще отсутствует.

## Приоритет настроек

```
CLI flag > env var (NRZ_*) > onreza.toml > hardcoded default
```

Например, `--environment production` у `nrz deploy` выбирает точный platform Environment, `NRZ_TOKEN` переопределяет любой сохранённый токен. Если флаг не задан, используется `NRZ_ENVIRONMENT`, затем выбор из `.onreza/environment.json`.

## Локальные файлы (.onreza/)

Не путайте `onreza.toml` (в git) с `.onreza/` (gitignored):

| Файл | Назначение |
|------|-----------|
| `.onreza/data/dev.db` | SQLite для локальной эмуляции |
| `.onreza/data/kv.<env>.json` | Персистенция local KV store по environment namespace |
| `.onreza/environment.json` | Личный выбор environment разработчика |
