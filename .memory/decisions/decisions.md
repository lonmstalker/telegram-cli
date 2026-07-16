# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-077 | File completion требует terminal state и scoped local path

- Async download/upload завершены только по response или matching ordered `updateFile` с terminal flag. Известный actual/expected size сверяется с transferred bytes; отсутствие размера маркируется отдельно, mismatch не становится complete.
- Download offset/limit остаётся resume primitive. Cancel — desired-state operation: already inactive не dispatch-ится, request/timeout завершается только после `getFile` inactive probe, иначе uncertain.
- Local/generated source canonicalized на daemon и обязан быть regular file внутри configured `TDLIB_FILES_DIR`; outside, missing root и symlink escape fail before TDLib. ID/remote source не превращается в filesystem path.
- Remote artifact provider остаётся Q001/P9; arbitrary client path не используется как server path. Остальные file families остаются generated raw/default-deny без отдельного file manager.

## [2026-07-15] accepted | D-20260715-078 | Chat administration использует exact plan поверх общего raw gate

- Read resolve, explicit membership и members pagination остаются отдельными existing workflows. Pending invite не становится membership proof; gap или отсутствие fresh right блокируют admin mutation.
- Title configuration — bounded planner/apply pair: plan фиксирует current/desired/sequence и hash exact generated `setChatTitle`; apply заново проверяет snapshot/right/hash и завершается только newer matching `updateChatTitle`, иначе uncertain без повтора.
- External receipt проходит protocol к daemon, но capability расходуется только common `td_call` для matching schema-validated request. Daemon хранит public verifier весь boot; signing key и self-approval surface отсутствуют.
- Прочие group/channel/moderation методы остаются достижимы через generated raw API и default-deny до consumer-driven review; per-method Rust family layer не создаётся.

## [2026-07-15] accepted | D-20260715-079 | Bot test коррелирует ordered reply и не раскрывает callback payload

- Test boundary — reducer sequence, записанный до trigger. Pass требует terminal send и subsequent incoming message от exact bot в exact chat; timeout continuous window — terminal failed test, gap/send uncertainty — incomplete.
- Reply output сохраняет IDs, content constructor и callback count, но не text/button labels/data. Callback выбирается recorded message/row/column, а payload формируется только внутри core.
- `getCallbackQueryAnswer` вызывается один раз: TDLib 502 — доказанный bot timeout, local response deadline — uncertain. Автоматический повтор отсутствует.
- Declarative spec Q001 и широкая cleanup-команда не вводятся: exact outbound/reply IDs задают owned artifact set, а остальные bot families остаются generated raw/default-deny.

## [2026-07-15] accepted | D-20260715-080 | Mini App init data передаётся one-shot in-memory handle

- `prepare_web_app_handoff` disarm-ит scoped Web App lease только после помещения URL в daemon-owned zeroizing store. CLI получает launch ID, owner-bound opaque handle и 60-second TTL, но не URL/init data.
- Runner забирает artifact один раз через existing private `0600` profile socket и передаёт secret browser adapter только по stdin. Expiry, take и daemon exit удаляют secret; persistent launch file не создаётся.
- Browser adapter возвращает только closed assertion/JS-error counters. Telegram prepared, browser pass/fail и exact close — отдельные proofs; нулевые assertions не считаются UI evidence.
- Remote topology Q001 не решается локальным broker: artifact-take route runner-only и не становится MCP tool. После любого browser outcome caller закрывает exact launch в finally.

## [2026-07-15] accepted | D-20260715-081 | Custom emoji lifecycle использует один typed action и fresh probes

- Curated surface состоит из `plan_custom_emoji_set`/`apply_custom_emoji_set` с tagged `create|add|delete`; TDJSON constructors формируются внутри core. Остальной sticker/emoji/reaction surface остаётся в generated raw registry, без method-family слоя.
- Media preparation принадлежит F010: create/add принимают только file ID, заново проверяют `getFile.remote.is_uploading_completed` и затем требуют external exact-plan approval. Create/add classified `admin`, delete — `destructive`.
- Create/add после response или timeout перечитывают exact set/inventory и не повторяют mutation вслепую. Delete проверяет ownership exact `set_id/name` и доказывает cleanup только через доступность имени; иначе результат `uncertain`.
- Contract: [`crates/telegram-core/src/workflows.rs`](../../crates/telegram-core/src/workflows.rs), [`tools/tdlib-registry-gen/capabilities.json`](../../tools/tdlib-registry-gen/capabilities.json), [`docs/sticker-set-workflow.md`](../../docs/sticker-set-workflow.md).

## [2026-07-15] accepted | D-20260715-082 | Story proof отделён от tgcalls signaling

- Photo story использует один typed `post_photo|delete` plan/apply surface поверх F010 file proof. Core строит content/privacy constructors; post classified `admin`, delete — `destructive`, оба расходуют external exact approval.
- Temporary story response не terminal: complete требует fresh exact `getStory` с `is_being_posted=false`. Lost response запускает только active-story reread и остаётся uncertain с candidate IDs; duplicate post отсутствует.
- Delete cleanup требует confirmed response и отсутствия exact ID в fresh active snapshot. Group-call surface ограничен safe inspect/leave; leave success/timeout доказывается повторным `getGroupCall.is_joined=false`.
- `groupCallJoinParameters.payload`, join response и media sockets не проходят CLI JSON. Выбор tgcalls adapter/consenting fixture остаётся Q001/P10, без самодельного WebRTC transport.
- Contract: [`crates/telegram-core/src/workflows.rs`](../../crates/telegram-core/src/workflows.rs), [`docs/story-call-workflow.md`](../../docs/story-call-workflow.md).

## [2026-07-15] accepted | D-20260715-083 | Account settings используют partial merge и exact session target

- Notification setter принимает optional fields, сначала читает full server scope и изменяет только managed values. Empty/already-converged patch не dispatch-ится; success/timeout terminal только после full reread equality.
- Session inventory сериализует exact ID и closed targeting/status flags, но исключает IP, location, device/platform/system/application metadata. Broad termination route отсутствует.
- `terminateSession` classified `auth_security`, требует external exact approval и fresh preflight. Current session fail closed; success/timeout перечитывает весь список, absence exact ID даёт verified, presence/read failure — uncertain без повтора.
- Password/recovery/OTP и другие Ready-state secrets не получили ordinary workflow input. До появления protected consumer их методы остаются generated raw/default-deny.
- Contract: [`crates/telegram-core/src/workflows.rs`](../../crates/telegram-core/src/workflows.rs), [`docs/account-settings-workflow.md`](../../docs/account-settings-workflow.md).

## [2026-07-15] accepted | D-20260715-084 | Business policy использует runtime identity и explicit connection

- Account scope берётся только из verified TDLib `getMe.type`: regular user и bot получают разные generated policy. Неизвестный/deleted type fail closed; protocol input не может повысить account kind.
- Curated surface ограничен exact connection inspect и text send. Connection ID обязателен в каждом request, safe default и cross-connection cache не создаются; весь остальной Business API остаётся generated raw/default-deny.
- Send preflight проверяет fresh `is_enabled/can_reply`. Receipt содержит connection/chat/message ID и outcome, но не customer content. После timeout выполняется один read-only refresh и mutation никогда не повторяется.
- TDJSON constructors и `@type` остаются внутри core; CLI принимает typed workflow fields. Contract: [`crates/telegram-core/src/workflows.rs`](../../crates/telegram-core/src/workflows.rs), [`apps/telegramd/src/identity.rs`](../../apps/telegramd/src/identity.rs), [`docs/business-workflow.md`](../../docs/business-workflow.md).

## [2026-07-15] accepted | D-20260715-085 | Curated payment boundary ограничен Stars invoice без credentials

- Полный payment/gift/Premium/Passport/affiliate API остаётся generated raw/default-deny. Curated route добавлен только для current-owner balance и Stars invoice, где TDLib принимает `credentials=null`.
- Plan строится из fresh form и ledger, содержит seller/amount/form ID и hash exact request. Apply заново строит plan; изменившийся form или amount не совпадёт с external one-shot approval.
- Apply не принимает card token, saved credentials, order/shipping, tip, Passport data или provider URL. Invoice name связан request hash, но не отражается в result; verification URL заменяется closed outcome.
- Accepted/timeout response не является settlement proof: completion требует новую exact purchase transaction в fresh ledger. Отсутствие доказательства остаётся uncertain без automatic resend. Contract: [`crates/telegram-core/src/workflows.rs`](../../crates/telegram-core/src/workflows.rs), [`docs/stars-payment-workflow.md`](../../docs/stars-payment-workflow.md).

## [2026-07-15] accepted | D-20260715-086 | Resource read агрегируется без implicit optimization

- Existing chat statistics walker остаётся единственным graph engine: capability проверяется из ordered full-info, async lineage раскрывается до data/error, repeat/timeout сохраняет partial proof.
- Resource snapshot использует только `getStorageStatisticsFast`, `getDatabaseStatistics` и `getNetworkStatistics`. Network entries суммируются; opaque database report не сериализуется.
- `optimizeStorage`, reset/add network statistics, export/cache/artifact subsystem не создаются. Они остаются generated raw/default-deny до explicit mutation/export consumer; read не имеет side effects.
- Contract: [`crates/telegram-core/src/workflows.rs`](../../crates/telegram-core/src/workflows.rs), [`docs/members-statistics-workflow.md`](../../docs/members-statistics-workflow.md).

## [2026-07-15] accepted | D-20260715-087 | Proxy utility работает только с existing ID и ordered connectivity proof

- Full platform coverage обеспечивают existing generated registry, single schema hash и default-deny data. Localization/options/themes/log/custom/test family wrappers и отдельный classifier не создаются.
- Curated proxy surface не принимает endpoint material: status сериализует только ID/enabled/type; enable/disable выбираются tagged input, поэтому missing action не означает disable.
- Setter rereads exact proxy list, выполняет mutation один раз и возвращает previous enabled ID как rollback target. Completion требует desired list state и более новый ordered `connectionStateReady`; divergence/timeout не повторяют mutation.
- Contract: [`crates/telegram-core/src/workflows.rs`](../../crates/telegram-core/src/workflows.rs), [`docs/platform-utilities-workflow.md`](../../docs/platform-utilities-workflow.md).

## [2026-07-15] accepted | D-20260715-088 | Reliability применяется на общем dispatch, raw mutation остаётся partial

- Generated `safe_read` получает один bounded retry только из TDLib 429 delay; daemon method-class scheduler сохраняет весь server block, добавляет bounded jitter только внутри automatic budget и пишет retry/flood counters.
- Universal raw mutation проходит policy/approval, scheduler и durable journal до transport. Один TDLib response не считается domain terminal proof: result возвращается как `partial` с `reconciliation_required=true`, exact replay блокируется.
- Reconcile/never workflows с duplicate-sensitive side effects используют одну data list и canonical name/input fingerprint; `complete=false`, interrupted dispatch и unknown outcome остаются `uncertain`, convergent workflows продолжают доказывать desired state собственным reread.
- Shared fixed metrics доступны через protocol/CLI status; raw audit хранит generated method/risk и closed operational fields без payload/identifier. Contract: [`docs/feature-logic-harness/reliability-policy-observability.md`](../../docs/feature-logic-harness/reliability-policy-observability.md), [`docs/idempotency-journal.md`](../../docs/idempotency-journal.md), [`docs/telemetry-audit.md`](../../docs/telemetry-audit.md).

## [2026-07-15] accepted | D-20260715-089 | MCP остаётся малым transport-principal adapter

- MCP публикует ровно восемь tool families: `session`, `auth.begin/status/wait`, `schema`, `workflow`, `call`, `events`; tool-per-TDLib-method каталог не создаётся.
- Adapter только переводит validated JSON Schema arguments в shared `DaemonRequest`. Principal приходит из transport context; model argument не может подменить его. Auth tools принимают только challenge ID/timeout metadata, никогда secret submission.
- Curated workflow input не содержит TDJSON constructors: их строит существующий Rust core. `@type` остаётся только в universal raw `call`, где он является discriminator полного generated registry, а не заменой generated Rust validation.
- Contract: [`apps/telegram-mcp/src/main.rs`](../../apps/telegram-mcp/src/main.rs), [`docs/feature-logic-harness/mcp.md`](../../docs/feature-logic-harness/mcp.md).

## [2026-07-15] accepted | D-20260715-090 | Remote MCP использует OpenSSH forced stdio

- Единственный MCP runtime — official `rmcp 2.2.0` stdio с protocol `2025-11-25`. Remote доступ туннелирует тот же channel через OpenSSH; отдельный TCP/HTTP/TLS/OAuth server не создаётся.
- Restricted key запускает fixed `ssh-stdio <identity>`. `SSH_CONNECTION` обязателен; identity выбирает только root-owned exact-mode policy с profile/scopes и становится daemon principal. Model arguments не могут менять identity или transport ceiling.
- Startup/initialize/list не обращаются к daemon и не создают TDLib client. `tools/call` подключается только к existing private owner socket; reconnect создаёт новый MCP lifecycle и никогда не является основанием replay uncertain mutation.
- Contract: [`docs/mcp-transport.md`](../../docs/mcp-transport.md), [`apps/telegram-mcp/src/main.rs`](../../apps/telegram-mcp/src/main.rs).

## [2026-07-15] accepted | D-20260715-091 | MCP login передаёт metadata, operator submit привязан к exact challenge

- Shared protocol v3 возвращает закрытые `LoginState`, `challenge_id` и typed `next_action`; MCP auth tools не имеют secret submission route или credential fields.
- `telegram-cli login tty <challenge_id>` — one-shot owner channel: ID сверяется до prompt, secret читается только из protected `/dev/tty`, отправляется один typed input и управление возвращается MCP `auth.wait/status`.
- Remote operator использует отдельную authenticated SSH PTY session, не restricted MCP key и не model terminal. Stale ID fail closed; terminal login proof остаётся daemon-owned `Ready -> getMe -> expected identity`.
- Contract: [`docs/cli-secure-login.md`](../../docs/cli-secure-login.md), [`docs/mcp-transport.md`](../../docs/mcp-transport.md), [`docs/feature-logic-harness/mcp.md`](../../docs/feature-logic-harness/mcp.md).
