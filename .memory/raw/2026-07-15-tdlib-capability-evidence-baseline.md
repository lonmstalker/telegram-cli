# TDLib capability evidence baseline

Дата: 2026-07-15.

## Scope

Этот checkpoint измеряет real pinned capability-like documentation до расширения runtime evidence grammar. Он также исправляет authorization recognizer, который раньше читал только `@description` и пропускал method-level contract в `setCustomLanguagePack.@info`.

## Exact evidence

- Pinned schema: 1010 methods, SHA-256 `10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31`.
- Current closed recognizer находит 193 unique methods. Sorted `method + LF` SHA-256: `cbe074623352b1b4e970af939aed6297e7ce37366d7fd5ad7cedcf1a36848706`.
- Test-extracted `has_runtime_gate_signal` body SHA-256: `5cdf338bec6fa08d0f69d31c999c8ca384581f96cb6105384cebf754e3e65f1a`.
- Method-level descriptions дают 162 methods, SHA-256 `f0ce76fbe2c80365483306b65f1334fdc20a6ad93d956d32aec45c0c1d3b99fa`; parameter tags дают 42 methods, SHA-256 `aff7c31486573fe7c5d3c5b3fb586e1499d0816f6c9b857c1d48700c415deb9b`. Sets пересекаются, поэтому их counts не суммируются.
- Пять real methods имеют exact reviewed runtime contracts, set SHA-256 `fe9034c9b7022707b3b29090ea6891209130cfb1b3acf69642bfbab652ee286d`.
- Foundation имеет exact runtime disposition для 5 real methods. Остальные 188 остаются fail-closed; sorted open-method set SHA-256: `c9e5131cd86d5ebe7eb697f409953d4090c58a4c21ba9e442075701c6d950a34`.
- Test связывает open set с `SchemaDrift` и exact `unsupported runtime documentation`; иной error kind или молчаливое принятие меняет gate.
- Authorization validation теперь читает значения всех structured method documentation tags. Red test на `setCustomLanguagePack` сначала воспроизвёл ошибочный Ready-only contract, затем прошёл с exact pre-authorization state set.
- Exact non-Ready authorization contract содержит 73 methods, sorted set SHA-256 `89a4dd651b3372d2310ddb6fa16e2e6827d0bd67b6555a8e5800694ceb0440b3`.

## TDD and resource evidence

- Red: `authorization_contract_reads_all_method_documentation_tags` отклонил правильный pre-authorization descriptor; corpus gate показал 188 undispositioned methods.
- Green: `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test --locked --offline -p tdlib-registry-gen --jobs 2 -- --test-threads=2` прошёл 37/37 tests.
- Whole-workspace tests, Clippy `-D warnings`, fmt, four workspace negative controls, wiki contracts и diff-check прошли; independent review вернул `Approved`.
- Изменение не добавляет dependency, thread, subprocess, network или runtime resource.
- Source checkpoint SHA-256: `capability.rs` — `34b24fbf6a4c0b7d7e011a77a555df77283705406a72c263e30432e62089f1be`; `capability/tests.rs` — `b79cb957b23fe7cb16f47d5688e2ba18d0932999a86942296290a131d9172a8f`.

## Boundary

- 188 open methods не считаются capability coverage.
- Exact signal set смешивает runtime capability, input prerequisite, retry и lexical false positives. Следующие reviewed tasks должны классифицировать эти lanes и добавлять closed typed atoms по source family.
- Method выходит из open set только после consumption всех его exact signals. Один method может одновременно требовать current-account right и external object fact; частичная поддержка не считается disposition.
- Canonical 1010-method capability policy/artifact, runtime evaluator, risk/prerequisite/retry fields и live acceptance остаются open.
