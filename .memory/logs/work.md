# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] completed | W-20260715-068 | Opaque leases заменены typed risk scopes

- Закрыт четвёртый Tasks-пункт P5: `telegram-protocol::RiskScope` содержит восемь planned scopes и сериализуется их stable snake_case names; неизвестные labels больше не принимаются как grants.
- Daemon owner ceiling берётся из `TELEGRAM_RISK_SCOPES` с read-only default. Lease request вне ceiling возвращает `scope_denied`; действующий lease строит account-scoped `RawPolicy` для generated pre-dispatch risk check.
- Behavior test доказывает denied financial self-grant, read-only lease authorization и send denial; existing JSONL lifecycle test использует typed scope. Contract: [D-20260715-063](../decisions/decisions.md), [`docs/daemon-leases.md`](../../docs/daemon-leases.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P5: preview -> plan hash -> external approval.

## [2026-07-15] completed | W-20260715-069 | Реализован external exact-plan approval gate

- Закрыт пятый Tasks-пункт P5: `PlanPreview` связывает high-risk method/risk/retry с SHA-256 exact validated request; общий raw dispatch требует matching `ApprovedPlan` до transport.
- `ApprovalVerifier` использует Ed25519 public key, expiry и nonce. Signing key остаётся у внешнего broker; forged signature, hash mismatch, replay и повтор consumption fail closed.
- Daemon optional загружает только public key из `TELEGRAM_APPROVAL_PUBLIC_KEY_HEX`; missing key оставляет dangerous methods `approval_required`. Tests используют deterministic external signer только как trust-boundary negative control.
- Contract: [D-20260715-064](../decisions/decisions.md), [`docs/external-plan-approval.md`](../../docs/external-plan-approval.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P5: metrics и redacted audit.

## [2026-07-15] completed | W-20260715-070 | Metrics и redacted audit закрыли P5 Acceptance

- Закрыт шестой Tasks-пункт P5: один fixed-shape snapshot покрывает request latency/outcomes, queue depth/rejections, retry/flood delay, update lag, freshness и active/max leases без labels или payload.
- Scheduler/lease manager обновляют принадлежащие им counters; daemon открывает owner-only `.telegramd-audit.jsonl` после profile lock. Audit schema принимает validated request, но сохраняет только generated method/risk и closed operational fields.
- Trust-boundary test записал request с chat ID и secret-shaped description canary: method присутствует, ID/canary отсутствуют, mode `0600`. Existing secret-output scan и полный gate подтверждают отсутствие утечки.
- Все P5 Acceptance-критерии закрыты behavior evidence для write reconciliation, delayed read retry, external signature/replay gate и telemetry redaction. Contract: [D-20260715-065](../decisions/decisions.md), [`docs/telemetry-audit.md`](../../docs/telemetry-audit.md).
- Следующий Tasks-пункт: P6 CLI commands session/status/login/hold/release, schema, call, workflow, events/watch.

## [2026-07-15] completed | W-20260715-071 | Реализован CLI session client

- Первый большой CLI Tasks-пункт P6 разбит в `plans.md` на четыре продуктовых подпункта; закрыт session slice: status, hold и release.
- Protocol root обобщён в closed `DaemonRequest/DaemonResponse`; status сообщает active leases, а существующие acquire/heartbeat/release wire shapes сохранены. CLI использует private JSONL socket и не зависит от core.
- CLI валидирует profile grammar, directory/socket ownership и exact modes до connect; parser принимает closed risk scopes и bounded TTL, daemon сохраняет authoritative повторную проверку.
- Tests доказали command-to-protocol mapping, private socket exchange и daemon acquire/heartbeat/release/status chain. Contract: [D-20260715-066](../decisions/decisions.md), [`docs/cli-session.md`](../../docs/cli-session.md).
- Следующий Tasks-подпункт P6: schema search/describe и universal `td call` через тот же daemon protocol.

## [2026-07-15] completed | W-20260715-072 | CLI получил generated discovery и universal raw call

- Закрыт второй подпункт CLI commands P6: version/capabilities/search/describe и `td call` идут через private daemon protocol; CLI по-прежнему зависит только от `telegram-protocol`.
- Daemon сериализует generated descriptors прямо из core и применяет matching lease/principal policy перед единственным `raw_api::td_call`. Closed codes различают validation/default-deny/account/risk/approval/transport/result failures без payload в error.
- CLI parser принимает один arbitrary JSON request; любой pinned method достигает validator/policy gate, а curated commands не обязаны показывать TDJSON `@type`.
- Tests покрывают CLI grammar, daemon schema search, runtime-required gate и existing core full registry/raw dispatch behavior. Contract: [D-20260715-067](../decisions/decisions.md), [`docs/cli-schema-call.md`](../../docs/cli-schema-call.md).
- Следующий Tasks-подпункт P6: CLI routes для всех реализованных core workflows.

## [2026-07-15] completed | W-20260715-073 | Все реализованные core workflows доступны из CLI route

- Закрыт третий подпункт CLI commands P6: `workflow list/run` через один protocol variant маршрутизирует все 13 P4 core workflows без CLI-side state machine.
- Daemon strict-deserializes closed owned inputs, требует matching lease/principal и вызывает existing core functions. Results сериализуются из typed receipts/envelopes; workflow errors получают closed protocol categories.
- Web App route сохраняет launch URL только в zeroizing core lease, выполняет wait/close и возвращает terminal receipt без URL/init data. Unknown workflow/input fields останавливаются до dispatch.
- CLI/parser, route discovery/input negative tests и существующие core workflow behavior suites green. Contract: [D-20260715-068](../decisions/decisions.md), [`docs/cli-workflows.md`](../../docs/cli-workflows.md).
- Следующий Tasks-подпункт P6: login и events/watch поверх authorization/update broker.

## [2026-07-15] completed | W-20260715-074 | CLI получил typed login status и cursor events

- Закрыт четвёртый подпункт CLI commands P6: `login` возвращает закрытый Rust `LoginState` из существующего authorization machine, не TDJSON object и не challenge values.
- Daemon остаётся доступен через private socket во время interactive authorization, запрещает raw/workflow dispatch до verified Ready и продолжает один DB-owner lifecycle.
- `events watch` требует matching lease и выдаёт bounded sequence/kind/cursor/gap metadata. Retention loss и скрытое workflow consumption маркируются gap; raw update payload не покидает daemon.
- CLI/parser и daemon event-buffer tests green. Contract: [D-20260715-069](../decisions/decisions.md), [`docs/cli-login-events.md`](../../docs/cli-login-events.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P6: human output и стабильный compact JSON/JSONL с versioned error/exit-code contract.

## [2026-07-15] completed | W-20260715-075 | CLI получил stable human/machine output contract

- Закрыт пятый Tasks-пункт P6: default human renderer даёт короткие digests, а `--output json|jsonl` и `TELEGRAM_OUTPUT` выбирают compact machine envelope v1.
- Daemon публикует root workflow completeness из typed core outcomes; incomplete workflow и event gap сериализуются `status=partial`. Structured command/lease/client errors не требуют parsing prose.
- Exit codes закреплены как 0 success/partial-visible, 2 input, 3 unavailable, 4 daemon rejection, 5 protocol/output. Machine errors идут в stdout envelope, human errors — в stderr.
- Golden envelope, output selection, human digest и existing private socket tests green. Contract: [D-20260715-070](../decisions/decisions.md), [`docs/cli-output-contract.md`](../../docs/cli-output-contract.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P6: streaming, cancellation и signal-safe lease release.

## [2026-07-15] completed | W-20260715-076 | Event watch получил heartbeat и deterministic cleanup

- Закрыт шестой Tasks-пункт P6: human/JSONL `events watch` поддерживает continuous cursor polling и обновляет lease по трети returned TTL; пустые polls после baseline не засоряют output.
- SIGINT/SIGTERM только ставят atomic marker. Loop делает explicit release до structured `cancelled`/exit 6; broken pipe освобождает lease и не пишет повторно. JSON mode остаётся one-shot и сохраняет caller ownership.
- Real private socket test проверяет exact heartbeat -> cancellation/pipe -> release request order. Existing cursor gap и machine envelope tests green.
- Contract: [D-20260715-071](../decisions/decisions.md), [`docs/cli-streaming.md`](../../docs/cli-streaming.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P6: secure TTY для OTP/2FA; secrets никогда не flags.
