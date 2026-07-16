# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] completed | W-20260715-088 | Реализован F016 account settings slice

- Notification scope read/set использует partial patch поверх fresh full snapshot; omitted values сохраняются, no-op не dispatch-ится, success/timeout подтверждается reread equality.
- `active_sessions` возвращает exact target ID и closed status flags, но не IP/location/device/platform strings. Synthetic canaries отсутствуют в serialized result.
- `plan/apply_terminate_session` classified `auth_security`, использует external exact approval, fresh preflight и запрет current session. Lost response reconciles exact ID disappearance без повторного terminate.
- Capability data добавляет четыре consumers; итог 122 reviewed/888 default-deny. Secret-bearing password/recovery methods остаются без ordinary JSON consumer. Contract: [D-20260715-083](../decisions/decisions.md), [`docs/account-settings-workflow.md`](../../docs/account-settings-workflow.md).
- Следующий Tasks-пункт P7: F017 Business.

## [2026-07-15] completed | W-20260715-089 | Реализован F017 Business slice

- Daemon выводит `regular_user|bot` из verified `getMe` и использует account kind в общем raw/workflow policy; caller не может объявить bot scope сам.
- `business_connection` и `send_business_text` требуют exact connection ID. Перед send перечитываются enabled/right flags; output редактирует customer text и metadata.
- Lost send response вызывает только refresh того же connection. Disconnect/right loss даёт incomplete `capability_lost`, сохранившееся право — `uncertain`; повторного send нет.
- Synthetic SC001/SC002 покрывают одинаковый chat ID в двух connections, cross-ID rejection, redaction и single dispatch. Capability data добавляет два consumers; итог 124 reviewed/886 default-deny. Contract: [D-20260715-084](../decisions/decisions.md), [`docs/business-workflow.md`](../../docs/business-workflow.md).
- Следующий Tasks-пункт P7: F018 payments/digital assets.

## [2026-07-15] completed | W-20260715-090 | Реализован F018 Stars payment slice

- `star_balance` получает current owner через TDLib и возвращает только fresh `starAmount`; transaction identifiers/details не сериализуются.
- Plan/apply поддерживает только Stars invoice: fresh form/seller/amount/balance входят в exact financial plan, credentials/order/shipping/tip не принимаются из CLI, external one-shot approval обязателен.
- Success/timeout завершается fresh ledger read. Только новая exact seller/amount purchase transaction даёт confirmed; иначе outcome uncertain/verification-required без repeat. Invoice name и verification URL не попадают в output.
- Synthetic SC001/SC002 и workflow discovery green. Capability data добавляет три consumers; итог 127 reviewed/883 default-deny. Contract: [D-20260715-085](../decisions/decisions.md), [`docs/stars-payment-workflow.md`](../../docs/stars-payment-workflow.md).
- Следующий Tasks-пункт P7: F019 statistics/resources.

## [2026-07-15] completed | W-20260715-091 | Реализован F019 statistics/resource slice

- Existing `chat_statistics` уже закрывает capability-first SC001/SC002: async tokens идут до data/error, repeat/timeout остаются partial с lineage, denial не превращается в not_found.
- Новый `resource_statistics` выполняет только три read call: fast storage, database и network. Output агрегирует bytes/sizes/counts и полностью редактирует opaque database report.
- Optimize/reset/export/cache механизмы не добавлены: mutations остаются generated raw/default-deny и никогда не запускаются из read workflow.
- Synthetic graph/capability/resource tests и discovery green. Capability data добавляет три resource consumers; итог 130 reviewed/880 default-deny. Contract: [D-20260715-086](../decisions/decisions.md), [`docs/members-statistics-workflow.md`](../../docs/members-statistics-workflow.md).
- Следующий Tasks-пункт P7: F020 platform utilities.

## [2026-07-15] completed | W-20260715-092 | Реализован F020 platform utilities slice

- SC001 закрыт existing pin/generated/default-deny gates: новый upstream method меняет единственный schema hash и до review не dispatch-ится; второй classifier не создан.
- `proxy_status` редактирует server/port/comment/credentials/secret и возвращает только ID/enabled/type. Add/edit/ping endpoints не получили model-visible input.
- Tagged enable/disable setter fresh-читает список, сохраняет previous enabled ID для rollback, делает один dispatch и ordered reread. Без нового `connectionStateReady` result partial `connectivity_diverged`.
- Synthetic SC002/redaction/discovery tests green. Capability data добавляет три consumers; итог 133 reviewed/877 default-deny. Contract: [D-20260715-087](../decisions/decisions.md), [`docs/platform-utilities-workflow.md`](../../docs/platform-utilities-workflow.md).
- Следующий Tasks-пункт P7: F021 reliability cross-cutting contract.

## [2026-07-15] completed | W-20260715-093 | F021 подключён как сквозной reliability contract

- Ранее отдельные scheduler/journal/audit/telemetry primitives подключены к production daemon: один shared snapshot обслуживает leases, admission, flood/retry и status; raw call проходит generated classification, scheduler и owner-only audit.
- Common raw executor распознаёт только TDLib 429 `retry after`, ждёт не меньше server delay и делает максимум один safe-read repeat. Synthetic history test доказывает два exact read dispatch; mutation classes generic retry не получают.
- Durable journal начинается до raw/reconcile-workflow side effect. Generic raw mutation остаётся partial `reconciliation_required`; workflow `complete=false`, crash и unknown outcome не возвращаются в queue без reconciliation. Stable machine envelope поднят до v2 для metrics/retry/reconciliation fields.
- Fault matrix flood/timeout/gap/cancellation/crash/unknown outcome и redaction canaries green. Contract: [D-20260715-088](../decisions/decisions.md), [`docs/feature-logic-harness/reliability-policy-observability.md`](../../docs/feature-logic-harness/reliability-policy-observability.md). Перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P7: F022 agent skill.

## [2026-07-15] completed | W-20260715-094 | F022 agent skill обновлён и P7 accepted

- Repo-local skill сохранил on-demand flow `login -> lease -> workflow describe/run -> raw fallback -> continuation -> release` без method catalog; обычные curated workflows не требуют caller-visible `@type`.
- Machine contract уточнён до v2: cold agent проверяет root status и останавливает exact mutation replay при `reconciliation_required=true`, не фабрикует approval и не принимает partial/gap за absence.
- Existing offline history/statistics/sticker/bot/Mini App traces повторно сверены с F022 rubric; raw destructive control заканчивается operator handoff. `tiktoken 0.12.0` измерил 806 `cl100k_base` / 662 `o200k_base`, оба ниже 1500.
- Contract: [D-20260715-073](../decisions/decisions.md), [D-20260715-088](../decisions/decisions.md), [`docs/agent-skill-eval.md`](../../docs/agent-skill-eval.md). Перед коммитом выполняется полный обязательный gate.
- P7 accepted; следующий Tasks-пункт — первый пункт P8 optional MCP.
