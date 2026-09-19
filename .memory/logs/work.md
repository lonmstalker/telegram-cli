# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-19] completed | W-20260719-011 | A3 migration basic group не выдаёт guessed membership

- Goal: не оставлять membership workflow на старом basic-group cache после TDLib migration в
  supergroup и не превращать такую ситуацию в `member/not_member` или бесконечный deadline.
- Sources: [`plans.md`](../../plans.md), [`chat-resolution-membership.md`](../../docs/chat-resolution-membership.md),
  pinned `basicGroup.upgraded_to_supergroup_id` schema и явное ТЗ пользователя.
- Actions: cache несёт `Migrated { supergroup_id }`; `membership_status` возвращает тот же typed
  state c `complete=false` после fresh `getBasicGroup`, без probing нового supergroup и без
  guessed membership. `leave_chat` останавливается без dispatch на typed incomplete
  `migration_required { supergroup_id }` receipt.
- Verification: deterministic status и leave cache tests фиксируют ID migration, incomplete
  outcome и отсутствие `leaveChat` dispatch. `cargo test --workspace --jobs 2 -q` — 166 passed,
  0 failed, 3 ignored; все `scripts/check-*.py` green под bundled Python 3.12.13; source-size,
  fmt и diff gates green.
- Next: A4 — удалить повторный chat-type dispatch и reducer wait loops без изменения behavior.

## [2026-07-19] completed | W-20260719-012 | A4 chat workflow refactor сохраняет contracts

- Goal: убрать шесть независимых разборов chat type и три повторённых reducer wait loop без
  изменения terminal/fail-closed semantics.
- Actions: `ChatKindRef` централизует normal/invite type dispatch; missing `is_channel` всё ещё
  invalid для chat-list. Один `wait_reducer_until` сохраняет predicate errors и deadline boundary
  для leave, title apply и message send. `membership_status_with` получает method явно и больше
  не обратным парсингом request discriminator.
- Verification: existing core/workspace tests без добавления acceptance cases — 166 passed,
  0 failed, 3 ignored; bundled Python 3.12.13 `scripts/check-*.py`, fmt/source-size/diff gates
  green. `workflows/mod.rs` сохранён в 2146-line ratchet.
- Next: A5 — one core constructor for title approval request.

## [2026-07-19] completed | W-20260719-013 | A5 title approval request создаётся только core

- Goal: исключить false `ApprovalDenied` при расхождении daemon receipt verification и core plan hash.
- Actions: публичный `ChatTitlePlan::approval_request()` создаёт exact `setChatTitle` request;
  core preview/apply и daemon approval verification используют его. Server больше не собирает
  собственный JSON request. Source-size ratchet `server.rs` снижен с 2550 до 2542 после reduction.
- Verification: existing exact approval/update test, workspace — 166 passed, 0 failed, 3 ignored;
  bundled Python 3.12.13 `scripts/check-*.py`, fmt/source-size/diff gates green.
- Next: A6 — list entry with absent `is_channel` degrades only to unknown.

## [2026-07-19] completed | W-20260719-014 | A6 malformed chat-list entry не рвёт snapshot

- Goal: выполнить documented degraded-list contract, не ослабляя strict resolve/inspection paths.
- Actions: только `chat_list_entry_kind` переводит supergroup без `is_channel` в `unknown`;
  common dispatcher и остальные consumers по-прежнему не получают guessed channel kind.
- Verification: deterministic snapshot с одной malformed entry сохраняет обе entries и одну
  `unknown`; workspace — 167 passed, 0 failed, 3 ignored; bundled Python 3.12.13
  `scripts/check-*.py`, fmt/source-size/diff gates green.
- Next: B1 — external logout graceful close.

## [2026-07-20] completed | W-20260720-001 | B1 external logout завершает daemon штатно

- Goal: `authorizationStateLoggingOut`/`Closing`/`Closed`, пришедшие извне, не должны превращать живой daemon в crash path или отправлять повторный `close`.
- Actions: lifecycle отличает external terminal shutdown от обычной auth loss после `LeaseServer::observe_authorization` (leases уже revoked); startup и interactive broker возвращают отдельный readiness, а running daemon ждёт `authorizationStateClosed` до bounded deadline, затем останавливает transport и завершает `Draining -> Closed` без daemon-initiated `close`.
- Verification: deterministic scripted TDJSON regression воспроизводит `Ready -> LoggingOut -> Closed`, доказывает `DaemonState::Closed` и отсутствие outbound `close`; `cargo test --workspace --jobs 2 -q` — 168 passed, 0 failed, 3 ignored; bundled Python 3.12.13 `scripts/check-*.py`, `cargo fmt --check` и `git diff --check` green.
- Boundary: первичный startup-terminal и interactive-terminal также завершаются cleanly; `UnexpectedAuthorizationState` остаётся для гонок/аномалий, которые не классифицированы terminal state.
- Next: B2 — ParametersRequired без wire challenge token.

## [2026-07-20] completed | W-20260720-002 | B2 ParametersRequired не становится owner challenge

- Goal: не публиковать wire `LoginChallengeId` для внутреннего TDLib parameters generation, который не имеет owner prompt/input.
- Actions: token выдаётся только `AuthorizationStep::Challenge`; coordinator формирует authoritative `LoginStatus` response, которым пользуется daemon server. Для `ParametersRequired` status теперь строго `state=Parameters`, `challenge_id=None`, `next_action=Wait`; server source-size ratchet уменьшен с 2542 до 2535 после упрощения routing.
- Verification: targeted Parameters status regression green; `cargo test --workspace --jobs 2 -q` — 169 passed, 0 failed, 3 ignored; bundled Python 3.12.13 `scripts/check-*.py`, `cargo fmt --check` и `git diff --check` green.
- Next: B3 — retry owner password prompt после definite rejection.

## [2026-07-20] completed | W-20260720-003 | B3 password rejection получает новый owner input

- Goal: after definite 2FA password rejection интерактивный CLI должен запросить новый secret у владельца, а не завершиться ошибкой или replay-ить прежнее значение.
- Actions: LoginDriver обрабатывает `LoginSubmissionRejected` для Password отдельно от code resend: пишет notice «Пароль отклонён, попробуйте ещё раз», не сохраняет secret и возвращается к fresh `LoginStatus -> LoginPrompt`; one-shot handoff сохраняет прежний exact-submit behavior. Secure-login documentation описывает password retry и исключает resend для `429/500`.
- Verification: deterministic Password → rejected → Password → submitted → Ready regression доказывает два независимых owner prompts, exact notice и пустой broker script; `cargo test --workspace --jobs 2 -q` — 170 passed, 0 failed, 3 ignored; bundled Python 3.12.13 `scripts/check-*.py`, `cargo fmt --check` и `git diff --check` green.
- Next: B4 — отдельный protocol code для invalid login input.

## [2026-07-20] completed | W-20260720-004 | B4 invalid login input отделён от stale challenge

- Goal: protocol должен отличать owner input, который невалиден для текущего challenge, от stale/mismatched opaque token.
- Actions: добавлен stable `CommandErrorCode::LoginInputInvalid` (`login_input_invalid` on machine wire). Daemon submit классифицирует `AuthorizationError::InvalidField` и `InputDoesNotMatchState` этим кодом; stale token остаётся `LoginChallengeInvalid`. Human CLI даёт отдельное bounded explanation без input values.
- Verification: deterministic daemon regression проверяет empty phone и Password-in-Phone state → `LoginInputInvalid`, stale token → `LoginChallengeInvalid`; protocol envelope и exact human output покрыты отдельно. `cargo test --workspace --jobs 2 -q` — 172 passed, 0 failed, 3 ignored; bundled Python 3.12.13 `scripts/check-*.py`, `cargo fmt --check` и `git diff --check` green.
- Next: B5 — убрать duplicate resend availability precondition из daemon.

## [2026-07-20] completed | W-20260720-005 | B5 resend availability принадлежит core

- Goal: убрать расходящуюся daemon precondition для `next_delivery_type`, сохранив лишь UX timeout от observed authorization state.
- Actions: daemon `begin_resend` теперь проверяет только elapsed TDLib timeout; затем всегда делегирует availability в `AuthorizationMachine::resend_code`, не меняя его `CodeResendUnavailable` outcome. Email resend path не изменён.
- Verification: regression с elapsed timeout и `next_type=null` проходит daemon gate и получает exact core `CodeResendUnavailable`; `cargo test --workspace --jobs 2 -q` — 173 passed, 0 failed, 3 ignored; bundled Python 3.12.13 `scripts/check-*.py`, `cargo fmt --check` и `git diff --check` green.
- Next: B6 — исправить lifecycle и secure-login documentation.

## [2026-07-20] completed | W-20260720-006 | B6 auth documentation соответствует runtime

- Goal: убрать из lifecycle contract несуществующий `InteractiveAuthorizationRequired` и явно зафиксировать bounded single-owner login timeout.
- Actions: lifecycle documentation описывает actual `Starting` brokered owner path и ссылается на `cli-login-events.md`. Secure-login documentation фиксирует sync 30-second `LoginSubmit` in a single-thread daemon serve loop и 35-second CLI socket timeout только для одного concurrent login client; parallel login не объявлен supported contract.
- Verification: absence obsolete identifier и наличие authoritative links/timeouts checked by focused text search; `cargo test --workspace --jobs 2 -q` — 173 passed, 0 failed, 3 ignored; bundled Python 3.12.13 `scripts/check-*.py`, `cargo fmt --check` и `git diff --check` green.
- Result: B1–B6 блока authorization закрыты; общая P10/live gate остаётся pending по plan boundaries.

## [2026-07-21] completed | W-20260721-001 | CHAT-006 open/close pairing принят live

- Goal: доказать scoped `openChat`/`closeChat` lifecycle на реальной returning session и не
  оставить chat открытым при error/timeout после dispatch.
- Findings: red regressions подтвердили, что timeout ответа `openChat` терял compensating cleanup,
  а full-info timeout использовал уже истёкший deadline для close. Два независимых review
  подтвердили operational impact и полезность отдельного cleanup window; blind повтор
  `closeChat` после uncertain timeout отклонён без desired-state probe.
- Actions: timeout ответа open теперь вызывает один exact compensating close; explicit/Drop
  cleanup получает `max(workflow_deadline, now + 4s)`. CHAT-006 добавлен в living plan и live
  ledger; contract/harness notes синхронизированы.
- Live verification: current-worktree daemon с process-local минимальным `read,presence` ceiling
  выполнил `inspect_chat(open=true)` на public supergroup fixture; root/result complete и
  `used_open_lease=true` получены после cleanup ACK. Lease released, daemon `Draining -> Closed`;
  join/send/message read не выполнялись, fixture/raw response не сохранены.
- Verification: targeted inspection — 4 passed, 0 failed; `cargo test --workspace --jobs 2 -q` —
  175 passed, 0 failed, 3 ignored; workspace clippy `-D warnings`, fmt, diff и все
  `scripts/check-*.py` под bundled Python 3.12.13 green.
- Wiki boundary: work journal rotated, current work/problem contracts valid. Общий
  `rotate-wiki-journal.py --all --check` остаётся red только на ранее зарегистрированной
  immutable historical link [P-20260719-001](../problems/problems.md); архив не переписывался.
- Links: [D-20260721-001](../decisions/decisions.md),
  [P-20260721-001](../problems/problems.md),
  [sanitized live checkpoint](../raw/2026-07-21-p10-chat-open-close.md).
- Next: CHAT-007 — найти существующий folder fixture и доказать terminal folder list contract.

## [2026-09-19] work | W-20260919-001 | Agent UX, profile onboarding и local installer

- Goal: доработать существующий Telegram CLI, сохранив runtime/policy/workflows, дать владельцу one-time setup и агенту самостоятельный reuse.
- Sources: пользовательская постановка, P9, исходный code audit, Fable medium code-quality review.
- Actions: setup/import-env, lazy daemon, --agent/run/call, offline discovery, doctor, skill init, source/bundle installer, README, serial lightweight harness и resource/IPC fixes.
- Verification: 181 default Rust tests + 6 MCP tests; 3 native/live cases ignored. Cold CLI integration, installer/bundle с isolated HOME, skill validator и fmt/clippy. Подробности и Fable usage — [review](../../docs/reviews/2026-09-19-agent-onboarding.md).
- Related: D-20260919-001/002; P-20260919-001/002.
- Next: owner setup в личном терминале; Linux clean-install/native acceptance и P9 upgrade/rollback отдельно. Секреты и реальный аккаунт не изменялись.

## [2026-09-19] work | W-20260919-002 | Готовая установка CLI и skills без toolchain

- Goal: другим пользователям нужна одна установка CLI, TDLib и skills без локальной сборки.
- Sources: уточнение пользователя; P9; [installer](../../scripts/install-release.sh), [bundle packager](../../scripts/package-release.sh), [installation checks](../../scripts/test-install.py).
- Actions: HTTPS downloader готового release, оба skills по умолчанию, SHA-256 до распаковки, проверка путей архива, version/prefix/skill options и portable checksum sidecar; README с полной установкой. macOS arm64 bundle включает только CLI/daemon/native и установщик, без account data.
- Verification: scripts/check.py verify — 13 checks; scripts/test-install.py — source/bundle/download, custom prefix, both skills, PATH без Cargo/Python, checksum/path/download negative cases. CLI/daemon minimum macOS 11.0, TDLib native pin не менялся.
- Boundary: Linux bundle отложен по явному выбору пользователя; конфигурация аккаунта и real Telegram не использовались. Download tests подменяют только HTTPS, выполняют настоящие установщик и бинарники.
- Next: code-quality review, публикация macOS release и проверка скачивания с GitHub в isolated HOME; Linux и signing остаются отдельной работой.

## [2026-09-19] work | W-20260919-003 | macOS release: public download acceptance

- Goal: опубликовать готовую установку для других пользователей и проверить реальную ссылку.
- Actions: release v0.1.0 из cf9050d, три assets (installer, bundle, SHA-256); first release только macOS arm64 по выбору пользователя.
- Verification: anonymous GitHub download + запуск в isolated HOME с PATH без build tools; CLI doctor/discovery и оба global skills прошли. TDLib/CLI/daemon minimum macOS 11.0, native dependencies только system libraries.
- Evidence: [public release install](../raw/2026-09-19-macos-release-install.md); Fable medium corrections и usage — [review](../../docs/reviews/2026-09-19-agent-onboarding.md).
- Next: Linux bundle и platform acceptance отдельно; owner setup выполняет пользователь, существующие account files не менялись.

## [2026-09-19] work | W-20260919-004 | Исправление QR onboarding

- Goal: дать владельцу завершить QR login без ожидания несуществующего OTP.
- Sources: P9, пользовательский defect report, [Telegram QR contract](https://core.telegram.org/api/qr-login), [review](../../docs/reviews/2026-09-19-qr-login.md).
- Actions: terminal QR вместо ссылки, исправление nonblocking TTY writes, понятная инструкция и RU/EN docs, patch version 0.1.1.
- Verification: 13-check harness, PTY refresh/privacy regression, old-version negative check, Vision decode двух synthetic QR, release/installer checks. Fable medium замечания обработаны, usage записан в review.
- Related: P-20260919-003. Auth state/DB/key не изменялись; native library переиспользована из установленного pinned bundle.
- Next: публикация macOS patch и установка; владелец повторяет telegram-cli login и сканирует QR на телефоне.
