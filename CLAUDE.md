# nrz — ONREZA Platform CLI

Аналог `vercel` / `wrangler` для ONREZA. Rust-based, single binary.

## Принципы разработки

**LLM-First CLI.** Все команды проектируются для автономного использования LLM-агентами:
- `--json` глобальный флаг (`NRZ_JSON=1`) — машинный вывод в stdout
- `--token` глобальный флаг (`NRZ_TOKEN`) — аутентификация без device flow
- `--env` глобальный флаг (`NRZ_ENV`) — выбор environment (production/preview/development или ID)
- Интерактивные prompts имеют non-interactive альтернативу (`--project-id`, `--force`)
- JSON output: один объект в stdout, ошибки `{"error": "..."}`, exit code 1
- Human output: цветной текст в stderr (по умолчанию)
- Новые команды обязаны поддерживать оба режима

## Архитектура

```
src/
  lib.rs            — библиотечный интерфейс (для тестов)
  main.rs           — entrypoint, clap парсинг, загрузка конфига
  config/           — onreza.toml конфигурация проекта
    mod.rs           — ProjectConfig, EnvVarDecl, EnvVisibility, load/save/generate_template
    config_tests.rs  — тесты конфига (11 тестов)
    env_decl_tests.rs — тесты [env] деклараций (12 тестов)
  cli/              — CLI определения (clap derive)
    mod.rs           — Cli, Command enum
    db.rs            — DbArgs, DbCommand
    db_handler.rs    — обработчик команд db
    kv.rs            — KvArgs, KvCommand
    kv_handler.rs    — обработчик команд kv
    env.rs           — EnvArgs, EnvCommand
    env_handler.rs   — обработчик команд env
    domains.rs       — DomainsArgs, DomainsCommand
    domains_handler.rs — обработчик команд domains
    db_migrate_handler.rs — обработчик команд db migrate/push
  link/
    environment_ref.rs   — load/save/resolve environment (per-environment DB)
    environment_ref_tests.rs — тесты environment_ref
  projects.rs       — nrz projects
  deployments.rs    — nrz deployments
  logs.rs           — nrz logs
  rollback.rs       — nrz rollback
  dev/              — nrz dev
    mod.rs           — оркестрация: emulator → spawn
    inject.rs        — генерация JS bootstrap для globalThis.ONREZA
    inject_tests.rs  — тесты inject
    process.rs       — child process менеджмент
  build/            — nrz build
    mod.rs           — валидация output dir + manifest
    manifest.rs      — парсинг и валидация manifest.json
    manifest_tests.rs — тесты manifest
  detect/           — детекция фреймворков, PM, SSR, адаптеров
    mod.rs           — оркестратор: detect(), infer_compute_type()
    types.rs         — все типы (DetectionResult, ComputeType, etc.)
    presets.rs       — 18 пресетов фреймворков (compile-time data)
    package_json.rs  — парсинг package.json
    package_manager.rs — детекция PM (packageManager field + lockfiles)
    ssr.rs           — SSR-анализ (Next.js, Nuxt, SvelteKit)
    adapter.rs       — детекция @onreza/* адаптеров
    vite_config.rs   — парсинг vite.config для outDir
    static_html.rs   — fallback static HTML detector
  detect_sync.rs    — best-effort sync detection results to API
  deploy/           — nrz deploy
    mod.rs           — upload + activate + migrations detection
  init/             — nrz init
    mod.rs           — инициализация проекта на платформе
    init_tests.rs    — тесты init
  migrations/       — D1 migration system
    mod.rs           — scan, checksum, next number
    tracking.rs      — _nrz_migrations tracking table
    mod_tests.rs     — тесты core логики
    tracking_tests.rs — тесты tracking
  emulator/         — локальная эмуляция платформы
    mod.rs           — data dir, общие утилиты
    kv.rs            — in-memory KV store с TTL (BTreeMap)
    kv_tests.rs      — тесты KV store
    db.rs            — D1-compatible SQLite (rusqlite)
    server.rs        — HTTP API для JS bootstrap (/__nrz/kv/*, /__nrz/db/*)
  upgrade/          — самообновление
    mod.rs           — скачивание и замена бинарника

tests/              — интеграционные тесты
  emulator_http_test.rs — тесты HTTP API эмулятора
  cli_integration_test.rs — интеграционные тесты CLI

Конфигурация:
  onreza.toml           — Конфигурация проекта (коммитится в git)
  lefthook.yml          — Git hooks конфигурация
  commitlint.config.js  — Правила для commitlint (standalone, без extends)
  .onrezarelease.jsonc  — Конфигурация onreza-release (versioning, changelog, binaries)
  package.json          — Node.js зависимости (commitlint, lefthook, onreza-release)
```

## Контракт

CLI не зависит от адаптеров. Связь — через BUILD_OUTPUT_SPEC:
- Адаптер генерирует `.onreza/manifest.json` при build
- CLI читает и валидирует этот манифест
- CLI загружает артефакты на платформу

Спецификация: `../deployment/docs/architecture/BUILD_OUTPUT_SPEC.md`

## Команды

| Команда | Описание |
|---------|----------|
| `nrz detect` | Детекция фреймворка, PM, SSR, compute type (--json, --slug-only, --save) |
| `nrz dev` | Запуск dev-сервера фреймворка + эмуляция ONREZA runtime |
| `nrz build` | Валидация build output и manifest |
| `nrz deploy` | Деплой на платформу |
| `nrz db shell` | Интерактивная SQLite консоль |
| `nrz db execute [sql]` | Выполнение SQL (аргумент, `--file`, или stdin) |
| `nrz db info` | Информация о базе (таблицы, размер) |
| `nrz db reset` | Сброс локальной БД |
| `nrz db reset --remote` | Сброс удалённой D1 БД (с подтверждением) |
| `nrz db migrate create <name>` | Создать новый файл миграции |
| `nrz db migrate apply` | Применить pending миграции (локально) |
| `nrz db migrate apply --remote` | Применить миграции на удалённой D1 |
| `nrz db migrate status` | Показать статус миграций |
| `nrz db push` | Выполнить SQL на удалённой D1 |
| `nrz kv get <key>` | Получить значение |
| `nrz kv set <key> <val>` | Установить значение |
| `nrz kv list` | Список ключей |
| `nrz kv clear` | Очистить KV |
| `nrz upgrade` | Обновить до последней версии |
| `nrz projects` | Список проектов |
| `nrz deployments` | Список деплоев проекта |
| `nrz logs` | Runtime логи проекта |
| `nrz env list` | Список переменных окружения |
| `nrz env set <key> <val>` | Установить переменную окружения |
| `nrz env delete <key>` | Удалить переменную окружения |
| `nrz env pull` | Скачать переменные в .env.local |
| `nrz env validate` | Валидация переменных по [env] декларации в onreza.toml |
| `nrz domains list` | Список кастомных доменов |
| `nrz domains add <domain>` | Добавить кастомный домен |
| `nrz domains remove <id>` | Удалить кастомный домен |
| `nrz domains verify <id>` | Проверить DNS домена |
| `nrz rollback` | Откат деплоя |
| `nrz init` | Инициализация проекта на ONREZA платформе |

## Установка

### Quick install

**Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/onreza/nrz-cli/main/install.sh | bash
```

**Windows (PowerShell 7+):**
```powershell
iwr -useb https://raw.githubusercontent.com/onreza/nrz-cli/main/install.ps1 | iex
```

### Скачать вручную

Скачайте бинарник для вашей платформы из [GitHub Releases](https://github.com/onreza/nrz-cli/releases):

- `nrz-linux-x64` — Linux x86_64
- `nrz-macos-x64` — macOS Intel
- `nrz-macos-arm64` — macOS Apple Silicon
- `nrz-windows-x64.exe` — Windows x86_64

### Сборка из исходников

```bash
cargo install --git https://github.com/onreza/nrz-cli
```

### Для разработки

```bash
cargo build                  # debug build
cargo build --release        # release build (LTO, strip)
cargo run -- dev             # запустить dev mode
cargo run -- build ./myapp   # валидировать билд
cargo test                   # тесты
```

## Зависимости (ключевые)

- **clap** — CLI парсинг (derive macros)
- **tokio** — async runtime
- **rusqlite** (bundled) — SQLite для D1 эмуляции
- **toml** — парсинг onreza.toml конфига
- **serde/serde_json** — JSON парсинг манифеста
- **reqwest** — HTTP клиент для deploy API
- **command-group** — child process groups (graceful shutdown)
- **console/indicatif** — цветной вывод, прогресс-бары

## Тестирование

### Структура тестов

**Unit-тесты** — в отдельных файлах `*_tests.rs` рядом с тестируемым модулем:
```
src/
  config/
    config_tests.rs — unit-тесты (11 тестов)
    env_decl_tests.rs — unit-тесты (12 тестов)
  emulator/
    kv.rs           — основной код
    kv_tests.rs     — unit-тесты (18 тестов)
  dev/
    inject.rs
    inject_tests.rs — unit-тесты (8 тестов)
  build/
    manifest.rs
    manifest_tests.rs — unit-тесты (14 тестов)
  migrations/
    mod_tests.rs      — unit-тесты (9 тестов)
    tracking_tests.rs — unit-тесты (5 тестов)
  init/
    init_tests.rs     — unit-тесты (6 тестов)
  link/
    environment_ref_tests.rs — unit-тесты (9 тестов)
```

Подключение в `mod.rs`:
```rust
#[cfg(test)]
mod xxx_tests;
```

**Интеграционные тесты** — в папке `tests/`:
- `tests/emulator_http_test.rs` — HTTP API эмулятора (5 тестов)
- `tests/cli_integration_test.rs` — CLI команды через assert_cmd (13 тестов)

### Запуск тестов

```bash
cargo test                    # все тесты
cargo test --test emulator_http_test   # конкретный интеграционный тест
cargo test kv_tests           # тесты конкретного модуля
```

### Правила написания тестов

1. **Unit-тесты** — тестируют отдельные функции/методы, используют `tempfile::tempdir()` для изоляции
2. **Интеграционные тесты** — тестируют публичный API (HTTP endpoints, CLI команды)
3. Никаких inline `#[cfg(test)] mod tests {}` в файлах с кодом — только отдельные `*_tests.rs`
4. Используем `assert_cmd` для CLI тестов, `reqwest` для HTTP тестов

## Конвенции

- Код на Rust, edition 2024
- `cargo fmt` перед коммитом
- `cargo clippy` без warnings
- Conventional Commits: `feat(dev):`, `fix(build):`, `chore(deps):` и т.д.
- Scopes: `dev`, `build`, `deploy`, `emulator`, `cli`, `config`, `deps`, `ci`

## Конфигурация проекта — onreza.toml

`onreza.toml` в корне проекта — единый конфиг, коммитится в git. Создаётся автоматически при `nrz init` / `nrz link`.

```toml
[project]
id = "proj_abc123"      # ID проекта на платформе

[dev]
# command = "npm run dev"
# port = 4321            # порт dev-сервера
# host = "127.0.0.1"     # bind host для эмулятора

# data_dir = ".onreza/data"
# db_name = "dev.db"

[build]
# output_dirs = ["dist", ".output", "build"]

[deploy]
# skip_migrations = false

[migrations]
# dir = "migrations"

[db]
# default_env = "development"

[env]
# strict = false

[env.declarations]
# DATABASE_URL = "sensitive"
# PUBLIC_API_URL = "plain"
# OPTIONAL_VAR = { visibility = "plain", required = false }
```

### Секция `[env]` — декларация переменных окружения

Объявляет какие env vars нужны проекту, их visibility (sensitive/plain) и обязательность.

**`[env]`** — настройки:
- `strict = true` — `nrz env push` загружает только переменные из `[env.declarations]` (также включается флагом `--declared-only`)

**`[env.declarations]`** — переменные:
- Строка `"sensitive"` / `"plain"` — shorthand, переменная обязательна
- Table `{ visibility = "...", required = false }` — для необязательных переменных

Используется в:
- `nrz env push` — определяет `is_secret` для каждой переменной (вместо эвристики по имени); при `strict`/`--declared-only` фильтрует только объявленные
- `nrz env validate` — проверяет что все required переменные заданы на платформе
- `nrz deploy` — pre-flight проверка required переменных перед деплоем (skip: `--skip-env-check`)

Приоритет: `CLI flag > env var (NRZ_*) > onreza.toml > hardcoded default`

## Локальные данные

`.onreza/` — полностью локальная директория (gitignored целиком, `nrz init` автоматически добавляет в `.gitignore`):
- `.onreza/data/dev.db` — SQLite файл для D1 эмуляции
- `.onreza/data/kv.json` — персистенция KV store (опционально)
- `.onreza/project.json` — legacy ссылка на проект (для обратной совместимости)
- `.onreza/environment.json` — личный выбор environment разработчика

## Как работает nrz dev

```
nrz dev
  1. Читает dev command из onreza.toml или CLI флага
  2. Создаёт .onreza/data/ директорию
  3. Поднимает emulator HTTP сервер (/__nrz/kv/*, /__nrz/db/*)
  4. Генерирует JS bootstrap скрипт (globalThis.ONREZA = {...})
  5. Запускает dev command с NODE_OPTIONS=--import <bootstrap>
  6. Dev server видит globalThis.ONREZA — всё работает
  7. Ctrl+C → graceful shutdown child process + emulator
```

## Релизы и Changelog

Релизы управляются через [onreza-release](https://github.com/onreza/onreza-release) — автоматическое определение версии по conventional commits, встроенный changelog, загрузка бинарников, npm-дистрибуция через trusted publishing (OIDC).

Конфигурация: `.onrezarelease.jsonc`

### Создание релиза

1. Перейди в [Actions → Release](https://github.com/ONREZA/nrz-cli/actions/workflows/release.yml)
2. Нажми **"Run workflow"**

Workflow автоматически:
- Определит следующую версию по conventional commits
- Обновит `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`
- Создаст коммит и тег
- Соберёт бинарники под все платформы (linux-x64, darwin-x64, darwin-arm64, win32-x64)
- Создаст GitHub Release
- Опубликует npm-пакеты (основной + platform-specific) с provenance

### Локальный dry-run

```bash
npx onreza-release --dry-run --verbose
```

### Формат коммитов (Conventional Commits)

Все коммиты должны следовать [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

**Допустимые типы:**
- `feat` — новая функциональность
- `fix` — исправление бага
- `docs` — документация
- `style` — форматирование (не влияет на код)
- `refactor` — рефакторинг
- `perf` — производительность
- `test` — тесты
- `chore` — рутинные задачи
- `ci` — CI/CD
- `build` — система сборки
- `revert` — откат изменений

**Допустимые скопы:** `cli`, `dev`, `build`, `deploy`, `emulator`, `kv`, `db`, `deps`, `ci`, `release`, `tests`

Примеры:
```bash
git commit -m "feat(dev): add custom command support"
git commit -m "fix(kv): handle expired entries in list command"
git commit -m "docs: update installation instructions"
```

### Git Hooks (Lefthook)

Проект использует [lefthook](https://github.com/evilmartians/lefthook) для проверки коммитов перед созданием:

```bash
# Установить lefthook и hooks
npm install

# Или установить lefthook глобально
cargo install lefthook
lefthook install
```

**Автоматические проверки:**
- `commit-msg` — проверка формата сообщения (commitlint)
- `pre-commit` — `cargo fmt` и `cargo clippy`
- `pre-push` — `cargo test`

### Перегенерация commitlint конфига

```bash
npx onreza-release generate-commitlint --format js --output commitlint.config.js
```

### Самообновление

```bash
nrz upgrade              # Обновить до последней версии
nrz upgrade --force      # Принудительно переустановить
nrz upgrade --version v0.1.0  # Установить конкретную версию
```

Команда автоматически определяет платформу, скачивает нужный бинарник с GitHub Releases и заменяет текущий исполняемый файл.

## Связанные репозитории

- `onreza/adapters` — TypeScript адаптеры (@onreza/adapter-astro, @onreza/adapter-nitro)
- `onreza/deployment` — платформа (builder, edge-server, nrz-isolate)
