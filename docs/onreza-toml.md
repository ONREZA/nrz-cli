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

Hugo автоматически определяется по `hugo.toml`, `hugo.yaml` или `hugo.json`
в корне либо в `config/_default`. Для проекта с `config.toml`, `config.yaml`
или `config.json` в этих каталогах укажите `framework = "hugo"`: общее имя конфигурации
само по себе не определяет генератор. Hugo STATIC может содержать только
вложенные страницы без корневого `index.html`.

В автоматической Go-сборке ищется один `main` package в корне или `cmd/*`,
подходящий для Linux amd64, `CGO=0` и выбранного compiler. Учитываются суффиксы
имён файлов и build constraints; Windows-only и cgo-only entry не выбираются.

Автосборка Hugo требует неизменных dependency inputs: `go.mod`, `go.sum` и
`hugo.direct.sum`, если они нужны вашим модулям. Если Hugo разрешает новые
зависимости или обновляет эти файлы, публикация прерывается без изменения
исходников. Сначала соберите сайт локально, проверьте и закоммитьте полученные
файлы зависимостей. Go workspaces и источники модулей вне проекта требуют
собственной `build.command`.

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
| `toolchain` | string | авто | Инструменты сборки: `"node"`, `"bun"`, `"python"` или `"native"`. Выбор не задаёт launcher приложения. |
| `python_version` | string | serving minor или `"3.14"` | Minor Python для compiler/installer: `"3.12"`, `"3.13"`, `"3.14"`. Без `toolchain` подразумевает `"python"`. |
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
| `runtime` | string | авто | Launcher приложения: `"node"`, `"bun"`, `"python"` или `"executable"`. Язык сборки выбирается отдельно. |
| `python_version` | string | Python build minor или `"3.14"` | Minor CPython для `PROCESS` launcher: `"3.12"`, `"3.13"` или `"3.14"`. |
| `entry` | string | авто | Файл запуска `PROCESS`: JS/TS, Python script или готовый executable. Относительный путь без `..`, не shell-команда. Compatibility alias: `entrypoint`. |
| `module` | string | нет | Python import module для запуска как `python -m`, например `"company.worker"`. |
| `application` | string | нет | Python import reference, например `"api.main:app"` или `"api.main:create_app()"`. |
| `server` | string | авто | Python server: `"asgi"` / `"uvicorn"` либо `"wsgi"` / `"gunicorn"`. Зависимость сервера должна быть объявлена в проекте. |
| `args` | string[] | `[]` | Буквальные аргументы приложения после точки запуска. |
| `app` | string | нет | Монорепо: какой workspace/пакет деплоить. Матчится по имени пакета из `package.json`, имени директории, или относительному пути. Эквивалент CLI флага `--app` / `--filter`. |

Для `nrz deploy --app web` CLI сначала выбирает workspace из root config, затем
строит финальный effective config для директории app. Если в app есть свой
`onreza.toml`, его поля переопределяют root config, а root project identity
(`project.id`/`name`/`workspace`) остается fallback. Проверить итоговое решение:
`nrz config explain --app web --json`. По умолчанию `config explain` также
подтягивает server project settings для `project.id`, как `nrz deploy`; для
локального-only просмотра используйте `nrz config explain --local`.

Для Python поля `entry`, `module` и `application` задают разные режимы запуска.
Явный режим дочернего приложения заменяет конкурирующие настройки родителя.
`server` наследуется в режиме application; явный `server` без `application`
выбирает application inference вместо родительского script/module.
Конфликтующие поля самого дочернего приложения отклоняются.

Явный запуск имеет приоритет над автодетектом. Единственный console script
из `pyproject.toml` запускается как callable, даже при наличии веб-фреймворка.
Объявленные console scripts сохраняют выбор Python рядом с Go/Dart helper или
JavaScript tooling; несколько scripts требуют явного выбора application.
Если зависимости содержат условия, включения других requirements-файлов
или формируются package backend (`setup.py` либо dynamic dependencies в
`pyproject.toml`),
задайте `application` и `server` явно: этих данных недостаточно для надёжного
автоматического выбора ASGI/WSGI. Установщик обрабатывает authored dependencies.
Для script/module укажите `entry`/`module`; для callable без ASGI/WSGI задайте
`project.framework = "python"` и `deploy.application`.

**Compute types:**

| Тип | Когда использовать |
|-----|--------------------|
| `static` | Статические сайты без серверного кода (Vite, CRA, Astro static) |
| `process` | Сервер на Node.js, Bun, CPython или готовый Linux executable |

Матрица приоритетов `frameworkPreset`/`compute`/`outputDirectory` описана в
[output-directory-contract.md](./output-directory-contract.md). Ключевое правило:
пользовательский `outputDirectory` из server settings является authoritative и
не допускает silent fallback; preset/default значения могут уточняться SSR-анализом.

`compute = "process"` и `compute = "static"` выполняются без `.onreza/manifest.json`.

**Приоритет entry point для JS/TS PROCESS:**
`[deploy] entry` > файл из прямого `scripts.start` с Bun/Node > авто-определение по фреймворку > `package.json "main"/"module"` > остальные script hints > `index.*` > heuristic scan по build output

Приложение выбирает launcher до install/build: `[deploy] runtime = "bun"` или
`"node"`, либо прямой `scripts.start` вида `bun [run] file` / `node file`.
Package manager, lockfile и install/build command выбирают инструменты сборки,
но не launcher приложения. `main`/`module` не вытесняют прямой start.
`[deploy] args` — буквальные аргументы после entry, а не флаги интерпретатора.

```toml
[deploy]
runtime = "bun"
entry = "dist/server.js"
args = ["--port", "8080"]
```

Полная декларация runtime/entry/args задаёт канонический запуск и заменяет
неподдерживаемый shell/loader start. Известный конфликт Bun/Node между
декларацией и прямым launcher, конфликт с framework или build manifest
завершается ошибкой. Без полной декларации shell chains, interpreter flags и
script aliases в прямом Bun/Node start отвергаются до установки и сборки.
Максимум — 64 аргумента, каждый до 4096 UTF-8 bytes без NUL; entry — относительный
путь до 4096 UTF-8 bytes без traversal.

Для локального Node deploy CLI после admission читает frozen NodeVersion и
проверяет actual local Node major до install/build. Bun проверяется по pinned
CLI toolchain. Platform runner проверяет frozen intent и trusted runtime target.
`SOURCE_BUNDLE_V1` сохраняет family/args и `buildRuntimeVersion`; materializer и
graph compiler отвергают несовместимые family/target, в том числе без dependencies.
Соседний слой может сохранять другой поддерживаемый target без дерева
зависимостей; его зависимости требуют отдельного build policy.
При заданном primary entry build manifest должен содержать COMPUTE-слой с этим
полным путём (directory + entry). Независимый typed-слой и STATIC-слой не
заменяют выбранный primary.
Default direct publication без явного intent сохраняет существующий Bun launcher.

Если entry не удалось определить однозначно:
- для strict-фреймворков (`nextjs`, `nuxt`) деплой завершается ошибкой с actionable подсказкой
- для остальных деплой тоже завершается ошибкой с просьбой явно задать `[deploy] entry`

CLI не патчит `package.json` в build output для PROCESS. Резолвленный entry —
путь файла, исполняемого закрытым runtime profile, а не shell command.
Если entry не найден или найден неоднозначно, деплой останавливается до отправки runtime metadata.

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

Для STATIC Python-сборки настройте build toolchain без полей запуска `PROCESS`:

```toml
[build]
toolchain = "python"
python_version = "3.13"
command = "mkdocs build"
output_directory = "site"

[deploy]
compute = "static"
```

`deploy.runtime`, `entry`, `module`, `application`, `server`, `args` и
`deploy.python_version` несовместимы с явно выбранным STATIC. Build toolchain и
launcher замораживаются раздельно в `metadata.sourceBuildContext` при детекции.
Для Git-деплоя задайте команду `mkdocs build` и output `site` в настройках
проекта ONREZA: Builder использует immutable Project snapshot. Эти два поля
из `onreza.toml` не импортируются автоматически и не переопределяют snapshot;
локальный CLI использует их из файла.
Python-команда сборки получает выбранный pinned interpreter, staged packages и
console scripts. Различные build/serving Python minor допустимы для code-only
output; runtime dependencies требуют совпадающих qualified minor.
Для приложения с объявленными dependencies `--skip-install` требует файлов
подготовленного дерева зависимостей, которые войдут в Python COMPUTE слой.
Пустого каталога `site-packages` недостаточно.

Локальная Python-сборка с нативными dependencies требует Linux x86_64 и
определённую glibc не ниже целевого manylinux ABI (сейчас 2.39). На macOS,
Windows, non-x64 host, musl либо неизвестной libc используйте Git/Builder
для нативных dependencies. Чистые Python packages доступны для локальной сборки.
После локальной пользовательской install-команды нативные dependencies,
сохранённые для Python COMPUTE, требуют Git/Builder независимо от ОС хоста:
произвольная команда не подтверждает их target ABI. Это правило применяется
и к дополнительным Python COMPUTE слоям; build-only STATIC packages исключаются.
Нативные расширения самого Python-приложения собирайте через Git/Builder.
После локальной пользовательской build-команды CLI проверяет публикуемые
Python-файлы: нативный application output требует сборки в Builder.
Установленные `.pth` paths и import hooks обрабатываются при сборке и запуске;
повторный bootstrap в том же interpreter не запускает hooks второй раз. Публикация готового output через `--skip-build` не запускает
host interpreter.
