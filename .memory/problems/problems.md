# Problem Journal

Active append-only problem lifecycle. Status changes добавляются новой entry с тем же `P-*` ID.

## [2026-07-15] consolidation | P-20260715-012 | Журнал консолидирован

- По явному указанию пользователя журнал очищен от per-method записей и бухгалтерии ротаций. Полная история — в git. Ниже восстановлены только актуальные открытые проблемы.
- P-20260715-005 (116 методов без typed disposition) упразднена как проблема: по правилу plans.md неотревьюенный метод получает default-deny, ревью добирается пачками и ничего не блокирует. Списки методов — в `docs/capability-notes.md`.

## [2026-07-15] open | P-20260715-001 | Database key не подключён к штатному gateway

- Локальный database encryption key получен и хранится по `.env.local` contract, но штатный запуск пока не принимает его. Закрывается задачей P1 «Database encryption key из file descriptor/file secret/OS keychain».

## [2026-07-15] open | P-20260715-003 | Linux x86_64 native artifact не закреплён

- Закреплён только macOS arm64 `tdjson`. Linux x86_64 artifact с provenance — открытая задача P0; без него не начинается P9.

## [2026-07-15] resolved | P-20260715-003 | Linux x86_64 native artifact закреплён

- TDLib `1.8.66` собран exact pinned builder для `x86_64-unknown-linux-gnu`; artifact SHA-256 `e90ca3c25ad034b7227df918816c227de2b9aef92539c994a3bd41c42d68161b`, provenance — `vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json`.
- `python3 scripts/check-tdlib-native-pin.py --require-local-artifact` проверяет оба supported target, Linux ELF identity, SONAME, dependencies, TDJSON exports, runtime version/commit и отсутствие DB-файлов в no-client smoke.
- Bit-for-bit reproducibility остаётся незаявленной границей, но не является acceptance-критерием P0.

## [2026-07-15] narrowed | P-20260715-001 | Core provider готов, daemon wiring ещё отсутствует

- P1 protected provider и `setTdlibParameters` integration готовы: FD/file/keychain sources, empty-key preflight deny и wrong-key 401 latch проверены synthetic tests.
- Проблема остаётся открытой до P2: product binaries всё ещё fail-closed заглушки, поэтому штатный `telegramd` пока не выбирает profile secret reference и не доказывает returning `Ready`. Исходная фраза «закрывается задачей P1 provider» уточнена по live repository truth.

## [2026-07-15] narrowed | P-20260715-001 | Core returning path доказан, product daemon всё ещё не wired

- P1 `CoreRuntime` и protected loader свежо доказали returning `Ready`, `getMe` и normal Closed без нового login; wrong/missing-key native boundary также green.
- Проблема остаётся открытой уже только на product boundary P2: `telegramd` пока не выбирает profile key reference, не владеет runtime/DB и не предоставляет lifecycle через protocol.

## [2026-07-15] resolved | P-20260715-001 | Protected key подключён к singleton daemon

- `telegramd` выбирает Base64 file secret reference из protected process configuration после canonical DB lock, передаёт key только в redacted/zeroizing core provider и достигает returning `Ready/getMe` без login input.
- Missing/wrong key остаётся fail closed; normal idle path завершает `close -> authorizationStateClosed`. Protected live P2 gate не вывел key, API credentials, DB paths или Telegram identity.

## [2026-07-17] open | P-20260717-001 | `tg-analytics` prod TDLib session не открывается текущим key reference

- Evidence: canonical `scripts/tg-agent.sh me` завершился fail closed с `Telegram::Unauthorized` / `Wrong database encryption key`; sanitized digest: [`2026-07-17-tg-analytics-tdlib-auth-preflight.md`](../raw/2026-07-17-tg-analytics-tdlib-auth-preflight.md).
- Impact: source session не подтверждает `Ready/getMe` и не может предоставить phone input для P10 first-login flow. Returning session текущего проекта этим не опровергается.
- Status: open external-source blocker; повторный запуск, key repair и reauthorization запрещены до отдельной явной задачи владельца.
- Next check: только после operator-side восстановления exact key reference повторить один `scripts/tg-agent.sh me`; phone number не печатать и не сохранять.

## [2026-07-18] open | P-20260718-001 | Owner TTY немедленно завершался после phone prompt

- Evidence: Codex app terminal показал `Телефон:`, затем closed `SecureTtyFailed`; fresh broker status сохранил exact `phone_number/challenge_id=1`, следовательно input не дошёл до daemon. Sanitized digest: [`2026-07-18-p10-owner-tty-failure.md`](../raw/2026-07-18-p10-owner-tty-failure.md).
- Impact: P10 first-login chain остановлена до owner phone submission; database/session не повреждены.
- Status: local CLI reader исправлен на bounded nonblocking `/dev/tty` read/retry без stdin fallback; unit tests, clippy и regular build green, но live owner retry ещё не выполнен.
- Next check: повторить тот же текущий one-shot challenge и закрыть проблему только после fresh broker transition.

## [2026-07-18] resolved | P-20260718-001 | Owner TTY устойчиво ждёт скрытый ввод

- Evidence: после corrected build повторный one-shot prompt оставался активным до явного `Ctrl+C`; CLI восстановил terminal и вернул cancellation, а fresh broker status сохранил текущий challenge без submission. Sanitized digest: [`2026-07-18-p10-owner-tty-retry.md`](../raw/2026-07-18-p10-owner-tty-retry.md).
- Resolution: bounded nonblocking `/dev/tty` read/retry устранил immediate failure; prompts теперь явно сообщают, что символы не отображаются и ввод завершается Enter. Tests 6/6, clippy `-D warnings` и regular arm64 build green.
- Boundary: закрыта только TTY transport-проблема; phone/OTP/2FA chain и P10 terminal acceptance остаются pending.

## [2026-07-18] open | P-20260718-002 | Expired code challenge live resend ещё не доказан

- Evidence: после длительного ожидания Telegram отклонил введённый OTP как `LoginSubmissionRejected`; прежний daemon не вызвал resend и сохранил тот же sanitized code challenge. Source diagnosis показал неиспользуемые `authenticationCodeInfo.next_type/timeout`.
- Implemented: [D-20260718-004](../decisions/decisions.md) добавляет exact timeout-aware resend path; workspace tests, clippy и macOS build green. Старый daemon остановлен, новый binary запущен без `logOut`/`destroy`.
- Current state: TDLib после restart безопасно вернулась в `phone_number`; owner TTY снова ждёт номер. Секреты не читались и не сохранялись.
- Resolution gate: owner проходит phone/code chain; если code timeout уже прошёл, live status должен получить новый challenge после одного resend, затем login достигает `Ready + getMe`. До этого P10 и эта проблема остаются open.

## [2026-07-18] narrowed | P-20260718-002 | First login принят, остаётся только live resend branch

- Fresh evidence: owner успешно завершил first login; daemon доказал `Ready + getMe`, штатный `Closed` и returning `Ready` без нового secret input ([W-20260718-008](../logs/work.md)). P10 authorization slice больше не блокируется этой проблемой.
- Remaining scope: successful chain не доказала actual `resendAuthenticationCode` после elapsed timeout. Deterministic core/protocol/daemon tests и timeout/`next_type` gates green, но Telegram-side resend transition ещё не наблюдался.
- Resolution gate: при естественно истёкшем code challenge получить sanitized evidence одного `LoginCodeResent`/нового challenge без blind retry; до этого проблема остаётся narrow open follow-up.

## [2026-07-18] resolved | P-20260718-003 | Authorization review выявил blind replay и непроверенный re-auth Ready

- Evidence: timeout path удалял только transport correlation и затем вызывал `submission_failed`; broker IDs начинались с 1 при каждом boot; основной `serve_until_idle` выставлял Ready по update без повторного identity proof; registration не передавал ToS/privacy confirmation; QR и email resend не были достижимы из owner flow. Supplied line references подтверждены по live source и pinned `td_api.tl`.
- Impact: возможны duplicate auth submission, stale cross-restart handoff, операции после re-auth под непроверенной identity, неявное принятие ToS и неполные QR/email flows. Отдельно TDLib `429/500` ошибочно попадали в UX «неверный OTP» и могли запускать resend.
- Resolution: [D-20260718-005](../decisions/decisions.md) вводит uncertain outcome/reconciliation, boot-scoped tokens, `Ready -> Starting -> identity-verified Ready` + lease revocation, owner-only prompt contract, explicit ToS/privacy, QR initiation, email resend, Apple/Google token parity, partial machine outcomes и redacted reducer Debug. `400` отделён от transient `429/500`; re-auth больше не наследует старый idle timer.
- Verification: `cargo test --workspace --all-targets -q` — 148 passed, 0 failed, 3 ignored; `cargo clippy --workspace --all-targets -- -D warnings` green; planning/workspace/skeleton/secret gates green. Generated-registry gate остаётся красным из-за существующего вне этой задачи dirty generator/registry WIP и не использован как completion evidence.
- Boundary: interrupt OTP prompt при истечении resend delay отклонён как неверная семантика — TDLib timeout не является OTP TTL. Полный AuthorizationCoordinator/LoginDriver refactor и один end-to-end fake-TDLib test не требуются для исправления подтверждённых дефектов; добавлены детерминированные boundary regressions. Live Telegram expired-code resend остаётся [P-20260718-002](../problems/problems.md).

## [2026-07-18] correction | P-20260715-003 | Reproducibility boundary закрыта в P9

- Исходный P0 resolution корректно закреплял Linux artifact после одной сборки и не заявлял bit-for-bit reproducibility.
- Первый Tasks-пункт P9 теперь выполнил отдельную independent exact-recipe сборку обоих targets; digest совпал с committed reference, а rebuild path fail closed при mismatch ([W-20260718-011](../logs/work.md)).
- Это correction к прежней boundary note, а не изменение P0 acceptance задним числом.
