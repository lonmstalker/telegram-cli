# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

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

## [2026-07-18] accepted | D-20260718-001 | Phone виден только в owner TTY

- Context: live P10 handoff показал, что полностью скрытый phone input выглядит как неработающий terminal; владелец явно указал, что номер не является password и разрешил echo.
- Decision: `phone_number` и `premium_purchase` читаются только из protected owner `/dev/tty`, но с включённым terminal echo. OTP, 2FA password, email и registration data остаются с выключенным echo.
- Evidence: явная инструкция владельца 2026-07-18; [`apps/telegram-cli/src/main.rs`](../../apps/telegram-cli/src/main.rs); обновлённые RU/EN authorization guides; CLI tests 6/6, protected protocol redaction test, clippy `-D warnings` и regular arm64 build green.
- Consequences: phone остаётся в owner terminal scrollback и может быть виден terminal host, но не принимается через args/stdin/env, не включается в machine output, daemon logs или project memory и продолжает храниться в `ProtectedString`/zeroizing buffers.

## [2026-07-18] accepted | D-20260718-002 | Human login проходит всю challenge chain одной командой

- Context: one-shot `login tty <challenge_id>` был выставлен пользователю как основной flow и заставлял вручную чередовать phone/code/password с status checks, хотя CLI уже владеет безопасным loop.
- Decision: default human `telegram-cli login` сам читает fresh challenge, запрашивает phone → OTP → 2FA/cloud password → email/registration по фактической TDLib chain и возвращается только на terminal state/error/cancel. Промежуточный `LoginSubmitted` пользователю не печатается.
- Machine boundary: `telegram-cli --output json|jsonl login` остаётся prompt-free status API. `login tty <challenge_id>` сохраняется только как backwards-compatible MCP/operator one-shot handoff; `login tty` — legacy alias полного human loop.
- Evidence: явная инструкция владельца; routing test `plain_human_login_is_interactive_but_machine_login_is_status_only`; CLI 7/7, protocol login-status redaction test, clippy `-D warnings`, regular arm64 build и RU/EN docs parity green.
- Consequences: основной UX не раскрывает внутренние challenge IDs и не требует повторного запуска команды; stale-ID fail-closed и MCP separation сохраняются.

## [2026-07-18] accepted | D-20260718-003 | Auth input виден, cloud password echo выбирает владелец

- Context: владелец явно отклонил hidden echo для phone/OTP/email/registration и потребовал самому выбирать видимость только для cloud password.
- Decision: phone, Telegram code, email address/code и registration names видны в owner `/dev/tty`. Перед cloud password CLI спрашивает `Скрыть cloud password? [y/N]:`; default Enter/`n` оставляет password видимым, `y` включает `EchoGuard` только для этого ввода.
- Boundary: authorization values всё ещё запрещены в args/stdin/env/machine output/logs и передаются как redacted `ProtectedString` с zeroizing buffers. Visible mode означает owner terminal echo/scrollback, а не отдельный CLI output.
- Evidence: явная инструкция владельца; visibility-choice test, CLI 8/8, protected protocol redaction test, clippy `-D warnings`, regular arm64 build и RU/EN docs parity green.
- Supersedes: echo policy [D-20260718-001](../decisions/decisions.md); его запрет model-visible transport и log output сохраняется.

## [2026-07-18] accepted | D-20260718-004 | Code resend следует server timeout, а не guessed OTP TTL

- Context: после долгого ожидания TDLib оставалась в `authorizationStateWaitCode`, CLI просила прежний OTP, а `LoginSubmissionRejected` завершал весь human flow без попытки `resendAuthenticationCode`.
- Decision: `authenticationCodeInfo.timeout` трактуется только как задержка до разрешённого resend, не как гарантированный срок жизни OTP. Daemon запоминает момент наблюдения exact code challenge и разрешает typed `resendAuthenticationCode` только после timeout и при ненулевом `next_type`.
- Human UX: обычный `telegram-cli login` делает одну proactive resend-попытку до OTP prompt; после отклонённого кода снова проверяет resend eligibility. До timeout пользователь может повторить ввод, после успешного resend CLI ждёт новый challenge. Exact-ID one-shot остаётся one-shot и сам resend не запускает.
- Safety: resend привязан к текущему `challenge_id`, concurrent submission блокируется, Telegram rejection не превращается в retry loop. Phone/OTP и TDLib error text не добавляются в protocol, logs или memory.
- Evidence: явная инструкция владельца; [`crates/telegram-core/src/authorization.rs`](../../crates/telegram-core/src/authorization.rs), [`apps/telegramd/src/server.rs`](../../apps/telegramd/src/server.rs), [`apps/telegram-cli/src/main.rs`](../../apps/telegram-cli/src/main.rs); workspace tests, clippy `-D warnings`, regular macOS build и docs checks green.

## [2026-07-18] accepted | D-20260718-005 | Ready и login submission разделяют observed и verified outcomes

- Context: supplied review показал три связанные trust-gap: response timeout очищал pending submission, numeric challenge generation повторялась после daemon restart, а re-auth `Ready` в основном serve loop не повторял `getMe`/identity proof.
- Decision: wire challenge становится opaque boot-scoped token; core submission фиксирует `NotSent`, `DefinitiveRejected` или `Uncertain`; timeout остаётся pending до fresh auth update либо late-response reconciliation. Любой auth update снимает verified readiness, переводит lifecycle `Ready -> Starting` и отзывает leases; только новый успешный `getMe`/expected-identity proof возвращает lifecycle в Ready и снова открывает operations.
- Owner boundary: protocol v4 добавляет отдельный private `LoginPrompt` для QR link, password hint и registration ToS/privacy. Machine status/MCP сохраняют только state/token/action. Registration требует explicit `terms_accepted`; default privacy choice даёт `disable_notification=true`.
- Evidence: [`crates/telegram-core/src/authorization.rs`](../../crates/telegram-core/src/authorization.rs), [`crates/telegram-core/src/transport.rs`](../../crates/telegram-core/src/transport.rs), [`apps/telegramd/src/lifecycle.rs`](../../apps/telegramd/src/lifecycle.rs), [`apps/telegramd/src/server.rs`](../../apps/telegramd/src/server.rs), [`crates/telegram-protocol/src/lib.rs`](../../crates/telegram-protocol/src/lib.rs); `cargo test --workspace --all-targets -q` — 148 passed, 3 ignored; workspace clippy `-D warnings` green.
- Consequences: numeric protocol v3 challenge handoffs intentionally несовместимы с v4; старый token после restart/profile/state change fail closed. Platform-specific passkey/web-token/Firebase/Premium/bot-token/password-recovery journeys не открываются generic raw pre-Ready route и остаются отдельным owner-broker scope.

## [2026-07-18] accepted | D-20260718-006 | Authorization имеет одного daemon owner и тестируемый CLI driver

- Context: после исправления auth defects production daemon всё ещё создавал отдельную `AuthorizationMachine` в startup и вторую внутри server broker, а human loop смешивал protocol transitions, Unix socket, TTY и sleep в `main.rs`.
- Decision: `AuthorizationCoordinator` создаётся до `reach_ready`, единолично владеет core machine, boot token, resend timing, pending/uncertain outcome и verified account, затем тем же instance передаётся в `LeaseServer`. Наружу выходит только закрытый `AuthorizationObservation`; lifecycle поставляет `getMe` proof, но не хранит readiness. CLI `LoginDriver<Broker, Prompter, Runtime>` владеет protocol loop, а socket/TTY/cancellation/polling реализованы adapters.
- Evidence: [`apps/telegramd/src/authorization.rs`](../../apps/telegramd/src/authorization.rs), [`apps/telegramd/src/lifecycle.rs`](../../apps/telegramd/src/lifecycle.rs), [`apps/telegramd/src/server.rs`](../../apps/telegramd/src/server.rs), [`apps/telegram-cli/src/login.rs`](../../apps/telegram-cli/src/login.rs), [architecture note](../../docs/brainstorms/2026-07-18-authorization-architecture.md); red compile checks предшествовали coordinator/driver implementation, затем `cargo test --workspace --all-targets -q` — 154 passed, 0 failed, 3 ignored и workspace clippy `-D warnings` green.
- Consequences: production `telegramd` содержит одну `AuthorizationMachine`; `LeaseServer` больше не имеет параллельных `ready/account_kind`; owner metadata не покидает coordinator через lifecycle API; multi-step/one-shot/cancellation/malformed CLI paths проверяются без TDLib, socket, TTY или sleep. Protocol v4, secret visibility и capability scope не изменены.
- Extends: safety semantics [D-20260718-005](../decisions/decisions.md); не supersede-ит live resend boundary [D-20260718-004](../decisions/decisions.md).

## [2026-07-18] accepted | D-20260718-007 | Native reproducibility сверяется с committed reference artifact

- Context: exact source и один успешный build доказывали provenance, но не bit-for-bit reproducibility; P9 требует повторяемый output для обоих supported targets.
- Decision: independent rebuild получает expected SHA-256 только из committed provenance с exact source/target equality. После полного inspection новый artifact публикуется лишь при совпадении digest; mismatch fail closed и не переписывает reference. Provenance `verified/2` допустим только после второй independent exact-recipe build.
- Target details: macOS recipe использует stable file-prefix maps для source/build roots; Linux собирается pinned container recipe. Artifact остаётся в ignored content-addressed cache, Git хранит policy/recipe/provenance.
- Evidence: [`build-tdlib-native.py`](../../scripts/build-tdlib-native.py), [`build-tdlib-linux-native.py`](../../scripts/build-tdlib-linux-native.py), target provenance и [sanitized checkpoint](../raw/2026-07-18-p9-reproducible-native-builds.md). Fresh provenance gate и все process/crash/input guards green; local Docker artifact gate в текущем checkpoint недоступен и не включён в claim.
- Consequences: first build не может self-certify reproducibility; последующий intentional recipe/source change требует нового reference cycle и explicit provenance update. Следующий P9 Tasks-пункт не затрагивается.
