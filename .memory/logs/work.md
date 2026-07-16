# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] completed | W-20260715-084 | Реализован F012 bots/testing slice

- `start_bot_and_wait_reply` фиксирует update boundary до trigger, переиспользует terminal send engine и принимает reply только от exact bot/chat после boundary; reply content redacted до protocol.
- `click_bot_callback` выбирает data по cached message/row/column, поэтому caller не видит payload и не пишет `@type`. TDLib 502, transport uncertainty и pass имеют разные typed outcomes; blind repeat отсутствует.
- Synthetic runtime test покрывает terminal start, correlated reply, redaction canary, answered callback и bot timeout. Recorded outbound/reply IDs задают exact cleanup set; destructive live cleanup остаётся P10.
- Capability data добавляет только `getCallbackQueryAnswer`; inline/game/bot-account/managed-bot остаются universal raw/default-deny, Q001 spec format не изобретён. Contract: [D-20260715-079](../decisions/decisions.md), [`docs/bot-testing-workflow.md`](../../docs/bot-testing-workflow.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P7: F013 Mini Apps.

## [2026-07-15] completed | W-20260715-085 | Реализован F013 Mini App handoff slice

- `prepare_web_app_handoff` возвращает separate Telegram-prepared/Browser-pending state и one-shot owner/TTL handle; raw URL/init data остаётся zeroizing в daemon memory и не попадает в CLI output/file.
- `telegram-webapp-runner` безопасно забирает handle через private socket, передаёт launch adapter только по stdin и принимает bounded closed DOM/bridge/network/JS evidence. Synthetic failed-browser scenario не содержит canary в args/report.
- `close_web_app_handoff` закрывает exact launch отдельным finally step; take/expiry/daemon exit очищают artifact, повторный run требует fresh open. Remote Q001 остаётся explicit.
- Existing `openWebApp/closeWebApp` capability rows переиспользованы, новое method review не понадобилось. Contract: [D-20260715-080](../decisions/decisions.md), [`docs/mini-app-handoff.md`](../../docs/mini-app-handoff.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P7: F014 stickers/custom emoji.

## [2026-07-15] completed | W-20260715-086 | Реализован F014 custom emoji lifecycle

- Закрыт восьмой Tasks-пункт P7: один typed `create|add|delete` input обслуживает plan/apply без caller-visible `@type`; existing `upload_sticker_file` остаётся единственной media-preparation границей.
- Apply проверяет terminal uploaded file, exact owned custom-emoji set и external one-shot approval. Create/add подтверждаются fresh inventory reread; timeout не вызывает blind retry.
- Delete classified destructive и считает cleanup доказанным только после `checkStickerSetNameResultOk`. Synthetic lifecycle test покрывает create, lost-response add reconciliation и delete cleanup; live disposable set остаётся P10 и потребует разрешения владельца перед удалением.
- Capability data ревьюит шесть фактических consumers; итог 112 reviewed/898 default-deny. Остальные sticker/emoji/status/reaction методы остаются generated raw surface. Contract: [D-20260715-081](../decisions/decisions.md), [`docs/sticker-set-workflow.md`](../../docs/sticker-set-workflow.md).
- Следующий Tasks-пункт P7: F015–F018 stories/calls/live, account settings, Business и payments/digital assets.

## [2026-07-15] completed | W-20260715-087 | Реализован F015 story/call/live slice

- Combined F015–F018 Tasks-пункт разбит в `plans.md` на четыре feature-подпункта; закрыт F015 без media engine и method-family wrappers.
- Typed photo-story plan/apply проверяет terminal upload, privacy shape и exact external approval. Temporary post response подтверждается fresh story reread; lost response возвращает active-story candidates/uncertain без повторного publish.
- Delete проверяет `can_be_deleted` и exact active-snapshot cleanup. Group-call inspect/leave использует desired-state probe; timeout может стать verified только по `is_joined=false`.
- Capability data добавляет шесть фактических consumers; итог 118 reviewed/892 default-deny. Join signaling/tgcalls adapter остаётся Q001, live side effects — P10. Contract: [D-20260715-082](../decisions/decisions.md), [`docs/story-call-workflow.md`](../../docs/story-call-workflow.md).
- Следующий Tasks-пункт P7: F016 account settings.
