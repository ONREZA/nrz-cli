# nrz — ONREZA Platform CLI

Аналог `vercel` / `wrangler` для ONREZA. Rust-based, single binary.

## Архитектура

```
src/
  main.rs           — entrypoint, clap парсинг
  cli/              — CLI определения (clap derive)
    mod.rs           — Cli, Command enum, DevArgs, BuildArgs, DeployArgs
    db.rs            — DbArgs, DbCommand
    kv.rs            — KvArgs, KvCommand
  dev/              — nrz dev
    mod.rs           — оркестрация: detect → emulator → spawn
    detect.rs        — определение фреймворка по package.json
    inject.rs        — генерация JS bootstrap для globalThis.ONREZA
    process.rs       — child process менеджмент
  build/            — nrz build
    mod.rs           — валидация output dir + manifest
    manifest.rs      — парсинг и валидация manifest.json (BUILD_OUTPUT_SPEC v1)
  deploy/           — nrz deploy
    mod.rs           — upload + activate
  emulator/         — локальная эмуляция платформы
    mod.rs           — data dir, общие утилиты
    kv.rs            — in-memory KV store с TTL (BTreeMap)
    db.rs            — D1-compatible SQLite (rusqlite)
    server.rs        — HTTP API для JS bootstrap (/__nrz/kv/*, /__nrz/db/*)
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

## Сборка и запуск

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

## Связанные репозитории

- `onreza/adapters` — TypeScript адаптеры (@onreza/adapter-astro, @onreza/adapter-nitro)
- `onreza/deployment` — платформа (builder, edge-server, nrz-isolate)
