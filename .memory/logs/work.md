# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] completed | W-20260715-077 | Реализован protected TTY login

- Закрыт седьмой Tasks-пункт P6: `login tty` читает phone/OTP/2FA/email/registration только из `/dev/tty` с отключённым echo; parser отвергает любые secret-shaped дополнительные arguments.
- Daemon authorization broker выдаёт non-secret challenge ID и преобразует закрытый `LoginInput` в existing core machine request. Stale, wrong-kind и pending submissions fail closed; raw TDJSON login route не создавался.
- Signal handler только ставит marker; nonblocking poll позволяет RAII guard восстановить echo. Input/frame buffers zeroize, protected Debug и machine responses не содержат secret values.
- Trust-boundary tests проверяют rejected argument и redaction canary; targeted CLI/protocol/daemon suites и Clippy green. Contract: [D-20260715-072](../decisions/decisions.md), [`docs/cli-secure-login.md`](../../docs/cli-secure-login.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P6: compact agent skill и cold-agent eval.

## [2026-07-15] completed | W-20260715-078 | Compact agent skill закрыл P6 Acceptance

- Закрыт восьмой Tasks-пункт P6: `.agents/skills/telegram-cli` учит cold agent одному workflow без API encyclopedia — acquire, discover, execute/continue, release — и запрещает второй DB owner, secret input, self-approval и false terminal claims.
- Protocol/CLI/daemon добавили `workflow describe` с machine-readable input example; behavior test проверяет, что каждый published example strict-deserializes в реальный route input.
- Offline eval artifact фиксирует passing history/statistics/sticker/bot/Mini App traces и raw/destructive controls. Одноразовый pinned `tiktoken 0.12.0` дал 774 cl100k/633 o200k tokens, без repo dependency.
- Все P6 Acceptance-критерии закрыты: 1010 raw methods достигают одного CLI gate, все 13 current core workflows list/describe/run доступны, machine decisions prose-free. Contract: [D-20260715-073](../decisions/decisions.md), [`docs/agent-skill-eval.md`](../../docs/agent-skill-eval.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт: P7 F007 users/contacts/profile; F008 chats/folders/topics; F009 messages/search; F010 files/media.

## [2026-07-15] completed | W-20260715-079 | Реализован F007 user/profile slice

- Первый большой Tasks-пункт P7 разбит в `plans.md` на четыре feature-подпункта; закрыт F007 users/contacts/profile без per-method modules.
- Core `user_profile` делает resolver/hydration для self/ID/public username, возвращает ordered/server freshness и explicit private-field availability без phone/birthdate/note/business values.
- `update_profile_name` использует desired-state shortcut и matching ordered update terminal proof; post-dispatch deadline остаётся uncertain. CLI получает оба route через existing generic workflow list/describe/run.
- Capability table ревьюит `getMe/getUser/setName`; все остальные F007 methods остаются universal raw/default-deny. Synthetic runtime test проверяет resolver order, private canaries и verified update. Contract: [D-20260715-074](../decisions/decisions.md), [`docs/user-profile-workflow.md`](../../docs/user-profile-workflow.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P7: F008 chats/folders/topics.

## [2026-07-15] completed | W-20260715-080 | Реализован F008 chat/folder/topic slice

- Existing `resolve_chat`, terminal-correct `load_chat_list` и paired `inspect_chat` закрывают read-only resolve, main/archive/folder list и open/close contracts без нового chat-family слоя.
- `forum_topics` продолжает short pages по exact returned cursor triple, дедуплицирует topics и отличает count/exhausted от repeated-cursor partial result.
- `set_forum_topic_closed` использует desired-state shortcut и server-state reconciliation после dispatch/timeout; mismatch остаётся uncertain. Оба route доступны через existing generic workflow discovery/run.
- Capability data добавляет только `getForumTopic/getForumTopics`; existing admin toggle contract переиспользуется, остальные F008 методы остаются universal raw/default-deny. Behavior tests и полный mandatory gate green. Contract: [D-20260715-075](../decisions/decisions.md), [`docs/forum-topic-workflow.md`](../../docs/forum-topic-workflow.md).
- Следующий Tasks-пункт P7: F009 messages/search.

## [2026-07-15] completed | W-20260715-081 | Реализован F009 message/search slice

- Existing history/search pagers сохраняют short-page/cursor completion и теперь требуют cached chat; `mark_read=false` не вызывает presence, explicit true выполняет один `viewMessages` только после complete page.
- Protected chat content заменяется closed marker до protocol; canary test доказывает отсутствие payload. `send_text_message` строит request внутри core и ждёт matching succeeded/failed update.
- Response/terminal timeout даёт `uncertain/complete=false`; single-dispatch test доказывает отсутствие blind resend. CLI получает route через existing generic discovery/run.
- Capability data добавляет только `sendMessage/viewMessages`; остальные F009 методы остаются universal raw/default-deny. Contract: [D-20260715-076](../decisions/decisions.md), [`docs/message-workflow.md`](../../docs/message-workflow.md).
- Следующий Tasks-пункт P7: F010 files/media.

## [2026-07-15] completed | W-20260715-082 | Реализован F010 file/media slice

- Existing async download/upload engine теперь сверяет known actual/expected size с transferred bytes; file ID/progress/terminal flag с size mismatch не дают ложный complete.
- `cancel_download` использует desired-state `getFile -> cancel -> getFile`, включая timeout reconciliation; offset/limit остаются bounded resume contract.
- Daemon принимает local/generated sources только как absolute canonical regular files внутри configured `TDLIB_FILES_DIR`; path traversal, outside path, missing root и symlink escape rejected до TDLib.
- Capability data добавляет `getFile/cancelDownloadFile`; остальные F010 методы остаются universal raw/default-deny. Q001 remote artifact provider честно отложен до P9. Contract: [D-20260715-077](../decisions/decisions.md), [`docs/file-transfer-workflow.md`](../../docs/file-transfer-workflow.md).
- Следующий Tasks-пункт P7: F011 groups/channels/moderation; F012 bots/testing; F013 Mini Apps; F014 stickers/custom emoji.

## [2026-07-15] completed | W-20260715-083 | Реализован F011 groups/channels/moderation slice

- Existing resolve/membership/members contracts переиспользованы без chat-family layer: read не join-ит, pending invite не равен membership, short/no-progress page не становится ложным terminal result.
- `plan_chat_title` требует fresh cached right, фиксирует current/desired/sequence и exact generated plan hash; `apply_chat_title` revalidates state и ждёт matching ordered update либо возвращает uncertain.
- Закрыт найденный P6 wire gap: `td preview`, optional one-shot approval в raw/workflow protocol и boot-lived public verifier делают admin route исполнимым без signing key в CLI/daemon. Receipt debug-redacted и exact-request bound.
- Capability data добавляет только `setChatTitle`; остальные moderation/invite/admin методы остаются universal raw/default-deny. Contract: [D-20260715-078](../decisions/decisions.md), [`docs/chat-administration-workflow.md`](../../docs/chat-administration-workflow.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P7: F012 bots/testing.
