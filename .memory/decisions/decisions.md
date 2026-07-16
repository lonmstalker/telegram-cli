# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-070 | Machine output имеет один versioned envelope

- `telegram-protocol` владеет envelope v1 с закрытым root status `ok/partial/error`; human renderer остаётся неверсированным presentation layer. One-shot compact JSON и JSONL сериализуют один и тот же envelope.
- Daemon вычисляет workflow `complete` из typed core result до сериализации. `complete=false` и event gap дают root `partial`, поэтому client не распознаёт domain prose или произвольные payload fields.
- Ошибки разделены на closed `command/lease/client` domains без arbitrary text. Exit codes стабильны по классам input, availability, daemon rejection и protocol/output failure; partial остаётся exit 0, но явно виден machine status.

## [2026-07-15] accepted | D-20260715-071 | Event stream владеет heartbeat и cleanup переданного lease

- Human/JSONL `events watch` повторяет существующий one-shot cursor request, публикует только первый baseline и непустые/gap batches, а heartbeat schedule берёт треть TTL из authoritative daemon response. Отдельный streaming transport не создаётся.
- SIGINT/SIGTERM handler выполняет только atomic store. Client loop освобождает lease обычным protocol request до `cancelled`; broken pipe использует тот же cleanup без повторной записи в закрытый output. TTL остаётся fallback для SIGKILL/crash.
- Explicit JSON сохраняет one-shot snapshot и не присваивает ownership caller-held lease. Cancellation относится к watch loop; уже dispatch-нутая mutation не объявляется отменённой.

## [2026-07-15] accepted | D-20260715-072 | Login secret проходит только owner-operated TTY challenge

- `login` остаётся безопасным status route, а `login tty` — единственным CLI route для phone/OTP/2FA/email/registration input; secret не принимается flags, stdin, env или output.
- CLI открывает `/dev/tty` как no-follow nonblocking character device, отключает echo RAII guard-ом и после SIGINT/SIGTERM возвращает termios обычным control flow. Protected strings и read/request buffers zeroize, debug surfaces redacted.
- Daemon broker сохраняет существующую `AuthorizationMachine`, принимает только exact current challenge ID и typed input, блокирует pending/stale submission и не добавляет второй auth state machine. TDJSON `@type` формируется внутри core authorization request.
- `LoginSubmitted` не является completion proof: только subsequent `Ready -> getMe -> expected identity` открывает raw/workflow dispatch. Live first-login остаётся P10 gate.

## [2026-07-15] accepted | D-20260715-073 | Agent skill загружает API только через runtime discovery

- Repo-local `telegram-cli` skill содержит только один lifecycle: status, minimal lease, workflow-first discovery, schema/raw fallback, structured continuation и release. TDLib method/workflow input catalog в skill отсутствует.
- Daemon добавляет `workflow describe <name>` с exact JSON input example из bounded workflow inventory. Agent загружает descriptor только выбранного route; raw fallback использует generated schema search/describe и caller `@type` только как TDJSON discriminator.
- Machine decisions читают только envelope v1. `partial/complete=false/pending/gap/next_action` требуют continuation, resync или operator action; prose, blind mutation retry, self-approval и owner secret ввод запрещены.
- Offline cold-context traces покрывают history, statistics, sticker prerequisite, bot terminal send и Mini App browser handoff. `tiktoken 0.12.0` измерил skill как 774 cl100k/633 o200k tokens при limit 1500; live variance остаётся P10 evidence.

## [2026-07-15] accepted | D-20260715-074 | User profile output закрывает private fields до protocol

- `user_profile` разрешает self, numeric ID и public username через existing generated calls; неизвестный username сначала проходит `searchPublicChat`/ordered `updateUser`, а non-user chat не становится user profile.
- Output состоит из selected public/state fields. Phone, birthdate, private note и business info заменяются closed `unavailable|redacted`; значение не достигает daemon protocol, telemetry или memory.
- `update_profile_name` проверяет current `getMe`, не dispatch-ит достигнутое состояние и после `setName` ждёт exact newer `updateUser`. Deadline после dispatch возвращает `uncertain/complete=false`, не automatic repeat.
- Capability data добавляет только фактических consumers `getMe`, `getUser`, `setName`; остальные contact/profile methods остаются full raw/default-deny surface без guessed contract.

## [2026-07-15] accepted | D-20260715-075 | Topic completion определяется cursor и server-state probe

- Main/archive/folder chat lists используют один `loadChats` engine и reducer positions; отдельная folder state machine не создаётся. Resolve остаётся read-only, optional open lifetime парно закрывается существующим lease workflow.
- `getForumTopics` следует всей returned cursor triple. Short page не доказывает terminal; zero cursor означает exhausted, repeated cursor — `no_progress/complete=false`.
- Close/reopen topic — desired-state mutation: совпавшее состояние не dispatch-ится, а accepted или timed-out request считается verified только после matching `getForumTopic`. Иначе receipt остаётся uncertain без blind retry.
- Curated inputs скрывают TDJSON discriminator; `@type` остаётся только внутри core и в universal raw call. Остальные folder/chat/topic методы остаются generated raw/default-deny, без per-method family layer.

## [2026-07-15] accepted | D-20260715-076 | Message read-state explicit, send terminal update-driven

- History/search требуют cached chat и не вызывают presence methods по умолчанию. `mark_read=true` dispatch-ит `viewMessages` только после complete page; partial chain не создаёт скрытый read-state side effect.
- Для chat с `has_protected_content` поле message content заменяется closed redaction marker до daemon protocol. Constructor сохраняется для machine routing, payload не сериализуется.
- Text send строит nested generated-validated TDJSON внутри core. Temporary message ID связывает ordered send terminal update; response/terminal timeout возвращает uncertain и никогда не запускает blind resend.
- Edit/delete/forward/reaction/poll и остальные message families остаются full raw/default-deny surface до отдельного capability review, без per-method workflow modules.

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
