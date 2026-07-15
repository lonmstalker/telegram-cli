# Полнота TDLib API

Статус: exact schema snapshot, strict Rust parser/inventory, generated Rust registry и native artifacts для macOS arm64 и Linux x86_64 закреплены; low-level TDJSON transport реализован с одним receive loop и `@extra` correlation. Registry даёт descriptors, recursive request validation, lossless unknown-object codec и generated capability disposition для каждого method. Reviewed risk/account/runtime/retry rows хранятся в [`capability-notes.md`](capability-notes.md), всё остальное — `DefaultDeny`. Universal core call/router и CLI surface ещё не созданы.

## Проверенный upstream baseline

- TDLib commit: `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Версия из `CMakeLists.txt`: `1.8.66`.
- SHA-256 `td_api.tl`: `10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31`.
- Functions: `1010`; definitions до `---functions---`: `2168`, из них 9 builtins и 2159 object constructors; distinct result type families: `745`.
- Updates: `184`; authorization states: `13`.
- Source: <https://github.com/tdlib/td/tree/07d3a0973f5113b0827a04d54a93aaaa9e288348>.
- Local manifest: [`vendor/tdlib/manifest.json`](../vendor/tdlib/manifest.json); gate: `python3 scripts/check-tdlib-pin.py`.
- Strict parser/inventory: [`crates/telegram-core/src/schema.rs`](../crates/telegram-core/src/schema.rs); gate: `cargo test --locked --offline -p telegram-core --lib --jobs 2`.
- Generated registry/codec: [`crates/telegram-core/src/registry.rs`](../crates/telegram-core/src/registry.rs), contract: [`tdlib-generated-registry.md`](tdlib-generated-registry.md); gate: `python3 scripts/check-tdlib-registry.py`.
- Reviewed capability contracts: [`capability-notes.md`](capability-notes.md); canonical data table: [`tools/tdlib-registry-gen/capabilities.json`](../tools/tdlib-registry-gen/capabilities.json).
- Planning boundary: `python3 scripts/check-planning-boundary.py` запрещает переносить номера из `HARNESS.md` в runtime code и machine-readable contracts.
- macOS arm64 native provenance: [`vendor/tdlib/native-builds/aarch64-apple-darwin.json`](../vendor/tdlib/native-builds/aarch64-apple-darwin.json).
- Linux x86_64 native provenance: [`vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json`](../vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json); exact builder recipe: [`vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.Dockerfile`](../vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.Dockerfile).
- Общий provenance gate: `python3 scripts/check-tdlib-native-pin.py`; проверка обоих локальных artifacts: `python3 scripts/check-tdlib-native-pin.py --require-local-artifact`.
- TDJSON transport contract: [`tdjson-transport.md`](tdjson-transport.md); native no-client proof: `.memory/raw/2026-07-15-tdjson-transport-native-smoke.md`.

Exact schema pin принят в `D-20260715-003`, strict parser — в `D-20260715-006`. Текущий macOS arm64 artifact и crash ownership доказаны correction checkpoint `W-20260715-008`; Linux x86_64 artifact закреплён в `W-20260715-040`. Для обоих artifacts bit-for-bit reproducibility не заявлена. Удаление numeric feature taxonomy из исполняемой архитектуры принято в `D-20260715-017`.

## Граница planning inventory и исполняемой архитектуры

`F001`…`F022` — только номера разделов документационного inventory в [`HARNESS.md`](../HARNESS.md) и связанных harness-файлах. Они не являются доменными типами, владельцами TDLib methods или полями generated contracts.

В коде используются предметные имена модулей и сущностей: `schema`, а позднее — `authorization`, `session`, `updates`, `messages`, `statistics` и другие семантические bounded contexts. Стабильный идентификатор raw API — точное имя TDLib method/constructor вместе с закреплённой signature evidence.

## Capability classification contract

Классификация методов — данные, а не код. В P3 `tdlib-registry-gen` строит registry из pinned
schema и одной capability-таблицы (файл данных: method → risk, account scope, runtime
requirements, retry/idempotency class):

- Каждый метод таблицы обязан существовать в pinned schema; duplicate/unknown rows fail closed.
- Метод без строки или без ревью получает `default-deny` — это валидное состояние, не ошибка.
- Схема закреплена одним SHA-256 (`scripts/check-tdlib-pin.py`); per-method signature/documentation
  hash evidence и тесты на мутации текста схемы не используются.
- Static prerequisites не выдаются за удовлетворённые; runtime-проверка — отдельный слой.
- Стартовое наполнение таблицы: [`capability-notes.md`](capability-notes.md).

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
