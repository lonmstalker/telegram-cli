# Полнота TDLib API

Статус: exact schema snapshot, strict Rust parser/inventory, schema-bound capability foundation и macOS arm64 native artifact закреплены. Linux artifact, полный capability/risk/retry policy, generated raw registry/codec/router и runtime implementation ещё не созданы.

## Проверенный upstream baseline

- TDLib commit: `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Версия из `CMakeLists.txt`: `1.8.66`.
- SHA-256 `td_api.tl`: `10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31`.
- Functions: `1010`; definitions до `---functions---`: `2168`, из них 9 builtins и 2159 object constructors; distinct result type families: `745`.
- Updates: `184`; authorization states: `13`.
- Source: <https://github.com/tdlib/td/tree/07d3a0973f5113b0827a04d54a93aaaa9e288348>.
- Local manifest: [`vendor/tdlib/manifest.json`](../vendor/tdlib/manifest.json); gate: `python3 scripts/check-tdlib-pin.py`.
- Strict parser/inventory: [`crates/telegram-core/src/schema.rs`](../crates/telegram-core/src/schema.rs); gate: `cargo test --locked --offline -p telegram-core --lib --jobs 2`.
- Schema-bound capability module: [`tools/tdlib-registry-gen/src/capability.rs`](../tools/tdlib-registry-gen/src/capability.rs); gate: `cargo test --locked --offline -p tdlib-registry-gen --lib --jobs 2`.
- Planning boundary: `python3 scripts/check-planning-boundary.py` запрещает переносить номера из `HARNESS.md` в runtime code и machine-readable contracts.
- macOS arm64 native provenance: [`vendor/tdlib/native-builds/aarch64-apple-darwin.json`](../vendor/tdlib/native-builds/aarch64-apple-darwin.json); artifact gate: `python3 scripts/check-tdlib-native-pin.py --require-local-artifact`.

Exact schema pin принят в `D-20260715-003`, strict parser — в `D-20260715-006`. Текущий macOS arm64 artifact и crash ownership доказаны correction checkpoint `W-20260715-008`; Linux x86_64 artifact и bit-for-bit reproducibility остаются открытыми. Удаление numeric feature taxonomy из исполняемой архитектуры принято в `D-20260715-017`.

## Граница planning inventory и исполняемой архитектуры

`F001`…`F022` — только номера разделов документационного inventory в [`HARNESS.md`](../HARNESS.md) и связанных harness-файлах. Они не являются доменными типами, владельцами TDLib methods или полями generated contracts.

В коде используются предметные имена модулей и сущностей: `schema`, `method_capability`, `capability`, а позднее — `authorization`, `session`, `updates`, `messages`, `statistics` и другие семантические bounded contexts. Стабильный идентификатор raw API — точное имя TDLib method/constructor вместе с закреплённой signature evidence.

## Capability generator contract

`tdlib-registry-gen` — non-default library tooling package. Его `capability` module является pure bounded transformation и не открывает TDLib DB, не использует сеть, subprocesses или resident resources.

- Inputs: pinned `td_api.tl` bytes и capability policy bytes. Vendor manifest проверяется отдельным schema-pin gate и не дублируется как непрозрачный generator input.
- Policy method set обязан в точности совпасть со schema method set; duplicate/missing/unknown rows fail closed.
- Каждая строка привязана к exact method name, canonical signature SHA-256 и documentation SHA-256.
- Static prerequisites представлены closed typed model; runtime-факты не выдаются за удовлетворённые.
- Bounds: schema 2 MiB, capability policy 4 MiB, output 4 MiB, максимум 2048 methods.
- Output детерминирован, отсортирован по method name и строится в памяти. Committed 1010-method capability policy/artifact пока отсутствует.

## Что входит в coverage

- Все functions после `---functions---` в закреплённом `td_api.tl`.
- Все входные и выходные objects, union constructors и updates, необходимые этим functions.
- Raw JSON round-trip с сохранением неизвестных полей для совместимости.
- Runtime capability result для bot-only, user-only, Premium, Business, admin-gated, financial и official-app-only возможностей.
- Полный core/CLI reachability; MCP parity обязательна только после включения MCP.

## Машинная запись метода

Будущий generated registry обязан содержать для каждого точного TDLib method:

- `schema_commit`, `schema_hash`, `method`, signature и documentation evidence;
- account/capability constraints;
- risk class и default policy;
- prerequisite workflow или `none`;
- update dependencies;
- retry/idempotency/reconciliation class;
- core, CLI и MCP availability;
- generated validation и test status.

Для каждого object constructor, update и authorization state registry также хранит schema signature/hash, symbol kind, codec status и routing/handler status. State-critical updates указывают reducer dependency; неизвестный update сохраняется losslessly и помечает зависимое состояние incomplete.

## Fail-closed gates

CI падает, если:

- planning ID попал в executable code или machine-readable contract;
- число разобранных functions не совпадает с generated registry;
- число constructors/objects не совпадает с generated codec registry;
- число updates не совпадает с lossless update registry и router;
- число authorization states не совпадает с явно обработанными auth states;
- отсутствует risk/retry/prerequisite/capability classification;
- core поддерживает method, но CLI не может вызвать его через общий raw contract;
- MCP включён, но его schema расходится с CLI/protocol;
- runtime TDLib version/schema не совпадает с закреплённым manifest;
- update constructor теряется вместо raw-preservation.

## Формула приёмки

Полная приёмка требует одновременного выполнения:

- `schema_functions == classified_methods == core_raw_methods == cli_raw_methods`;
- `schema_object_constructors == registry_object_constructors == codec_object_constructors`;
- `schema_updates == registry_updates == lossless_routed_updates`;
- `schema_authorization_states == registry_authorization_states == handled_authorization_states`.

После включения MCP дополнительно: `cli_protocol_methods == mcp_protocol_methods`. Curated workflow coverage измеряется отдельно и не подменяет raw coverage.
