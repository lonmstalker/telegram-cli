# TDLib capability generator foundation digest

Дата: 2026-07-15.

## Scope

P0.5a создаёт статическую capability-модель и pure generator boundary до review полного 1010-method corpus. Источники: pinned `vendor/tdlib/td_api.tl`, exact owner policy/artifact, `plans.md`, `product.md`, `HARNESS.md` и инструкция пользователя о TDD, bounded resources и independent review.

## Verified implementation

- `telegram-core::method_capability` содержит closed vocabularies account, current-account entitlement, application, DC, 13 authorization states, 16 administrator rights, 16 member rights и 14 business-bot rights; rights inventories сверяются с pinned schema.
- `CapabilityDescriptor` разделяет additive synchronous execution, exact method-level axes, typed DNF runtime evidence и parameter-value notices. Descriptor не является policy permission и не утверждает, что runtime account уже удовлетворяет требованиям.
- Semantic refs не допускают relabel одинаковых wire types: chat target — только exact `chat_id`/`supergroup_id`, forum topic — `forum_topic_id`, business runtime evidence — только unambiguous `business_connection_id`.
- Public constructors fail closed до canonicalization: максимум 16 DNF clauses, 32 atoms, 32 parameter notices и 16 synchronous string values. Business-connection evidence несовместимо с regular-user-only alternatives; chat ownership несовместимо с bot-only alternatives.
- `tdlib-registry-gen::capability::generate` остаётся pure bounded function. Он повторно генерирует owner manifest из reviewed source, требует exact schema/owner/method coverage, связывает каждую row с signature/documentation/owner hashes и выдаёт canonical JSON только после полной проверки.
- Document recognizers сравнивают capability axes, runtime DNF и parameter notices exact, поэтому policy не может добавить скрытое сужение. Распознанный capability/runtime gate signal вне exact reviewed corpus даёт `SchemaDrift`; parameter-only gate нельзя поднять до method-wide restriction.
- Generator source digest включает capability/owner engines и semantic dependencies `method_capability.rs`, `feature.rs`, `schema.rs`.

## Resource and integrity evidence

- Input caps: vendor manifest 64 KiB, schema 2 MiB, owner/capability policy по 4 MiB, owner/output по 4 MiB, methods 2048.
- Pure generation не запускает network, subprocesses, threads или resident resources; verification запускалась с `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`.
- После verification `target` занимает 144 MiB; `.work-*` и generator temp leftovers не обнаружены.
- Implementation SHA-256 на checkpoint: `method_capability.rs` — `9ba379ce099a8778329af6bd9a43c0a9d0aa531b2bd8d89943dc06ece3a161da`; `capability.rs` — `6d561f70dd6d237a9c6e77452bbfbf4f6395054031a5712ee515a83b5ee84efc`.

## Verification

- Red-green tests воспроизвели и закрыли omitted/extra runtime requirements, omitted/extra/wrong parameter notices, auth widening, account/application/DC narrowing, semantic-ID relabel, incomplete DNF, reviewed unsupported pinned gate signals и public constructor caps.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test --workspace --all-targets --jobs 2`: generator 34/34, core 20/20, остальные targets green.
- `cargo clippy --workspace --all-targets --jobs 2 -- -D warnings`, `cargo fmt --all -- --check`, `python3 scripts/check-workspace-boundaries.py`, `git diff --check`: green.
- Два независимых reviewer passes: `Approved`, blockers отсутствуют.

## Explicit boundary

- Canonical capability policy/artifact для 1010 methods ещё не создан и не принят.
- Runtime evaluator, текущие account/right checks, risk, prerequisite/retry classes, registry/codec/router, daemon/CLI и live Telegram acceptance этим checkpoint не реализованы.
- Unsupported documentation остаётся fail-closed boundary до отдельного corpus review; foundation нельзя выдавать за full capability coverage.
