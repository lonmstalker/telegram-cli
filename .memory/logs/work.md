# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-15] completed | W-20260715-054 | Capability classification перенесена в одну data table

- Закрыт второй Tasks-пункт P3: reviewed contracts из `docs/capability-notes.md` перенесены в canonical `tools/tdlib-registry-gen/capabilities.json` с risk, account scope, runtime requirements и retry/idempotency class.
- Generator валидирует closed vocabularies, duplicate/unknown method rows и непустые requirements. Каждый schema method получает generated descriptor; отсутствие reviewed row даёт `DefaultDeny` без guessed classification.
- Behavior checks доказывают alignment `CAPABILITIES`/`METHODS`, representative reviewed read и unreviewed deny; generator negative control отклоняет unknown/duplicate rows. Никаких hardcoded count/hash тестов или per-family modules нет.
- Durable contract: [D-20260715-049](../decisions/decisions.md); human reference: [`docs/capability-notes.md`](../../docs/capability-notes.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P3: `version`, `capabilities`, schema search/describe и universal `td call` в core.

## [2026-07-15] completed | W-20260715-055 | Реализован universal core discovery/call

- Закрыт третий Tasks-пункт P3: `telegram_core::raw_api` даёт verified runtime/schema version, generated capabilities, case-insensitive token search, symbol/type describe и один generic `td_call`.
- `td_call` принимает только `ValidatedRequest`, использует transport-owned correlation, возвращает lossless `TdObject` и отклоняет known successful constructor вне declared result family. Unknown method останавливается до transport send.
- Tests используют generated registry и существующий startup transport mock: поиск/describe/default-deny, runtime version и successful `getMe` dispatch проверены без per-method wrapper catalog.
- Durable contract: [D-20260715-050](../decisions/decisions.md); implementation contract: [`docs/core-raw-api.md`](../../docs/core-raw-api.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P3: policy применяется до raw dispatch.

## [2026-07-15] completed | W-20260715-056 | Policy встроена до raw dispatch

- Закрыт четвёртый Tasks-пункт P3: единственная `raw_api::td_call` теперь требует `RawPolicy`; schema validation и generated capability lookup выполняются до любого transport send.
- Reviewed method допускается только при matching account kind и granted risk. Unreviewed row, wrong account и missing risk fail closed отдельными `PolicyError`; runtime requirement не выдаётся за live proof.
- Trust-boundary tests подтверждают positive reviewed read и отсутствие backend send при default-deny; pure policy checks покрывают account/risk negative paths.
- Durable contract: [D-20260715-051](../decisions/decisions.md); implementation contract: [`docs/core-raw-api.md`](../../docs/core-raw-api.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P3: coverage report генерируется из manifest/registry в `docs/tdlib-api-coverage.md`.

## [2026-07-15] completed | W-20260715-057 | Generated coverage report закрыл P3 Acceptance

- Закрыт пятый Tasks-пункт P3: `tdlib-registry-gen` обновляет bounded generated block в `docs/tdlib-api-coverage.md` вместе с Rust registry, а один gate проверяет deterministic equality обоих artifacts.
- Report сопоставляет независимые manifest и generated counts для methods, constructors, updates и authorization states; отдельно показывает capability coverage, reviewed rows и валидный `DefaultDeny` remainder.
- P3 Acceptance подтверждают существующие behavior gates: universal core equality, round-trip всех constructors, lossless update fallback, early runtime mismatch и policy-before-send. Update registry получил прямую проверку принадлежности каждого generated имени семейству `Update`.
- Durable contract: [D-20260715-052](../decisions/decisions.md); implementation evidence: [`docs/tdlib-api-coverage.md`](../../docs/tdlib-api-coverage.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт: P4 разделяет pure `resolve` и explicit `ensure_membership`.

## [2026-07-15] completed | W-20260715-058 | Resolve отделён от explicit membership

- Закрыт первый Tasks-пункт P4: `telegram_core::workflows` предоставляет typed `resolve` и `ensure_membership`; caller не формирует TDJSON `@type`, а оба пути используют общий validated/policy-gated raw call.
- Read resolver покрывает known chat ID, public username и invite preview и тестом не dispatch-ит join methods. Membership dispatch возможен только через отдельную функцию и сохраняет success/request-pending/approval-required/declined/unknown outcomes.
- Capability data дополнена пятью фактическими workflow methods: три `read/safe_read` resolver calls и два `reversible_mutation/reconcile` join calls; account scope консервативно regular-user only.
- TDLib error возвращается sanitized structured error без invite link и не становится ложным `not_found` или membership proof. Contract: [D-20260715-053](../decisions/decisions.md), [`docs/chat-resolution-membership.md`](../../docs/chat-resolution-membership.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P4: повторный `loadChats`, ordered position cache и documented terminal condition.

## [2026-07-15] completed | W-20260715-059 | Реализован terminal-correct chat-list loader

- Закрыт второй Tasks-пункт P4: `load_chat_list` поддерживает typed Main/Archive/Folder target и повторяет policy-gated `loadChats` до documented TDLib error `404` в пределах одного deadline.
- Raw call возвращает internal correlation boundary; runtime применяет preceding events к reducer. Ordered view читается из canonical `chat.positions`, сортируется по `(order, chat_id)` descending и не дублирует cache вторым индексом.
- Integration backend дал две короткие successful load-порции с position updates и только затем `404`; workflow сделал три calls и вернул `(20, 3), (10, 2)`, доказав, что `ok`/short batch не terminal.
- Capability data добавила только фактический `loadChats` consumer (`read`, `safe_read`, regular user). Contract: [D-20260715-054](../decisions/decisions.md), [`docs/chat-list-loading.md`](../../docs/chat-list-loading.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P4: full chat workflow с cache wait, optional open lease и full info.

## [2026-07-15] completed | W-20260715-060 | Реализован full chat inspection workflow

- Закрыт третий Tasks-пункт P4: `inspect_chat` принимает ID/username/public link/invite, применяет resolver updates до response boundary, ждёт reducer cache и выбирает exact full-info request по cached `ChatType`.
- Optional open path использует scoped `OpenChatLease`: success test доказал `searchPublicChat -> openChat -> getSupergroupFullInfo -> closeChat`, failure test — `closeChat` после TDLib full-info error.
- Private invite с `chat_id=0` возвращает `MembershipRequired`, `complete=false`; negative test подтверждает отсутствие join/open dispatch. Public link parser отклоняет invite-shaped link вместо смены ветки.
- Capability data дополнена тремя full-info reads и двумя presence calls, без method-family module. Contract: [D-20260715-055](../decisions/decisions.md), [`docs/chat-inspection-workflow.md`](../../docs/chat-inspection-workflow.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P4: history/search pagination до count/date/no-progress boundary.

## [2026-07-15] completed | W-20260715-061 | Реализована honest history/search pagination

- Закрыт четвёртый Tasks-пункт P4: `chat_history` следует derived minimum-ID cursor, `search_chat_messages` — exact returned cursor; оба принимают bounded count/min-date/page-limit options и один deadline.
- Short history page при limit 100 продолжила второй call и завершилась только по requested count. Search test доказал returned cursor forwarding и partial `NoProgress` на cursor repeat; отдельные cases закрыли date и cursor-0 exhaustion.
- Approximate `total_count` не используется, duplicate boundary IDs не дублируют output, message raw JSON/order сохраняются. Capability data добавила только `getChatHistory`/`searchChatMessages` как regular-user read/safe-read.
- Contract: [D-20260715-056](../decisions/decisions.md), [`docs/message-pagination.md`](../../docs/message-pagination.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P4: members/statistics capability fields, async graph tokens и freshness rules.

## [2026-07-15] completed | W-20260715-062 | Реализованы members/statistics chains и freshness evidence

- Закрыт пятый Tasks-пункт P4: `supergroup_members` проверяет reducer-owned `can_get_members`, продолжает short pages и различает count/exhausted от partial no-progress.
- `chat_statistics` проверяет `can_get_statistics`, раскрывает recursive async graphs до data/error, сохраняет token lineage и возвращает partial с unresolved token при repeat/deadline.
- Result содержит capability cache sequence, collection `observed_at` и явный `ServerSnapshot`; completion не выдаётся за real-time freshness. Capability data добавила только фактические `getSupergroupMembers`/`getStatisticalGraph` consumers.
- Contract: [D-20260715-057](../decisions/decisions.md), [`docs/members-statistics-workflow.md`](../../docs/members-statistics-workflow.md). Targeted core workflow tests и Clippy green; перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P4: file/sticker/bot/Web App workflows с ожиданием terminal updates.

## [2026-07-15] completed | W-20260715-063 | Реализованы representative terminal-update workflows

- Закрыт шестой Tasks-пункт P4: typed `download_file`/`upload_sticker_file` не требуют caller-authored `@type` и принимают success только из terminal response field или matching ordered `updateFile`.
- `start_bot` ждёт reducer-owned send succeeded/failed после temporary ID; acknowledgement не terminal, deadline возвращает uncertain без retry.
- Scoped `WebAppLease` держит launch URL в redacted/zeroizing container, коррелирует `updateWebAppMessageSent` по launch ID и парно вызывает `closeWebApp`; browser/UI success не заявлен.
- Synthetic runtime test прошёл настоящий transport/reducer path для всех четырёх chains. Capability data добавила только пять фактически dispatch-имых методов. Contract: [D-20260715-058](../decisions/decisions.md), [`docs/terminal-domain-workflows.md`](../../docs/terminal-domain-workflows.md).
- Следующий Tasks-пункт P4: gap marker и обязательный resync после update lag.

## [2026-07-15] completed | W-20260715-064 | Gap/resync закрыл P4 Acceptance

- Закрыт седьмой Tasks-пункт P4: explicit `mark_update_gap` сохраняет affected local sequence, а state-dependent workflows fail closed с `ResyncRequired` до dispatch.
- `resync_after_gap` выполняет reviewed `getCurrentState`, отделяет response boundary и atomically заменяет reducer только после полной validation snapshot; invalid snapshot оставляет прежнее gapped state.
- Behavior tests доказывают gap persistence, atomic rollback/replacement, workflow block-before-send и successful snapshot recovery. Capability data добавила только фактический `getCurrentState` consumer.
- Все P4 Acceptance-критерии закрыты существующими tests: resolver-before-inspection, short-page non-terminal, paired open/close и send succeeded/failed wait. Contract: [D-20260715-059](../decisions/decisions.md), [`docs/update-gap-resync.md`](../../docs/update-gap-resync.md).
- Следующий Tasks-пункт: P5 per-account/per-chat/method-class budgets и bounded flood-aware backoff.

## [2026-07-15] completed | W-20260715-065 | Реализованы multi-scope budgets и flood backoff

- Закрыт первый Tasks-пункт P5: существующий per-account FIFO scheduler расширен explicit `ScopeBudget` для account/chat/generated risk class с queue cap и rate window; никаких guessed production defaults нет.
- Generated capability data определяет method class; unreviewed method и missing class budget fail closed до ticket. Read concurrency/FIFO mutation barrier сохранены.
- Flood scope block не короче server delay; bounded jitter не превышает configured automatic maximum, а слишком длинный delay отключает automatic retry вместо truncation.
- Tests покрывают независимые rate dimensions, каждую queue dimension, generated classification/default-deny, FIFO/cancellation и flood bounds. Contract: [D-20260715-060](../decisions/decisions.md), [`docs/daemon-scheduler.md`](../../docs/daemon-scheduler.md).
- Следующий Tasks-пункт P5: retry только для safe reads и convergent desired-state operations.

## [2026-07-15] completed | W-20260715-066 | Retry ограничен safe reads и exact desired state

- Закрыт второй Tasks-пункт P5: один `telegram_core::retry` executor читает `RetryClass` из generated capability data и fail-closed отклоняет `reconcile`, `never`, unknown и default-deny methods до dispatch.
- Safe read выполняет bounded retry только после полного supplied delay. Convergent operation получает один immutable request, перед повтором обязательно probes desired state и не повторяется при unknown/uncertain outcome.
- Tests доказывают minimum delay, same-request identity, probe-before-repeat, reconciliation success и отсутствие attempt для запрещённых classes. Contract: [D-20260715-061](../decisions/decisions.md), [`retry.rs`](../../crates/telegram-core/src/retry.rs).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P5: durable idempotency journal с reconciliation вместо blind retry.

## [2026-07-15] completed | W-20260715-067 | Реализован durable idempotency journal

- Закрыт третий Tasks-пункт P5: `telegram-core::idempotency` сохраняет SHA-256 operation fingerprint и четыре required states в strict owner-only append-only JSONL; daemon открывает его внутри canonical locked profile до runtime dispatch.
- `pending` и outcomes fsync-ятся; restart/torn-tail fault test переводит interrupted dispatch в `uncertain`, после чего blind begin возвращает `ReconcileRequired`.
- Reconciliation хранит только evidence digest; доказанный `Applied` не dispatch-ится снова, `Unknown` остаётся uncertain, `NotApplied` разрешает новый explicit begin. Unsafe/corrupt file и invalid transition fail closed.
- Contract: [D-20260715-062](../decisions/decisions.md), [`docs/idempotency-journal.md`](../../docs/idempotency-journal.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P5: risk scopes read/presence/send/reversible/admin/destructive/financial/auth-security.
