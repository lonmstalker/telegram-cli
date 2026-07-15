# Полнота TDLib API

Статус: exact schema snapshot, strict Rust parser/inventory и macOS arm64 native artifact закреплены; Linux artifact, classified generated registry и runtime implementation ещё не созданы.

## Проверенный upstream baseline

- TDLib commit: `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Версия из `CMakeLists.txt`: `1.8.66`.
- SHA-256 `td_api.tl`: `10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31`.
- Functions в `td_api.tl`: `1010` на момент проверки 2026-07-15.
- Type/object definitions до `---functions---`: `2168`: 9 builtins и 2159 object constructors; distinct result type families: `745`.
- Updates: `184`; authorization states: `13`.
- Source: <https://github.com/tdlib/td/tree/07d3a0973f5113b0827a04d54a93aaaa9e288348>.
- Local manifest: [`vendor/tdlib/manifest.json`](../vendor/tdlib/manifest.json); offline gate: `python3 scripts/check-tdlib-pin.py`.
- Strict parser/inventory: [`crates/telegram-core/src/schema.rs`](../crates/telegram-core/src/schema.rs); corpus gate: `cargo test -p telegram-core --lib --jobs 2`.
- macOS arm64 native provenance: [`vendor/tdlib/native-builds/aarch64-apple-darwin.json`](../vendor/tdlib/native-builds/aarch64-apple-darwin.json); local artifact gate: `python3 scripts/check-tdlib-native-pin.py --require-local-artifact`.
- Current local artifact: 27 654 296 bytes, SHA-256 `5dbd30094b4fbfd35904e88d88e413f423ea7283bd81b34305eac31be6852e7e`; correction evidence: [reviewed rebuild digest](../.memory/raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md).

Этот exact commit принят как initial production schema pin в `D-20260715-003`; переход на другой commit требует явного manifest/schema diff, parser gate и повторной классификации. Strict-subset parser/inventory принят в `D-20260715-006` и `W-20260715-009`; это schema evidence, а не доказательство generated registry/codec/router/runtime parity. Текущий macOS arm64 artifact и crash ownership доказаны correction checkpoint `W-20260715-008`; `W-20260715-007` остаётся историей первой pre-review build. Linux x86_64 artifact и classified generated registry остаются отдельными P0 gates. Одна reviewed macOS rebuild не считается доказательством bit-for-bit reproducibility; RSS/tree limits являются sampled watchdog thresholds.

## Что входит в coverage

- Все functions после `---functions---` в закреплённом `td_api.tl`.
- Все входные и выходные objects, union constructors и updates, необходимые этим functions.
- Raw JSON round-trip с сохранением неизвестных полей для совместимости.
- Runtime capability result для bot-only, user-only, Premium, Business, admin-gated, financial и official-app-only возможностей.
- Полный core/CLI reachability; MCP parity обязательна только после включения MCP.

## Машинная запись метода

Generated manifest обязан содержать для каждого точного TDLib `@type`:

- `schema_commit`, `schema_hash`, `method`;
- единственный `feature_id` из `F001..F022`;
- account/capability constraints;
- risk class и default policy;
- prerequisite workflow или `none`;
- update dependencies;
- retry/idempotency/reconciliation class;
- core, CLI и MCP availability;
- generated validation и test status.

Для каждого object constructor, update и authorization state registry также хранит schema signature/hash, symbol kind, feature owner, codec status и routing/handler status. State-critical updates дополнительно указывают reducer dependency; неизвестный update сохраняется losslessly и помечает зависимое состояние incomplete.

Точное имя TDLib method уже является стабильным ID; отдельные ручные `API###` не вводятся.

## Fail-closed gates

CI падает, если:

- число разобранных functions не совпадает с generated registry;
- число constructors/objects не совпадает с generated codec registry;
- число updates не совпадает с lossless update registry и router;
- число authorization states не совпадает с явно обработанными auth states;
- method не назначен feature или назначен нескольким;
- отсутствует risk/retry/prerequisite classification;
- core поддерживает method, но CLI не может вызвать его через общий raw contract;
- MCP включён, но его schema расходится с CLI/protocol;
- runtime TDLib version/schema не совпадает с закреплённым manifest;
- update constructor теряется вместо raw-preservation.

## Feature ownership

| Feature | Основные семейства |
|---|---|
| F001–F006 | lifecycle, auth, schema/raw, chains, CLI, MCP |
| F007 | users, contacts, profile identity |
| F008 | chats, lists, folders, topics, secret chats |
| F009 | messages, history, search, drafts, reactions, polls |
| F010 | files, uploads, downloads, generated/media content |
| F011 | basic groups, supergroups, channels, members, invites, moderation, boosts |
| F012 | bots, inline queries, callbacks, games, bot-side testing |
| F013 | Web Apps and Mini App launch lifecycle |
| F014 | stickers, custom emoji, emoji status and set lifecycle |
| F015 | stories, calls, group calls, live streams |
| F016 | account settings, privacy, notifications, devices, sessions, push |
| F017 | Business features, connected/managed bots and quick replies |
| F018 | payments, invoices, Stars, gifts, Premium, Passport, affiliates |
| F019 | statistics, async graphs, revenue, storage and network resources |
| F020 | localization, links, themes, backgrounds, proxies, logs, test/custom calls |
| F021 | cross-cutting retry, policy, limits, audit and metrics classification |
| F022 | agent discovery and compact-skill routing metadata |

## Формула приёмки

Полная приёмка требует одновременного выполнения:

- `schema_functions == classified_methods == core_raw_methods == cli_raw_methods`;
- `schema_object_constructors == registry_object_constructors == codec_object_constructors`;
- `schema_updates == registry_updates == lossless_routed_updates`;
- `schema_authorization_states == registry_authorization_states == handled_authorization_states`.

После включения MCP дополнительно: `cli_protocol_methods == mcp_protocol_methods`. Curated workflow coverage измеряется отдельно и не подменяет raw coverage.
