# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-19] completed | W-20260719-006 | CHAT-005 invite preview проверен live

- Goal: закрыть pending CHAT-005 на явно переданном owner disposable fixture без membership, presence или утечки invite token/raw response.
- Sources: [`live-regression.md`](../../docs/live-regression.md), [D-20260719-001](../decisions/decisions.md), owner-supplied ephemeral fixture и [sanitized live evidence](../raw/2026-07-19-p10-chat-invite-preview.md).
- Execution: fresh committed daemon достиг returning `Ready`; runtime discovery подтвердил отдельный `preview_invite_link`; один успешный вызов под `read` lease дал root/result `complete=true`, kind `channel`, TDLib visibility `non_public`, current access `accessible`, chat ID present, zero temporary access and no join request.
- Projection/safety: наружу проверены только allowlisted keys; description/member IDs/invite link отсутствовали. `ensure_membership`, join, open и send не запускались; deterministic dispatch regression сохраняет exact `checkChatInviteLink`-only path.
- Cleanup: lease released explicitly, daemon `Draining -> Closed`. URL/token, title и chat ID не записаны в Git, terminal evidence или memory.
- Boundary: CHAT-005 accepted; CHAT-006–009 и history/search/members/statistics остаются pending. Общая P10 не accepted.

## [2026-07-19] completed | W-20260719-007 | Terminal leave реализован, rejoin остановлен на approval

- Goal: реализовать typed terminal leave и выполнить запрошенный owner live round-trip
  leave → join по ранее переданной invite link без raw bypass и blind retry.
- Implementation: `leaveChat` reviewed как `reversible_mutation/reconcile`; core ждёт более новый
  reducer membership status, возвращает idempotent already-left без dispatch и сохраняет timeout
  uncertain. Daemon публикует strict `leave_chat` input/workflow; capability registry и docs
  regenerated. Durable semantics: [D-20260719-002](../decisions/decisions.md).
- Verification: deterministic terminal/idempotent/timeout tests green; workspace — 158 passed,
  0 failed, 3 ignored; clippy `-D warnings`, fmt, planning/workspace, secret-output и diff gates
  green. Returning live auth достигла `ready`, все три typed contracts обнаружены.
- Live: leave terminal `verified_left`; повторный `ensure_membership` вернул partial
  `request_pending`. Lease released, daemon `Draining -> Closed`; invite/title/chat ID/raw response
  не сохранены. Evidence:
  [`2026-07-19-p10-chat-membership-roundtrip.md`](../raw/2026-07-19-p10-chat-membership-roundtrip.md).
- Boundary: typed implementation и доказанный leave закрыты, но CHAT-010/P10 task не accepted до
  admin approval и terminal member proof; внешний blocker —
  [P-20260719-002](../problems/problems.md). Join не повторялся и chat-ID bypass не выполнялся.

## [2026-07-19] completed | W-20260719-008 | Async membership receipt/status закрыли CHAT-010

- Goal: не блокировать join до admin approval, вернуть немедленный typed status и принять поздний
  Telegram member update без повторной mutation.
- Implementation: `ensure_membership` разделяет submission/membership completeness и больше не
  сериализует raw result; read-only `membership_status` поддерживает chat ID/invite, fresh group
  probe и response-boundary update application. Journal policy вынесена из server; status не
  journaled, structural ratchets снижены до 2679/2146. Decision correction:
  [D-20260719-002](../decisions/decisions.md).
- Deterministic verification: pending → late `updateSupergroup/member` → status member, exact join
  count 1; closed mappings для current TDLib member statuses и честный unresolved. Workspace —
  163 passed, 0 failed, 3 ignored; clippy/fmt/planning/workspace/skeleton/registry/secret/diff gates
  green.
- Live: returning auth `ready`; discovery/describe подтвердили `membership_status`; один read-only
  status по owner fixture дал complete `member` server snapshot. Lease released, daemon
  `Draining -> Closed`; invite/title/chat ID/raw response не сохранены. Evidence:
  [`2026-07-19-p10-chat-async-membership-status.md`](../raw/2026-07-19-p10-chat-async-membership-status.md).
- Boundary: CHAT-010 accepted, [P-20260719-002](../problems/problems.md) resolved; общая P10 всё ещё
  pending по остальным domain/live-failure scenarios.

## [2026-07-19] completed | W-20260719-009 | A1 resolve применяет response boundary

- Goal: не маркировать reducer-derived supergroup fields как fresh server snapshot, пока updates,
  доставленные до `getChat/searchPublicChat` response, не применены.
- Sources: [`plans.md`](../../plans.md), [`chat-resolution-membership.md`](../../docs/chat-resolution-membership.md), runtime response-boundary contract и явное ТЗ пользователя.
- Actions: `resolve` теперь требует resync, получает correlation boundary, применяет ordered updates
  до него и только затем проецирует `ChatIdentity`; daemon caller обновлён через mutable runtime.
  Deterministic backend доставляет `updateSupergroup` до response и доказывает, что reducer usernames
  вошли в resolved identity.
- Verification: targeted regression green; `cargo test --workspace --jobs 2 -q` — 164 passed,
  0 failed, 3 ignored. Все `scripts/check-*.py` green под bundled Python 3.12.13; системный
  Python 3.9.6 не поддерживает pinned native guard `dataclass(slots=True)`, поэтому не использован
  для этой проверки. `cargo fmt`, `git diff --check` green.
- Next: A2 — связать journal classification с единственным workflow catalog.

## [2026-07-19] completed | W-20260719-010 | A2 каталог workflow стал единственным источником journal policy

- Goal: исключить silent loss idempotency journal при добавлении mutation в discoverable workflow list.
- Sources: [`plans.md`](../../plans.md), [`idempotency-journal.md`](../../docs/idempotency-journal.md),
  `workflow_catalog.rs`, `server.rs` и явное ТЗ пользователя.
- Actions: единая типизированная таблица хранит name, valid JSON input example и explicit
  `journaled` boolean; server discover/list/run и input example читают её. `is_journaled_workflow`
  делает только lookup в этой таблице. Комментарий фиксирует boundary: не-plan/apply mutations
  journaled, plan/apply остаются под one-shot exact hash. Exhaustiveness test проверяет unique
  catalog names, exact lookup/classification и невозможность journal для отсутствующего имени.
- Verification: targeted catalog и input-contract tests green; `cargo test --workspace --jobs 2 -q`
  — 164 passed, 0 failed, 3 ignored. Все `scripts/check-*.py` green под bundled Python 3.12.13;
  `server.rs` ratchet снижен с 2679 до 2550 после фактического сокращения файла, `cargo fmt` и
  `git diff --check` green.
- Next: A3 — вернуть typed migration state для basic group, upgraded to supergroup.

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
