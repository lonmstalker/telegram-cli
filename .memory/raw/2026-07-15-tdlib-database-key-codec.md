# TDLib 1.8.66 database-key codec evidence

Дата проверки: 2026-07-15.

## Source identity

- TDLib commit: `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Source archive SHA-256: `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- Archive получен и проверен существующим pinned native-build pipeline; secret values и live DB не использовались.

## Проверенные контракты

- `vendor/tdlib/td_api.tl`: `setTdlibParameters ... database_encryption_key:bytes ... = Ok`.
- Exact source `td/tl/tl_json.h`: `from_json_bytes` вызывает `base64_decode` для JSON string.
- Exact source `td/telegram/TdDb.cpp`: empty key превращается в internal raw key `cucumber`.
- Exact source `td/telegram/TdDb.cpp`: binlog `WrongPassword` возвращается как TDLib error `401`, `Wrong database encryption key`.

## Reproduction

```sh
shasum -a 256 target/tdlib-native/downloads/1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb.tar.gz
rg -n 'setTdlibParameters' vendor/tdlib/td_api.tl
tar -xOf target/tdlib-native/downloads/1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb.tar.gz td-07d3a0973f5113b0827a04d54a93aaaa9e288348/td/tl/tl_json.h | rg -n -C 5 'from_json_bytes'
tar -xOf target/tdlib-native/downloads/1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb.tar.gz td-07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/TdDb.cpp | rg -n -C 5 'as_db_key|Wrong database encryption key'
```

Вывод: TDJSON request обязан передавать non-empty raw key bytes как padded Base64; missing/empty key нужно остановить до request, а error 401 — обработать fail-closed.
