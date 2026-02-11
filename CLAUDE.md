# nrz — ONREZA Platform CLI

Аналог `vercel` / `wrangler` для ONREZA. Rust-based, single binary.

## Архитектура

```
src/
  lib.rs            — библиотечный интерфейс (для тестов)
  main.rs           — entrypoint, clap парсинг
  cli/              — CLI определения (clap derive)
    mod.rs           — Cli, Command enum
    db.rs            — DbArgs, DbCommand
    db_handler.rs    — обработчик команд db
    kv.rs            — KvArgs, KvCommand
    kv_handler.rs    — обработчик команд kv
  dev/              — nrz dev
    mod.rs           — оркестрация: detect → emulator → spawn
    detect.rs        — определение фреймворка по package.json
    detect_tests.rs  — тесты detect
    inject.rs        — генерация JS bootstrap для globalThis.ONREZA
    inject_tests.rs  — тесты inject
    process.rs       — child process менеджмент
  build/            — nrz build
    mod.rs           — валидация output dir + manifest
    manifest.rs      — парсинг и валидация manifest.json
    manifest_tests.rs — тесты manifest
  deploy/           — nrz deploy
    mod.rs           — upload + activate
  emulator/         — локальная эмуляция платформы
    mod.rs           — data dir, общие утилиты
    kv.rs            — in-memory KV store с TTL (BTreeMap)
    kv_tests.rs      — тесты KV store
    db.rs            — D1-compatible SQLite (rusqlite)
    server.rs        — HTTP API для JS bootstrap (/__nrz/kv/*, /__nrz/db/*)

tests/              — интеграционные тесты
  emulator_http_test.rs — тесты HTTP API эмулятора
  cli_integration_test.rs — интеграционные тесты CLI
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
| `nrz dev` | Запуск dev-сервера фреймворка + эмуляция ONREZA runtime |
| `nrz build` | Валидация build output и manifest |
| `nrz deploy` | Деплой на платформу |
| `nrz db shell` | Интерактивная SQLite консоль |
| `nrz db execute <sql>` | Выполнение SQL запроса |
| `nrz db info` | Информация о базе (таблицы, размер) |
| `nrz db reset` | Сброс локальной БД |
| `nrz kv get <key>` | Получить значение |
| `nrz kv set <key> <val>` | Установить значение |
| `nrz kv list` | Список ключей |
| `nrz kv clear` | Очистить KV |

## Установка

### Quick install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/onreza/nrz-cli/main/install.sh | bash
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
- **serde/serde_json** — JSON парсинг манифеста
- **reqwest** — HTTP клиент для deploy API
- **command-group** — child process groups (graceful shutdown)
- **console/indicatif** — цветной вывод, прогресс-бары

## Тестирование

### Структура тестов

**Unit-тесты** — в отдельных файлах `*_tests.rs` рядом с тестируемым модулем:
```
src/
  emulator/
    kv.rs           — основной код
    kv_tests.rs     — unit-тесты (18 тестов)
  dev/
    detect.rs
    detect_tests.rs — unit-тесты (10 тестов)
    inject.rs
    inject_tests.rs — unit-тесты (8 тестов)
  build/
    manifest.rs
    manifest_tests.rs — unit-тесты (14 тестов)
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
- Scopes: `dev`, `build`, `deploy`, `emulator`, `cli`, `deps`, `ci`

## Локальные данные

`nrz dev` создаёт `.onreza/data/` в проекте пользователя:
- `dev.db` — SQLite файл для D1 эмуляции
- `kv.json` — персистенция KV store (опционально)

Эта директория должна быть в `.gitignore`.

## Как работает nrz dev

```
nrz dev
  1. Определяет фреймворк (astro, nuxt, sveltekit, nitro) по package.json
  2. Создаёт .onreza/data/ директорию
  3. Поднимает emulator HTTP сервер (/__nrz/kv/*, /__nrz/db/*)
  4. Генерирует JS bootstrap скрипт (globalThis.ONREZA = {...})
  5. Запускает `bunx <framework> dev` с NODE_OPTIONS=--import <bootstrap>
  6. Framework dev server видит globalThis.ONREZA — всё работает
  7. Ctrl+C → graceful shutdown child process + emulator
```

## Релизы

Релизы публикуются автоматически через GitHub Actions при создании тега `v*.*.*`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Workflow (`.github/workflows/release.yml`):
- Запускает тесты на каждом PR/push
- Собирает бинарники под Linux x64, macOS x64/arm64, Windows x64
- Создаёт GitHub Release с чек-суммами

## Связанные репозитории

- `onreza/adapters` — TypeScript адаптеры (@onreza/adapter-astro, @onreza/adapter-nitro)
- `onreza/deployment` — платформа (builder, edge-server, nrz-isolate)
