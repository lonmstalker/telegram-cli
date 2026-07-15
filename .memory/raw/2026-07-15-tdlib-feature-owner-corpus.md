# Immutable digest: P0.4b TDLib feature-owner corpus

Date: 2026-07-15

## Scope

Этот digest фиксирует reviewed owner-only classification всех 1010 methods exact TDLib `1.8.66` schema. Он не доказывает capability, risk, prerequisite, retry/idempotency, generated codec/router, constructor/update/auth-state parity или runtime implementation.

## Canonical artifacts

- `policy/tdlib-feature-owners.json`: 184 373 bytes, file SHA-256 `dc3dd5c21c85dc1e429f1cdf76f8e7947e90a9df1d4f1d0cb4483a8972841141`.
- `generated/tdlib-feature-owners.json`: 402 507 bytes, file SHA-256 `06f8ed5d4321376a6dd99e522130ed40fa1a67ffda86e505e81134b37e98d194`.
- Canonical mapping SHA-256 из artifact: `4687dc47c69e798e0afb9e0ea8ee82ed5d91bcde00e51f476ff08ea8226ce790`.
- Независимый exact owner digest по sorted `method + NUL + feature_id + LF`: `72c741dc024091b5f25f5fc97a46224a557560dcb483bb0d3a29c4085f9b13ac`.
- Semantic policy/rules/overrides SHA-256: `7b1f2bff...d9d8`, `cbec907c...96d4`, `96f238dc...0d50`.

Policy содержит 17 non-empty feature rules, 252 positive name atoms и 372 exact overlap overrides. Generated artifact содержит 1010 sorted unique rows; exact set совпадает со всеми parsed methods pinned schema. Direct owner counts: `F001=3`, `F002=25`, `F003=0`, `F004=1`, `F005=0`, `F006=0`, `F007=50`, `F008=74`, `F009=156`, `F010=32`, `F011=92`, `F012=63`, `F013=23`, `F014=72`, `F015=99`, `F016=63`, `F017=49`, `F018=105`, `F019=17`, `F020=86`, `F021=0`, `F022=0`. Zero direct methods у cross-cutting/product-surface features не являются missing coverage: каждый schema method всё равно имеет ровно одного domain owner.

## TDD and semantic review evidence

Первая corpus-проверка была red: fixed `check` и engine corpus test отклонили отсутствующие policy/artifact. Первый mechanically complete draft давал 1010/1010, но independent semantic audit заблокировал commit: broad camel-name matches ошибочно пересекали domain/risk boundaries. До публикации были исправлены:

- шесть `group_call_id` message-control methods: F009 → F015;
- story notification exceptions: F015 → F016; per-chat auto-delete: F016 → F009;
- Markdown/text-entity quartet: F020 → F009;
- terms acceptance: F016 → F002; application verification token: F002 → F020;
- Passport preferred language и stake dice: F020 → F018;
- TON ledger, withdrawal и ad-account URLs: F019 → F018;
- post-payment giveaway launch/info: F018 → F011; payment-option methods остались F018;
- network setter: F019 → F020; network statistics остались F019.

После correction policy evidence и artifact были полностью regenerated, не отредактированы вручную. Corpus test закрепляет exact root/row shape, schema-derived canonical signature/hash/source line, 22 per-feature count/hash oracles, global owner digest, final-owner agreement positive examples и adversarial tables для Start/Star, Callback/Call/testCall, auth/login/Passport, messages/group calls/stories, giveaways/payments, revenue/withdrawal, network state/statistics, links/WebApp, stickers и folders/invites.

Real app-boundary test использует одну уникальную temp root, копирует только четыре corpus files суммарно 1 725 977 bytes и очищает root через RAII. Он доказывает byte-identical `check`/no-op `generate`, semantic-equivalent JSON whitespace, read-only rejection corrupted output, atomic repair только по явному `generate`, fail-closed stale override evidence и отсутствие fixed sibling temp после success/error.

## Verification and review

Fresh bounded checks:

```text
cargo run --locked --offline --quiet --jobs 2 -p tdlib-registry-gen -- check
RUST_TEST_THREADS=2 cargo test --locked --offline -p tdlib-registry-gen --jobs 2 -- --test-threads=2
RUST_TEST_THREADS=2 cargo test --locked --offline --workspace --all-targets --jobs 2 -- --test-threads=2
cargo clippy --locked --offline --workspace --all-targets --jobs 2 -- -D warnings
cargo fmt --all -- --check
git diff --check
```

Результат до commit: 19 generator tests и 14 core tests passed; product skeleton crates не получили ложных runtime tests. Independent final reviewer проверил semantic boundaries, oracle independence, owner-only shape, read-only/atomic/resource behavior и дал `Approved`, findings отсутствуют. `target` — 117 MiB; corpus temp roots и background generator processes отсутствуют.

## Boundary and next gate

Решение `D-20260715-008` принимает exact owner mapping как reviewed product classification. Изменение schema, rule match set, override candidate/signature, per-feature set или exact owner digest обязано fail closed и требует нового semantic review. Следующий P0 gate добавляет capability/risk/prerequisite/retry classification; constructor/update/auth-state registry, codec/router и runtime остаются отдельными незакрытыми задачами.
