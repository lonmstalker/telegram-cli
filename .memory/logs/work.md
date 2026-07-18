# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-15] completed | W-20260715-096 | Реализован local и authenticated SSH MCP transport

- Второй Tasks-пункт P8 закрыт official `rmcp 2.2.0`: newline stdio проходит MCP 2025-11-25 initialize/list lifecycle, возвращает восемь tools и до call не касается daemon.
- Remote mode переиспользует stdio через OpenSSH restricted forced command. `SSH_CONNECTION`, fixed identity и root-owned exact-mode profile/scope policy обязательны; отдельного network listener нет.
- Scope ceiling проверяется до lease request, затем daemon повторно применяет owner ceiling/generated policy. Reconnect не replay-ит mutation; machine envelope сохраняет partial/reconciliation contract.
- Synthetic stdio trace и negative remote-without-SSH probe green. Contract: [D-20260715-090](../decisions/decisions.md), [`docs/mcp-transport.md`](../../docs/mcp-transport.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P8: brokered login с challenge ID/next action и secret entry вне model-visible transport.

## [2026-07-15] completed | W-20260715-097 | Реализован brokered MCP login и P8 accepted

- Shared machine protocol v3 добавляет typed `next_action` к sanitized `LoginStatus`; MCP `auth.begin/status/wait` возвращают только state/challenge metadata и не принимают secret fields.
- Новый one-shot `telegram-cli login tty <challenge_id>` сверяет current ID до prompt, читает ровно один owner secret через existing protected `/dev/tty` и возвращает управление для следующего MCP wait/status step.
- Stale challenge fail closed; remote owner channel документирован как отдельная authenticated SSH PTY session. MCP остаётся eight-tool adapter без TDLib/core dependency или второго DB owner.
- P8 Acceptance закрыт protocol/CLI/MCP tests, synthetic stdio lifecycle и SSH identity/scope gates. Contract: [D-20260715-091](../decisions/decisions.md), [`docs/mcp-transport.md`](../../docs/mcp-transport.md).
- Следующий Tasks-пункт: P9 reproducible pinned TDLib builds для обоих targets.

## [2026-07-17] work | W-20260717-001 | Начата P10-проверка authorization

- Цель: проверить authorization с product boundary, начиная с offline state/key/broker contracts и существующей encrypted returning session, не выполняя `logOut` и не передавая phone/OTP/2FA агенту.
- Sources: [`docs/authorization-state-machine.md`](../../docs/authorization-state-machine.md), [`docs/database-encryption-key.md`](../../docs/database-encryption-key.md), [`docs/cli-secure-login.md`](../../docs/cli-secure-login.md), [sanitized live digest](../raw/2026-07-17-p10-authorization-returning-check.md).
- Actions: точечные auth/key/daemon/CLI/MCP tests; machine precondition `socket_unavailable`; macOS-only rebuild stale embedded provenance; singleton `telegramd`; CLI login status; read lease acquire/release; idle shutdown.
- Verification: 11 targeted tests passed; CLI envelope v3 дал `state=ready` без challenge; daemon доказал `Ready + getMe`, затем `Draining -> Closed`. Secret/identity/path values не выводились и не сохранялись.
- Boundary: first phone/OTP/2FA login не выполнялся, поэтому P10 authorization scenario и сама фаза остаются pending.
- Next: создать отдельный disposable/new profile без затрагивания returning DB, получить typed first-login challenge и передать exact `telegram-cli login tty <challenge_id>` владельцу через protected TTY; затем проверить returning restart этого profile.

## [2026-07-18] completed | W-20260718-001 | Добавлена двуязычная пользовательская документация

- Добавлены зеркальные source-checkout guides: [`user-guide.ru.md`](../../docs/user-guide.ru.md) / [`user-guide.en.md`](../../docs/user-guide.en.md) и пошаговая authorization: [`authorization-guide.ru.md`](../../docs/authorization-guide.ru.md) / [`authorization-guide.en.md`](../../docs/authorization-guide.en.md).
- User flow покрывает singleton daemon, protected `.env.local`, login status, read lease, `user_profile`, release, machine envelope и graceful `close`; auth flow объясняет TTY, `challenge_id`, все typed `next_action`, QR, returning restart и fail-closed диагностику.
- P9 packaging и P10 first phone/OTP/2FA live acceptance явно оставлены незавершёнными; секреты и local build host в tracked docs не записаны.
- Verification: relative Markdown links, RU/EN section/fence parity и `git diff --check` green; targeted protocol/CLI/daemon authorization tests — 3 passed, 0 failed.

## [2026-07-18] work | W-20260718-002 | Подготовлен isolated first-login challenge

- Цель: продолжить P10 без затрагивания returning DB и передать phone/OTP/2FA только владельцу через protected `/dev/tty`.
- Sources: [`docs/authorization-state-machine.md`](../../docs/authorization-state-machine.md), [`docs/cli-secure-login.md`](../../docs/cli-secure-login.md), [sanitized live digest](../raw/2026-07-18-p10-first-login-challenge.md).
- Actions: подтверждены ignored `.env.local` mode `0600` и готовые local arm64 binaries; создан новый ignored profile с session directories mode `0700`; запущен один `telegramd` с production DC и отдельными DB/files paths.
- Verification: fresh envelope v3 дал `status=partial`, `state=phone_number`, `challenge_id=1`, `next_action=submit_via_protected_channel`; secrets не читались и не передавались.
- Boundary/next: daemon остаётся запущенным; владелец выполняет exact `login tty 1`, после чего нужен свежий status и продолжение challenge chain до `Ready + getMe`, затем returning restart. P10 пока не accepted.

## [2026-07-18] work | W-20260718-003 | Исправлен immediate owner TTY failure

- После prompt owner terminal вернул `SecureTtyFailed`; fresh broker status сохранил исходный phone challenge, поэтому secret не был submitted. Evidence: [sanitized live digest](../raw/2026-07-18-p10-owner-tty-failure.md), проблема [P-20260718-001](../problems/problems.md).
- Post-prompt reader больше не зависит от `poll` events конкретного PTY: nonblocking `/dev/tty` читается bounded retry loop с signal checks; stdin fallback не добавлен.
- Verification: delayed nonblocking-input regression green; весь `telegram-cli` — 6 passed, 0 failed; targeted clippy `-D warnings` и regular arm64 build green.
- Next: владелец повторяет текущий `login tty 1`; только fresh challenge transition закроет P-20260718-001 и продолжит P10.

## [2026-07-18] completed | W-20260718-004 | Разделён echo для phone и auth secrets

- По явной инструкции владельца phone input стал видимым в owner `/dev/tty`; OTP, 2FA, email и registration остались hidden. Durable boundary: [D-20260718-001](../decisions/decisions.md).
- CLI по-прежнему не принимает authorization values через args/stdin/env и передаёт phone в redacted `ProtectedString`; RU/EN user authorization и CLI/MCP contracts синхронизированы.
- Verification: `telegram-cli` 6 passed, protected protocol redaction test passed, targeted clippy `-D warnings`, regular local arm64 build и `git diff --check` green.
- Fresh broker status после build: `phone_number/challenge_id=1/submit_via_protected_channel`; P10 ждёт owner submission.

## [2026-07-18] completed | W-20260718-005 | Human login сведён в одну команду

- Default human `telegram-cli login` теперь запускает существующий owner loop и сам продолжает fresh phone/code/password/email/registration challenges до terminal state; intermediate `LoginSubmitted` не выводится. Durable rule: [D-20260718-002](../decisions/decisions.md).
- JSON/JSONL `login` остался prompt-free sanitized status route для агента/MCP; exact-ID one-shot сохранён только как compatibility/operator surface.
- RU/EN user/authorization guides и CLI contracts обновлены: primary flow — одна команда, ручной `challenge_id` описан только в advanced machine/operator section.
- Verification: CLI 7 passed вне sandbox, protocol login-status redaction 1 passed, clippy `-D warnings`, regular local arm64 build, docs heading parity и `git diff --check` green.
- Live continuation: phone уже принят, fresh state `code/challenge_id=2`; владелец продолжает одной командой `telegram-cli login`.

## [2026-07-18] completed | W-20260718-006 | Echo auth input передан под owner control

- Phone/OTP/email/registration теперь видны в owner TTY. Только cloud password предлагает inline choice `[y/N]`, default visible; hidden mode остаётся opt-in. Durable policy: [D-20260718-003](../decisions/decisions.md).
- Choice parser не аллоцирует normalized copy: случайно введённая в prompt строка zeroize-ится как исходный `ProtectedString`; invalid choice повторяет только безопасный prompt.
- CLI/MCP contracts и RU/EN guides синхронизированы без добавления args/stdin/env/machine-output credential path.
- Verification: CLI 8 passed, protected protocol redaction 1 passed, clippy `-D warnings`, regular local arm64 build, docs parity и `git diff --check` green.
- Fresh continuation state: `code/challenge_id=2`; default human `telegram-cli login` продолжит chain с видимого OTP.

## [2026-07-18] completed | W-20260718-007 | Добавлен timeout-aware resend в human login

- Причина `LoginSubmissionRejected` подтверждена по runtime path: core сохранял `next_type/timeout`, но daemon очищал только pending submission, а CLI завершала flow; `resendAuthenticationCode` нигде не вызывался. Durable policy: [D-20260718-004](../decisions/decisions.md), live follow-up: [P-20260718-002](../problems/problems.md).
- Shared protocol получил metadata-only `LoginCodeResend`; core строит exact typed request, daemon проверяет current challenge, `next_type` и elapsed server timeout. Human CLI до prompt автоматически запрашивает свежий code один раз, а после rejection либо делает resend, либо повторяет ввод без выхода из одной команды.
- RU/EN user/authorization guides и CLI/core contracts объясняют различие OTP lifetime и resend timeout. Secret input, arbitrary TDJSON и error text в wire/output не добавлены.
- Verification: workspace 86 passed + 3 ignored в core и все остальные package suites green; targeted core/protocol/daemon resend tests green; clippy workspace/all-targets `-D warnings`, regular macOS `telegramd`/CLI build, `git diff --check` и wiki journal contract green.
- Live boundary: обновлённый daemon запущен с тем же isolated DB; после restart незавершённый code challenge вернулся в safe `phone_number`, поэтому owner повторяет phone input. Actual Telegram resend и terminal first-login proof ещё pending.

## [2026-07-18] completed | W-20260718-008 | P10 first-login и returning authorization доказаны

- Владелец завершил full human `telegram-cli login` в owner TTY; agent не читал phone/OTP/cloud password. Singleton daemon достиг `Ready`, что по production lifecycle включает `authorizationStateReady`, successful `getMe` и expected-identity check.
- Первый daemon без leases штатно прошёл `Draining -> Closed`. Тот же encrypted profile повторно запущен без secret input и сразу вернул machine envelope `status=ok/state=ready/challenge_id=null/next_action=ready`.
- Sanitized external evidence: [`2026-07-18-p10-first-login-returning-acceptance.md`](../raw/2026-07-18-p10-first-login-returning-acceptance.md). `logOut`/`destroy` не вызывались; identity, phone, OTP и local secret/path values не сохранены.
- P10 authorization slice принят. Общая фаза P10 остаётся pending по остальным live scenarios. Expired-code Telegram resend не наблюдался в successful chain и остаётся узким follow-up [P-20260718-002](../problems/problems.md).

## [2026-07-18] completed | W-20260718-009 | Провалидирован и закрыт authorization review

- Goal: проверить каждое supplied замечание против live repo и pinned TDLib, отклонить бессмысленные/переусложняющие предложения и исправить только подтверждённые дефекты.
- Sources: supplied review text; [`plans.md`](../../plans.md); [`vendor/tdlib/td_api.tl`](../../vendor/tdlib/td_api.tl); authorization transport/core/protocol/daemon/CLI paths; [D-20260718-004](../decisions/decisions.md).
- Actions: закрыты blind replay/late-response reconciliation, boot/profile token collision, re-auth `Ready -> Starting -> verified Ready`, lease generation и stale idle timer, ToS/privacy, QR, email resend, Apple/Google protocol parity, transient error classification, partial machine outcome и reducer Debug. Durable boundary: [D-20260718-005](../decisions/decisions.md); resolved cluster: [P-20260718-003](../problems/problems.md).
- Rejected: resend-delay во время открытого OTP prompt не инвалидирует OTP; generic raw pre-Ready route нарушил бы verified-identity/secret boundary; обязательный monolithic coordinator/LoginDriver refactor и полный fake-TDLib vertical test не являются необходимыми bug fixes.
- Verification: `cargo test --workspace --all-targets -q` — 148 passed, 0 failed, 3 ignored; `cargo clippy --workspace --all-targets -- -D warnings` green; planning/workspace/skeleton/secret gates green. `check-tdlib-registry.py` честно не включён в green claim: существующий dirty generator/registry WIP сообщает stale output и не относится к auth diff.
- Next: live Telegram expired-code resend остаётся узким [P-20260718-002](../problems/problems.md); общая P10 продолжается по остальным scenarios.

## [2026-07-18] completed | W-20260718-010 | Authorization архитектура консолидирована

- Goal: после bugfix-прохода устранить архитектурное дублирование auth ownership и сделать human login loop детерминированно тестируемым без изменения wire/secret/capability behavior.
- Sources: [`plans.md`](../../plans.md), [architecture note](../../docs/brainstorms/2026-07-18-authorization-architecture.md), [`docs/authorization-state-machine.md`](../../docs/authorization-state-machine.md), [D-20260718-005](../decisions/decisions.md), live daemon/CLI source.
- Actions: production startup и server переведены на один daemon-owned coordinator; verified account и auth observation закрыты внутри него; submit/resend объединены одним dispatch/outcome path без silent invariant failures. CLI loop извлечён в generic driver с fake broker/prompter/runtime tests; TTY и socket оставлены тонкими adapters. Durable boundary: [D-20260718-006](../decisions/decisions.md).
- Verification: два TDD compile-red этапа завершены green; `cargo test --workspace --all-targets -q` — 154 passed, 0 failed, 3 ignored; `cargo clippy --workspace --all-targets -- -D warnings`, fmt, diff, planning/workspace/skeleton/secret gates green. Production `telegramd` source содержит ровно одну `AuthorizationMachine` внутри coordinator.
- Next: auth architecture task закрыт. Platform-specific alternative auth journeys и live Telegram expired-code resend остаются отдельным scope; последний отслеживается как [P-20260718-002](../problems/problems.md).

## [2026-07-18] completed | W-20260718-011 | P9 native builds доказаны воспроизводимыми

- Scope: первый Tasks-пункт P9 — reproducible pinned TDLib builds для macOS arm64 и Linux x86_64. Durable contract: [D-20260718-007](../decisions/decisions.md).
- Actions: обе build-команды требуют exact committed source/target provenance, сравнивают SHA-256 нового artifact с reference до публикации и записывают `verified/2`; macOS recipe стабилизирует embedded source/build prefixes, Linux использует pinned container recipe.
- Evidence: [sanitized digest](../raw/2026-07-18-p9-reproducible-native-builds.md), canonical target provenance и fresh provenance-only native pin gate с 19 negative controls.
- Verification: native build guard, commit provenance, input snapshot, inspection parent-death, parent-death cleanup и stale-work recovery suites green. Fresh `--require-local-artifact` не заявлен: Docker daemon недоступен; это не заменяет recorded independent-build proof.
- Next: P9 остаётся in progress; следующий Tasks-пункт — launchd/systemd socket activation, persistent DB и keychain/file-secret integration.
