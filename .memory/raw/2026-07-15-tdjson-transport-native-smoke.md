# TDJSON transport native smoke digest

Дата: 2026-07-15. Immutable runtime evidence для первого Tasks-пункта P1.

## Artifact и команда

- macOS arm64 artifact SHA-256: `5dbd30094b4fbfd35904e88d88e413f423ea7283bd81b34305eac31be6852e7e`.
- Artifact path получен через canonical `scripts/tdlib_native.py::artifact_cache_path` и передан test process абсолютным `TDJSON_LIBRARY_PATH`; значение не является secret.
- Команда: `cargo test -p telegram-core tdjson_native::tests::pinned_native_no_client_call_uses_real_tdjson_transport -- --ignored --exact` с указанным path.
- Result: 1 passed, 0 failed; wall time test 0.03 s.

## Доказанный slice

- Exact native library динамически открыта через четыре используемых exports из пяти, закреплённых native provenance; `td_json_client_execute` transport не вызывает.
- `getOption { name: "version" }` отправлен обычным transport request с owned `@extra`; response был сопоставлен pending caller и вернул `optionValueString { value: "1.8.66" }`.
- TDLib parameters, database path/key и authorization secrets не передавались; это no-client transport smoke, не login/session proof.
- Pure suite дополнительно: 15 passed, 0 failed, 1 native test ignored by default. Reversed parallel responses, один receive thread, update order, unmatched `@extra` и malformed JSON fail-closed проверены deterministic fake backend.

## Граница

- Smoke выполнен на macOS artifact. Linux artifact ABI/exports/runtime identity доказаны его native provenance/inspection, но Rust transport test на Linux здесь не запускался.
- Authorization, ordered reducer/cache, restart и daemon lifecycle этим evidence не доказаны.
