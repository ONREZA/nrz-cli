# Разбор Codex Security findings от 2026-07-26

Источник: `codex-security-findings-2026-07-26T15-46-24.546Z.csv`.

Работа выполнена в отдельных ветках:

- `nrz-cli`: `security/codex-findings-2026-07-26`;
- канонический `deployment/crates/nrz-fn-policy`:
  `security/codex-nrz-cli-findings-2026-07-26`.

Vendored-копия `nrz-fn-policy` в `nrz-cli` обновлена штатным
`sync:nrz-cli-crates`.

## Итог

- 34 findings исправлены в актуальном коде.
- #34 более не соответствует текущему роутеру; аналогичная оставшаяся KV-поверхность
  дополнительно защищена session token.
- #32 является ожидаемой моделью доверия к host proxy/native CA; изменение кода
  не требуется.
- #16, #17 и #18 усилены, приняты как временный риск и отложены в platform
  `deployment/ROADMAP.md`; same-origin SHA-256 не аутентифицирует GitHub Release
  при компрометации самого release/workflow, поэтому findings не закрыты.

Дубликаты, которые стоит связать в Security Cloud:

- #6, #9, #12 и #13 — один root cause: непроверенный authoritative
  `outputDirectory`;
- #8 и #14 — варианты symlink escape при Next.js refinement;
- #5 и #11 — варианты выхода выбранного monorepo app за repository root;
- #15 и #38 — manifest/output containment на границе упаковки;
- #16, #17 и #18 — один release-authenticity contract для трёх consumers.

## Сводка

| # | Решение | Оценка уровня |
|---:|---|---|
| 1 | Исправлено | High подтверждён |
| 2 | Исправлено в source of truth и vendor | High подтверждён |
| 3 | Исправлено | High подтверждён |
| 4 | Исправлено | High подтверждён |
| 5 | Исправлено | High подтверждён |
| 6 | Исправлено; дубликат общей output boundary | High подтверждён |
| 7 | Исправлено | High подтверждён |
| 8 | Исправлено; вариант общей output boundary | High подтверждён |
| 9 | Исправлено; дубликат #6 | High подтверждён |
| 10 | Исправлено | High подтверждён |
| 11 | Исправлено | High подтверждён |
| 12 | Исправлено; дубликат #6/#9 | High подтверждён |
| 13 | Исправлено; дубликат #12 | High подтверждён |
| 14 | Исправлено; вариант #8 | High подтверждён |
| 15 | Исправлено | High подтверждён |
| 16 | Deferred / accepted risk; нужен authenticity verifier | High подтверждён |
| 17 | Deferred / accepted risk; нужен authenticity verifier | High подтверждён |
| 18 | Deferred / accepted risk; нужен authenticity verifier | High подтверждён |
| 19 | Исправлено | Medium подтверждён |
| 20 | Исправлено | Medium подтверждён |
| 21 | Исправлено | Medium подтверждён |
| 22 | Исправлено | Medium подтверждён |
| 23 | Исправлено | Medium подтверждён |
| 24 | Исправлено | Medium подтверждён |
| 25 | Исправлено | Medium подтверждён |
| 26 | Исправлено | Medium подтверждён |
| 27 | Исправлено | Medium подтверждён |
| 28 | Исправлено | Medium подтверждён |
| 29 | Исправлено | Medium подтверждён |
| 30 | Исправлено | Medium подтверждён |
| 31 | Исправлено | Medium подтверждён |
| 32 | False positive / accepted host trust | Рекомендуется Informational |
| 33 | Исправлено | Medium подтверждён |
| 34 | Исходное описание устарело; остаточная поверхность исправлена | Рекомендуется Outdated |
| 35 | Исправлено | Low подтверждён |
| 36 | Исправлено | Рекомендуется Informational |
| 37 | Исправлено | Low подтверждён |
| 38 | Исправлено; пересекается с #15 | Рекомендуется Medium |
| 39 | Исправлено как correctness bug | Informational подтверждён |

## Feedback для Security Cloud

### #1 — DB commands can target databases outside the project

**Статус:** исправлено. **Уровень:** High соответствует риску выполнения
query/delete/config над другой БД, доступной тому же workspace token.

> Finding подтверждён. ID-shaped values больше не обходят project attachment
> lookup, а fallback к любой БД из глобального списка удалён.
> `resolve_db_by_id_or_name` принимает только БД с attachment к выбранному
> project и сопоставляет внутри этого множества как ID, так и имя. Добавлена
> regression-проверка cross-project ID/name.

### #2 — ONREZA Functions policy can be bypassed via Bun aliases

**Статус:** исправлено. **Уровень:** High подтверждён.

> Finding подтверждён динамическим PoC. Канонический
> `deployment/crates/nrz-fn-policy` теперь отклоняет plain aliases,
> assignments, `globalThis.Bun`, `globalThis.process` и computed global access.
> Исправление синхронизировано в `nrz-cli/vendor`; platform parity suite и CLI
> preview regression tests проходят. Vendor-only patch не использовался:
> source of truth исправлен первым.

### #3 — Unverified remote installer runs in privileged release jobs

**Статус:** исправлено. **Уровень:** High подтверждён.

> Remote `curl | sh` удалён из CI и release jobs. Новый bootstrap скачивает
> строго Dagger 0.21.4 для Linux x86_64 по HTTPS, сверяет архив с reviewed
> SHA-256 `4db2f807...f2485e9672` и fail-closed отклоняет любую версию без
> добавленного в репозиторий digest. Обновление Dagger теперь требует явного
> review нового checksum.

### #4 — NPM installer downloads unsigned GitHub binaries

**Статус:** исправлено для подмены GitHub asset после npm publication.
**Уровень:** High подтверждён.

> Finding подтверждён. NPM package теперь получает те же release archives как
> Dagger input, вычисляет SHA-256 каждого platform archive и встраивает digests
> в опубликованный package. Postinstall принимает только HTTPS, ограничивает
> redirects/size/time, полностью проверяет SHA-256 до gunzip/tar extraction и
> fail-closed завершает установку при mismatch. Старый target удаляется до
> extraction, а отсутствие ожидаемого binary после extraction является
> ошибкой. Таким образом, замена одного GitHub asset не совпадёт с digest из
> отдельно опубликованного npm package.

### #5 — App config can select build output outside the repository

**Статус:** исправлено. **Уровень:** High подтверждён.

> Выбранный app canonicalize-ится и обязан находиться под canonical monorepo
> root. Output candidates проходят общую проверку: запрещены absolute,
> parent/Windows/UNC paths, финальный symlink и canonical escape. Добавлены
> regression tests для symlinked app и внешнего output.

### #6 — Authoritative output_directory can escape project root

**Статус:** исправлено. **Уровень:** High подтверждён.

> Все уровни output resolution, включая authoritative user value, используют
> одну fail-closed containment function. Absolute/parent/Windows/UNC paths,
> final symlink и canonical target вне project root отклоняются до manifest
> loading или упаковки. Это общий root-cause fix; #9/#12/#13 являются
> дубликатами по другим источникам того же значения.

### #7 — pnpm sandbox install auto-enables all dependency scripts

**Статус:** исправлено. **Уровень:** High подтверждён.

> Автоматическая установка
> `npm_config_dangerously_allow_all_builds=true` /
> `pnpm_config_dangerously_allow_all_builds=true` полностью удалена вместе с
> эвристическим parser-ом policy files. CLI больше не расширяет разрешение на
> dependency lifecycle scripts; project/package-manager policy остаётся
> единственным authority.

### #8 — User .next refinement can follow symlinked output outside repo

**Статус:** исправлено. **Уровень:** High подтверждён.

> Next.js refinement больше не использует прямой `is_dir()` на `.next` paths.
> Каждый refined candidate проходит canonical project containment; финальный
> symlink и intermediate symlink с внешним target отклоняются.

### #9 — Authoritative outputDirectory can escape project root

**Статус:** исправлено; дубликат #6. **Уровень:** High подтверждён.

> Finding описывает тот же boundary, что #6, с другим spelling/source поля.
> Исправление централизовано в output resolver и применяется ко всем
> authoritative/detected/framework candidates. Рекомендуется связать с #6 как
> duplicate, а не вести отдельный локальный guard.

### #10 — Symlinked Prisma package copy can leak local files

**Статус:** исправлено. **Уровень:** High подтверждён.

> Prisma source symlink canonicalize-ится и копируется только при target внутри
> canonical project root; dangling/external targets пропускаются. Финальный
> destination package и весь parent tree `node_modules/@prisma` проверяются
> через `symlink_metadata`, поэтому copy не пишет через destination symlink.

### #11 — Monorepo --app can escape repo and deploy host files

**Статус:** исправлено. **Уровень:** High подтверждён.

> Local detection FS больше не разрешает parent/absolute/Windows paths и не
> следует symlink target за canonical root. После workspace resolution app path
> дополнительно canonicalize-ится и обязан начинаться с monorepo root.

### #12 — Server outputDirectory can escape project root

**Статус:** исправлено; дубликат #6/#9. **Уровень:** High подтверждён.

> Server-provided outputDirectory проходит ту же централизованную containment
> проверку, что user и detected values. Внешний canonical target не может стать
> build/deploy root.

### #13 — Server outputDirectory can escape project root

**Статус:** исправлено; дубликат #12. **Уровень:** High подтверждён.

> Текст и sink совпадают с #12. Рекомендуется пометить duplicate после принятия
> общего output resolver fix.

### #14 — Next standalone output can follow symlink outside project

**Статус:** исправлено; вариант #8. **Уровень:** High подтверждён.

> `.next/standalone` выбирается только через общую canonical containment
> function. Symlinked standalone root или intermediate `.next` с target вне
> project отклоняется до подготовки Next.js artifact.

### #15 — Symlinked deploy output can upload files outside build dir

**Статус:** исправлено. **Уровень:** High подтверждён.

> Output root не может быть symlink/escape. Manifest layer, entry и prerender
> files canonicalize-ятся и обязаны оставаться внутри canonical output.
> SOURCE_BUNDLE scanner также отклоняет symlink target вне artifact root.
> Упомянутый старый tar helper более не является текущим upload path.

### #16 — Self-update installs unsigned release binaries

**Статус:** deferred / accepted risk; не закрывать. **Уровень:** High подтверждён.

> Updater больше не доверяет `browser_download_url`: URL строится только для
> `onreza/nrz-cli`, выбранного tag и ожидаемого asset. Release обязан содержать
> `checksums-sha256.txt`; archive ограничен по размеру и SHA-256 проверяется до
> extraction/replacement. Workflow также публикует Sigstore-backed GitHub
> Artifact Attestations. Replacement использует unique `create_new` candidate,
> owner-only mode во время записи и atomic rename на Unix, не следуя старому
> предсказуемому `.tmp`. Остаток: checksum находится в том же GitHub Release, а
> updater пока не проверяет attestation/signature, поэтому компрометация release
> workflow/assets всё ещё входит в заявленный threat model.

### #17 — Windows installer installs unsigned release binaries

**Статус:** deferred / accepted risk; не закрывать. **Уровень:** High подтверждён.

> PowerShell installer скачивает archive и checksum manifest, требует ровно
> один валидный digest, сверяет `Get-FileHash SHA256` до extraction и запускает
> `--version` по точному установленному path. Temp directory имеет GUID-based
> unique name, downloads ограничены по времени/размеру. Документация больше не
> предлагает `iwr | iex`. Остаток тот же, что #16: same-release checksum не
> является независимой подписью.

### #18 — Installer downloads unsigned release binaries

**Статус:** deferred / accepted risk; не закрывать. **Уровень:** High подтверждён.

> Unix installer скачивает `checksums-sha256.txt`, требует единственную запись
> ожидаемого asset и проверяет её через sha256sum/shasum/openssl до extraction.
> Download ограничен по времени/размеру, а несуществующий в release matrix
> `linux-arm64` больше не рекламируется. Документация больше не предлагает
> `curl | bash`; реальный install v0.36.2 проверен в изолированной директории.
> Остаток: нужен обязательный independent authenticity verifier.

### #19 — Node module symlinks can pull secrets into SSR artifacts

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Selected runtime scan больше не ставит любой symlink target в очередь.
> Допустимы только уже выбранные roots или canonical workspace package roots,
> объявленные monorepo contract. Разрешение применяется к точному package root,
> а не к любому его descendant: symlink на произвольный project файл (PoC с
> `.env`, в том числе внутри workspace package) возвращает
> `INVALID_BUILD_OUTPUT`.

### #20 — rules pull follows symlinks when writing rules file

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> `rules pull` проверяет destination через `symlink_metadata` и отклоняет
> symbolic link даже с `--force`. Запись выполняется в unique `create_new`
> temporary file с `sync_all`, затем rename заменяет сам directory entry, а не
> target symlink.

### #21 — Unsanitized runtime log tail printed on deploy failure

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Общий terminal sanitizer удаляет ANSI CSI/OSC и опасные C0/C1 control bytes.
> Runtime build-log tail проходит sanitizer до human output; redaction bearer,
> credentials и URLs сохранена. Для однострочных repository-derived полей
> newline/tab дополнительно заменяются пробелами. Status/success/warn и
> top-level human errors используют тот же boundary.

### #22 — Recursive symlinks accepted in source bundles

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> SOURCE_BUNDLE planning отклоняет self/ancestor recursive links, включая
> `dir/loop -> .` и эквивалентный `../dir`, до archive creation. Regression
> подтверждает fail-closed поведение.

### #23 — Source bundle temp archive is created world-readable

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Unix temp archive создаётся атомарно через `create_new` с mode `0600`;
> regression проверяет отсутствие group/other permissions. Cleanup on error и
> Drop cleanup сохранены.

### #24 — Server framework detection can bundle project secrets

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Express/Koa detector теперь требует dependency из `dependencies` либо
> runtime-семантического `optionalDependencies`, а не совпадение только в
> `devDependencies`. Vite/static app с dev-only Express mock больше не
> классифицируется как server framework и не выбирает project root как PROCESS
> artifact.

### #25 — Unbounded remote detection manifest can exhaust resources

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Stdin и JSON ограничены 4 MiB; отдельно ограничены tree entries, число files,
> path length/depth, размер одного content и общий content budget. Manifest
> отклоняет absolute/parent/Windows paths. Stdin читается bounded reader-ом
> (`limit + 1`) и oversized input отклоняется до parsing; локальный detector
> также читает только regular files не более 512 KiB.

### #26 — Next metadata copy follows destination symlinks

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> До copy проверяется каждый component destination directory tree, включая
> корни `_static`, `_prerender` и `public`, а также финальный file через
> `symlink_metadata`. Root/nested/final destination symlink приводит к ошибке,
> внешний target не изменяется.

### #27 — Auto-generated COMPUTE manifests skip path validation

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Auto-generated PROCESS/COMPUTE manifest проходит тот же `validate` и
> `verify_files`, что authored manifest, до serialization и deploy planning.
> Невалидный/missing/escaping entry не может пройти через generated path.

### #28 — Health autodetection can hang on symlinked special files

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Health scanner использует `symlink_metadata`, принимает только regular files
> и читает через bounded `take(MAX + 1)`. Symlinks, `/dev/zero` и symlinked
> package manifest пропускаются; recursive Nest scan также не следует symlinks.

### #29 — Unvalidated deployment ID is interpolated into API paths

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> `--resume-deployment` парсится как UUID до создания runner-context URL.
> Empty, slash/query/traversal и произвольные строки возвращают
> `INVALID_ARGUMENT`, не выполняя HTTP request.

### #30 — Unsanitized detect output enables terminal escape injection

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Все repository/manifest-derived поля human detect output проходят общий
> terminal sanitizer: framework/name/version, package manager, command/output,
> monorepo packages, config files, SSR features, structure и reason. JSON mode
> остаётся структурированным и не модифицируется.

### #31 — Empty --project-id falls back to onreza.toml project

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Явно переданный empty/whitespace `--project-id` теперь немедленно отклоняется
> и не может silently fallback к linked project из `onreza.toml`. Fallback
> сохраняется только когда flag действительно отсутствует; непустые explicit и
> config IDs нормализуются через `trim`.

### #32 — Reqwest defaults enable proxy/native-CA token interception

**Статус:** false positive / accepted trust model. **Рекомендуемый уровень:**
Informational.

> Системный proxy и native CA являются ожидаемой operator/host trust boundary
> для developer/CI CLI и нужны для enterprise TLS interception. Атакующий,
> способный изменить trusted root store или системную proxy-конфигурацию,
> уже контролирует host-level trust. Отключение defaults сломает легитимные
> окружения и не является универсальным security fix. Если продукту требуется
> certificate pinning/no-proxy mode, это должен быть отдельный explicit
> threat-model contract, а не исправление данного finding.

### #33 — Project IDs are injected into API paths unescaped

**Статус:** исправлено. **Уровень:** Medium подтверждён.

> Project CRUD кодирует ID через percent encoding как один path segment перед
> GET/PATCH/DELETE. Slash, traversal, query и fragment syntax больше не меняют
> endpoint. Добавлен regression с `/../` и query.

### #34 — Unauthenticated dev emulator exposes SQL endpoints

**Статус:** исходное описание устарело; текущий аналог исправлен.
**Рекомендуемый resolution:** Not relevant anymore / Outdated.

> Указанных SQL routes и SQLite execution sink в актуальном emulator router
> больше нет; текущая поверхность содержит KV endpoints. Для неё добавлен
> случайный per-session token, обязательный header middleware на всех routes
> (включая health), token injection в bootstrap и authenticated readiness.
> Bootstrap создаётся по unique `create_new` path с mode `0600` на Unix и
> удаляется при readiness failure/нормальном завершении. Запрос без token
> получает 401. Поэтому исходный SQL finding следует закрыть как outdated, а
> текущий residual считается отдельно устранённым.

### #35 — Symlinked pnpm hoist dir can delete outside symlinks

**Статус:** исправлено. **Уровень:** Low подтверждён.

> Перед traversal `node_modules/.pnpm/node_modules` проверяется через
> `symlink_metadata`. Если сам hoist directory является symlink, prune
> полностью пропускается и внешний directory не перечисляется/не изменяется.

### #36 — Committed local project metadata leaks identifiers

**Статус:** исправлено. **Рекомендуемый уровень:** Informational.

> `.kaneo.json` удалён из tracking и добавлен в `.gitignore`. Содержавшиеся
> workspace/project IDs сами по себе не являются credential или secret, поэтому
> Low завышает security impact; это hygiene/privacy metadata issue.

### #37 — Numeric API error codes can leak raw error bodies

**Статус:** исправлено. **Уровень:** Low подтверждён.

> Structured API error parser принимает string и numeric code через untagged
> enum и нормализует code в string. Numeric code больше не ломает parsing и не
> переводит обработку в raw-body fallback. Regression сохраняет message/code/
> details.

### #38 — Manifest paths can escape build output directory

**Статус:** исправлено. **Рекомендуемый уровень:** Medium, либо duplicate #15.

> Validator отклоняет absolute, Windows drive/UNC и parent paths для layer
> directory/entry/prerender fields. `verify_files` canonicalize-ит каждую
> существующую ссылку и требует containment внутри canonical output. Поскольку
> актуальный deploy sink действительно может упаковать host file, Low
> недооценивает impact; root cause пересекается с #15.

### #39 — SSR fallback parsing can mis-detect static output

**Статус:** исправлено. **Уровень:** Informational подтверждён.

> Finding корректно классифицирован как functional correctness. Fallback parser
> теперь ограничивает выражение текущего property до следующей top-level
> запятой/закрывающей скобки, ищет только top-level `||`/`??` и учитывает
> вложенные braces/quotes. Unrelated `featureFlag: false` и вложенный
> `(featureFlag || false)` больше не превращают SSR config в static-compatible.

## Отложенный release-authenticity contract (#16–18)

Текущий слой уже даёт:

1. SHA-256 verification до extraction/installation во всех трёх consumers.
2. Ограничение updater URL ожидаемым repository/tag/asset.
3. Sigstore-backed GitHub Artifact Attestations для каждого release archive.
4. Удаление рекомендаций remote-script pipe-to-shell/iex.

Но заявленный threat model включает компрометацию GitHub Release/workflow. Для
полного закрытия нужен один обязательный consumer-side trust anchor:

- pinned public key и detached signature manifest, где private key хранится вне
  GitHub repository/release authority; либо
- обязательная Sigstore/GitHub attestation verification с pinning repository и
  workflow identity.

GitHub документирует стандартную проверку как
`gh attestation verify <artifact> -R ONREZA/nrz-cli`. Делать её обязательной в
install scripts означает добавить зависимость от authenticated/available `gh`;
для Rust updater потребуется встроенный verifier или отдельный helper. До выбора
этого product contract #16–18 остаются `Deferred / Accepted Risk` и не считаются
закрытыми. Условия возврата и Definition of Done зафиксированы в platform
`deployment/ROADMAP.md`.

Официальная документация:
<https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations>.

## Проверки

- `cargo test --all`: passed;
- `cargo clippy --all-targets --all-features -- -D warnings`: passed;
- `mise run check`: passed;
- `bun test ./.dagger/scripts/release-scripts.test.ts`: 17 passed;
- `shellcheck install.sh .github/scripts/install-dagger.sh`: passed;
- `cargo test -p nrz-fn-policy`: 22 passed;
- canonical `nrz-fn-policy` Clippy with `-D warnings`: passed;
- реальная установка `v0.36.2` изменённым `install.sh` в `/tmp`: checksum
  verified, `nrz 0.36.2` executed;
- `pwsh` отсутствует в текущем окружении, поэтому `install.ps1` не был
  исполнен/распарсен PowerShell runtime;
- `git diff --check`: passed.
