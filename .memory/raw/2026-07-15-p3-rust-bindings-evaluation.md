# P3 external Rust bindings evaluation

Дата: 2026-07-15.

## Проверенный кандидат

- `cargo info tdlib-rs@1.4.0`, `cargo info tdlib-rs-gen@1.4.0` и
  `cargo info tdlib-rs-parser@1.4.0` загрузили published crates из crates.io.
- Published `tdlib-rs 1.4.0` фиксирует `TDLIB_VERSION = "1.8.61"`, commit
  `11e254af695060d8890024dd7faa1cc2d6685ef8`; project pin — TDLib 1.8.66,
  `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- `tdlib-rs-gen 1.4.0/src/functions.rs` генерирует `json!` request с literal `"@type"`,
  проверяет `response["@type"]` и вызывает `serde_json::from_value(...).unwrap()`.
- `tdlib-rs-gen 1.4.0/src/enums.rs` генерирует `#[serde(tag = "@type")]`; generated
  structs не имеют `serde(flatten)` storage для неизвестных fields. Неизвестный enum
  constructor поэтому не удовлетворяет требуемому lossless round trip.

Primary sources: [tdlib-rs repository](https://github.com/FedericoBruzzone/tdlib-rs),
[published crate documentation](https://docs.rs/tdlib-rs/1.4.0/tdlib_rs/).

## Вывод

Готовый crate не принят как runtime API: он не совпадает с exact pin и не выполняет
forward-compatible raw contract. Использован минимальный вариант: существующий repo
parser + generated static Rust descriptors + один общий validator/lossless codec.
Native TDLib binaries остаются уже закреплёнными artifacts проекта.
