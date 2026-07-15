# Generated TDLib registry

`tools/tdlib-registry-gen` читает единственный pinned source
`vendor/tdlib/td_api.tl` существующим strict parser из `telegram-core::schema` и создаёт
`crates/telegram-core/src/registry/generated.rs`. Committed artifact содержит Rust
descriptors всех builtins, constructors, methods, result types, updates и authorization
states вместе с fields, signatures и documentation. Та же generation принимает одну
таблицу [`capabilities.json`](../tools/tdlib-registry-gen/capabilities.json) и добавляет
reviewed risk/account/runtime/retry data либо `DefaultDeny` для каждого method.

## Runtime contract

- `ValidatedRequest` принимает только method из pinned registry, отклоняет неизвестные
  поля и рекурсивно проверяет типы присутствующих fields. Отсутствующие fields не
  объявляются ошибкой: TL schema не кодирует TDJSON default/required semantics.
- Concrete object type может ссылаться на constructor name (`proxy:proxy`), abstract
  type — на result family (`OptionValue`); validator поддерживает обе формы.
- `TdObject` требует только object с непустым string `@type`, сохраняет исходный
  `serde_json::Value` целиком и при наличии возвращает descriptor pinned constructor.
  Поэтому неизвестный будущий constructor и неизвестные fields известного constructor
  проходят exact round trip без проекции.
- Caller не владеет transport `@extra`. `@type` остаётся TDJSON wire-discriminator внутри
  generated registry/общего codec; application workflows не получают per-method JSON
  wrappers.
- Capability generator отклоняет duplicate/unknown rows и closed-vocabulary ошибки.
  Отсутствующая row валидна и не наследует свойства похожего method.

Registry пересоздаётся `cargo run -p tdlib-registry-gen`. Gate
`python3 scripts/check-tdlib-registry.py` проверяет deterministic equality committed
artifact и generator output; schema identity по-прежнему закрепляет только
`scripts/check-tdlib-pin.py`.

## Почему не готовый crate

Проверенный `tdlib-rs 1.4.0` поддерживает TDLib 1.8.61, тогда как product pin — 1.8.66.
Его generator также создаёт `@type` внутри request functions и tagged Serde enums без
lossless unknown-field storage. Полная проверка сохранена в
[external evaluation digest](../.memory/raw/2026-07-15-p3-rust-bindings-evaluation.md).
Проект повторно использует готовые pinned native `tdjson` artifacts, но не подменяет
exact schema/forward-compatibility contract несовместимым Rust wrapper.
