# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

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

## [2026-07-18] accepted | D-20260718-008 | Auth decomposition следует ответственности, а не числу строк

- Context: после ownership refactor поведение было закрыто тестами, но auth source всё ещё содержал крупные orchestration methods и сотни строк inline test fixtures.
- Decision: mixed status/transition/I/O/error orchestration разделяется на named helpers с одним outcome; исчерпывающие state/prompt mappings могут превышать 40 физических строк и остаются цельными, пока выполняют одну табличную ответственность. Большие unit-test модули подключаются через sibling `tests.rs`; маленькие локальные tests могут оставаться inline.
- Evidence: [`apps/telegram-cli/src/login.rs`](../../apps/telegram-cli/src/login.rs), [`apps/telegramd/src/authorization.rs`](../../apps/telegramd/src/authorization.rs), [`apps/telegramd/src/lifecycle.rs`](../../apps/telegramd/src/lifecycle.rs), [`crates/telegram-core/src/authorization.rs`](../../crates/telegram-core/src/authorization.rs), [architecture note](../../docs/brainstorms/2026-07-18-authorization-architecture.md).
- Consequences: auth behavior и protocol/secret/capability boundaries не меняются; future review должен искать смешанные причины изменения, а не дробить exhaustive mappings ради формального line-count.

## [2026-07-18] accepted | D-20260718-009 | Daemon socket transport принадлежит общему client crate

- Context: CLI, MCP и Web App runner независимо повторяли profile path, effective uid, security-sensitive metadata validation и JSON socket exchange; при этом timeout и response framing у consumers различались.
- Decision: `telegram-client` единолично владеет client-side `socket_path`, `valid_name`, `effective_uid`, `validate_socket` и exchange I/O. `telegram-protocol` остаётся I/O-free wire crate, а приложения сохраняют mapping `ClientErrorCode` в локальные ошибки.
- Compatibility: metadata predicates перенесены буквально: directory current uid/exact `0700`, socket current uid/Unix type/`nlink == 1`/exact `0600`. Default `exchange` сохраняет CLI line/35s contract; explicit `ExchangeOptions` сохраняют MCP EOF/35s и runner bounded-newline-16KiB/5s contracts, включая runner `SocketUnavailable` на connect race.
- Evidence: [`crates/telegram-client/src/lib.rs`](../../crates/telegram-client/src/lib.rs), manifests трёх consumers, [`check-workspace-boundaries.py`](../../scripts/check-workspace-boundaries.py), [socket contract](../../docs/daemon-profile-socket.md); workspace test/clippy и boundary negative controls green.
- Consequences: новые daemon clients переиспользуют один trust boundary и обязаны выбирать framing/deadline осознанно; consumer-specific presentation и domain response matching в library не переносятся.

## [2026-07-19] accepted | D-20260719-001 | Public resolve, invite preview и membership являются разными операциями

- Context: прежний `ChatTarget::InviteLink` смешивал вид ссылки, публичность канала и membership/access. `inspect_chat` дополнительно зависел от `updateNewChat` после уже полученного `chat` response и сериализовал raw cached/full-info objects.
- Decision: `resolve_chat` принимает только ID, public username или public link; `preview_invite_link` отдельно и terminal классифицирует TDLib `is_public` и текущий access; только `ensure_membership` может вызвать join. Invite-shaped URL fail closed в public resolve/inspection, а обычный public URL fail closed в invite preview.
- Projection: read workflows возвращают allowlisted `ChatIdentity`/`InvitePreview`; raw chat, full info, message/draft payload, description, member IDs и invite token не покидают core. Public identity обогащается authoritative reducer snapshot, но прямой `chat` response достаточен для hydration и full-info selection.
- Evidence: [`chat.rs`](../../crates/telegram-core/src/workflows/chat.rs), [`chat_inputs.rs`](../../apps/telegramd/src/chat_inputs.rs), [workflow docs](../../docs/chat-resolution-membership.md), [sanitized live checkpoint](../raw/2026-07-19-p10-chat-read-projection.md).
- Consequences: наличие invite link больше не означает private channel; preview без доступа является завершённым preview, а не partial workflow или доказанным membership requirement. Получение/публикация private invite требует отдельного explicit product surface и не возвращается inspect по умолчанию.
