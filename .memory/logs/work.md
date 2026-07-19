# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-18] completed | W-20260718-013 | Daemon socket client устранён из трёх приложений

- Goal: закрыть один P6 Tasks-пункт — вынести повторяющийся client-side Unix socket trust boundary из CLI, MCP и Web App runner без изменения поведения.
- Sources: [`plans.md`](../../plans.md), [`HARNESS.md`](../../HARNESS.md), [socket contract](../../docs/daemon-profile-socket.md), три live client implementations и явная инструкция пользователя. Durable boundary: [D-20260718-009](../decisions/decisions.md).
- Actions: добавлен product lib `telegram-client`; path/name/euid/metadata validation и JSON exchange удалены из приложений. Timeout, EOF/line/bounded-newline framing и connect-error различия передаются options; CLI/runner error mapping остаётся app-local. Socket integration test перенесён, trust-boundary metadata/framing tests добавлены.
- Verification: `cargo test --workspace` — 157 passed, 0 failed, 3 ignored; `cargo clippy --workspace --all-targets -- -D warnings` и `python3 scripts/check-workspace-boundaries.py` (4 negative controls) green. Финальный exact build/test/boundary gate выполняется после journal rotation.
- Next: implementation-enabler закрыт; P9 packaging и оставшиеся P10 live scenarios не затронуты.

## [2026-07-18] correction | W-20260718-013 | Финальный test count уточнён

- После удаления одного избыточного profile-name теста ради правила пропорциональности финальный `cargo test --workspace` содержит 156 passed, 0 failed и 3 ignored native tests; исходный checkpoint 157/0/3 был выполнен до этого удаления.
- Финальный `cargo build --workspace` и `python3 scripts/check-workspace-boundaries.py` (4 negative controls) green; socket trust-boundary и framing tests сохранены.

## [2026-07-19] completed | W-20260719-001 | Общие Cargo dependencies централизованы

- Goal: убрать повторяющиеся версии и workspace-relative paths из member manifests без изменения dependency graph, features или разрешённых версий.
- Sources: явная инструкция пользователя; root и member `Cargo.toml`; baseline `cargo metadata --no-deps --format-version 1` и `cargo tree --workspace`.
- Actions: root `[workspace.dependencies]` стал владельцем пяти повторяющихся external crates и трёх повторяющихся internal path dependencies; восемь members используют `dep.workspace = true`. Однократные `base64`, `ed25519-dalek`, `rmcp` и `tokio` сохранены локально.
- Verification: `cargo tree --workspace` до/после совпадает exact (167 строк); Git object hash `Cargo.lock` остался `f06105dd77018f77fdc9f661cba23b7423fe33d6`; `cargo build --workspace` green; `cargo test --workspace` — 156 passed, 0 failed, 3 ignored; workspace boundary gate — 4 negative controls green.
- Next: phase status и runtime contracts не менялись; отдельного follow-up нет.

## [2026-07-19] completed | W-20260719-002 | Structural regressions закрыты автоматическими guards

- Goal: не допустить возврата app-local daemon socket clients, неконтролируемого роста Rust source-файлов и явных member versions для зависимостей из `[workspace.dependencies]`.
- Actions: добавлены три stdlib-only Python guards и paired temporary-fixture tests; source limit равен 1500 строк с exact ratchet для `telegramd/server.rs` и `workflows/mod.rs`, `telegramd` явно отделён от consumer scan, обычные/dev/build/target Cargo dependency tables проверяются на `workspace = true`. Общий `check-workspace-boundaries.py` стал canonical harness entrypoint; контракт описан в [`workspace-structural-guards.md`](../../docs/workspace-structural-guards.md).
- Verification: все три paired tests и три прямых checks green; общий workspace boundary gate green с 4 negative controls и 3 structural guards; Python 3.9 byte-compilation и `git diff --check` green.
- Boundary: Rust/runtime/Cargo graph не менялись; CI workflow и активных git-хуков в репозитории нет, поэтому guards подключены к существующей harness-процедуре.

## [2026-07-19] completed | W-20260719-003 | Создан live regression ledger и закрыт первый chat read slice

- Goal: завести воспроизводимый regression source of truth и доказать на существующей авторизованной TDLib-сессии первый P10 chat slice без mutation/presence/message payload.
- Sources: [`plans.md`](../../plans.md), [`live-regression.md`](../../docs/live-regression.md), [`chat-list-loading.md`](../../docs/chat-list-loading.md), [sanitized live evidence](../raw/2026-07-19-p10-chat-read-regression.md), явная инструкция пользователя.
- Actions: `load_chat_list` расширен compact `entries` с closed chat kinds при сохранении прежних `positions`; full cached chat/last message/file objects в inventory не сериализуются. Regression ledger закрепил stable scenario IDs, точные команды, terminal proof и pending boundaries.
- Verification: current-worktree binaries прошли returning `Ready`; main list — 11 entries (4 channel, 1 supergroup, 6 private/service), archive — 0, оба `complete=true/all_chats_loaded`; forbidden payload keys отсутствуют; lease released, daemon `Draining -> Closed`. `cargo test --workspace --all-targets -q` — 156 passed, 0 failed, 3 ignored; workspace clippy `-D warnings` green.
- Boundary: names/IDs/raw responses не сохранены; link/invite resolve, folder/forum, presence/open, gap/resync и F009+ scenarios остаются pending. Общая P10 не accepted.

## [2026-07-19] correction | W-20260719-003 | Wiki check отделён от chat regression evidence

- Fresh project gates green: planning, workspace structural guards, skeleton, generated registry, secret-output canary, fmt и diff check; standalone rotation test suite также green.
- `rotate-wiki-journal.py --all --check` остаётся exit 1 только из-за tracked historical decision link, существующего в `HEAD`; это зафиксировано как [P-20260719-001](../problems/problems.md). Immutable shard/checksum не менялись.
- Chat implementation/live verification остаются доказанными; общий wiki link-integrity claim не заявляется до отдельного resolution P-20260719-001.

## [2026-07-19] completed | W-20260719-004 | CHAT-004 public-link resolve проверен live

- Goal: найти среди реально доступных channel публичные fixtures и закрыть CHAT-004 без join,
  open, send или изменения read-state.
- Sources: [`live-regression.md`](../../docs/live-regression.md),
  [sanitized live evidence](../raw/2026-07-19-p10-chat-public-resolve.md), runtime workflow/schema
  discovery и явная инструкция пользователя.
- Actions: четыре channel из compact CHAT-001 inventory проверены через `inspect_chat(open=false)`;
  найденные публичные кандидаты затем разрешены только workflow `resolve_chat`. Три URL совпали
  с точными исходными chat IDs; три похожих кандидата отклонены по mismatch/error. У одного
  channel публичная ссылка не подтверждена.
- Verification: returning authorization снова достигла `Ready`; три успешных вызова дали
  `status=ok`, `complete=true` и exact chat-ID match. Использовался только `read` lease;
  `ensure_membership`, open, send и raw default-deny bypass не выполнялись. URL, usernames, IDs,
  descriptions и raw responses в Git не сохранены.
- Boundary: CHAT-004 принят; invite preview, folder/forum fixtures, presence/open и остальные P10
  scenarios остаются pending. Общая P10 не accepted.

## [2026-07-19] correction | W-20260719-004 | Финальные gates и cleanup подтверждены

- Fresh verification: `cargo test --workspace --all-targets -q` — 156 passed, 0 failed,
  3 ignored; workspace clippy `-D warnings`, planning boundary, structural workspace gate,
  active work/problems journal checks и `git diff --check` green.
- Read lease к моменту explicit release уже истёк (`lease_not_found`); daemon после zero activity
  штатно завершился `Draining -> Closed`. Поиск exact public fixtures в regression/wiki/log/raw
  artifacts пуст, поэтому Telegram URLs/usernames не попали в Git.
- Общий `rotate-wiki-journal.py --all --check` по-прежнему красный только на известной immutable
  historical link problem [P-20260719-001](../problems/problems.md); этот boundary не включён в
  acceptance CHAT-004.

## [2026-07-19] completed | W-20260719-005 | Chat read projection разделяет public resolve и invite preview

- Goal: устранить смешение public/private/access semantics, убрать raw TDLib objects из chat read results и исключить false timeout при прямом `chat` response без cache update.
- Decision: [D-20260719-001](../decisions/decisions.md). `resolve_chat` теперь принимает только ID/public username/public link; отдельный terminal `preview_invite_link` проецирует `is_public` и access без join; `inspect_chat` возвращает compact identity/full-info kind и использует response как hydration source.
- Safety tests: raw chat/full-info/invite canaries отсутствуют в serialized results; public/non-public preview классифицируется по TDLib data; resolve/preview не вызывают join/open/close. Daemon reject-ит invite target в resolve/inspect и публикует отдельный strict preview input.
- Refactor: chat input adapters вынесены в `apps/telegramd/src/chat_inputs.rs`, chat negative test — рядом с workflow; source ratchets снижены до 2691/2201 строк без повышения лимита.
- Live: fresh binaries достигли returning `Ready`; CHAT-001/003/004 повторно дали terminal list, compact inspect `open=false`, public visibility и exact same-ID resolve. Forbidden raw fields отсутствовали, read lease released, daemon `Draining -> Closed`; sanitized evidence: [`2026-07-19-p10-chat-read-projection.md`](../raw/2026-07-19-p10-chat-read-projection.md).
- Verification: `cargo test --workspace --all-targets -q` — 156 passed, 0 failed, 3 ignored; workspace clippy `-D warnings`, fmt, planning, structural workspace, skeleton, registry, secret-output и diff checks green.
- Boundary: CHAT-005 live остаётся pending без disposable invite fixture; private invite не извлекался из full info ради теста. Общая P10 не accepted.

## [2026-07-19] completed | W-20260719-006 | CHAT-005 invite preview проверен live

- Goal: закрыть pending CHAT-005 на явно переданном owner disposable fixture без membership, presence или утечки invite token/raw response.
- Sources: [`live-regression.md`](../../docs/live-regression.md), [D-20260719-001](../decisions/decisions.md), owner-supplied ephemeral fixture и [sanitized live evidence](../raw/2026-07-19-p10-chat-invite-preview.md).
- Execution: fresh committed daemon достиг returning `Ready`; runtime discovery подтвердил отдельный `preview_invite_link`; один успешный вызов под `read` lease дал root/result `complete=true`, kind `channel`, TDLib visibility `non_public`, current access `accessible`, chat ID present, zero temporary access and no join request.
- Projection/safety: наружу проверены только allowlisted keys; description/member IDs/invite link отсутствовали. `ensure_membership`, join, open и send не запускались; deterministic dispatch regression сохраняет exact `checkChatInviteLink`-only path.
- Cleanup: lease released explicitly, daemon `Draining -> Closed`. URL/token, title и chat ID не записаны в Git, terminal evidence или memory.
- Boundary: CHAT-005 accepted; CHAT-006–009 и history/search/members/statistics остаются pending. Общая P10 не accepted.

## [2026-07-19] completed | W-20260719-007 | Terminal leave реализован, rejoin остановлен на approval

- Goal: реализовать typed terminal leave и выполнить запрошенный owner live round-trip
  leave → join по ранее переданной invite link без raw bypass и blind retry.
- Implementation: `leaveChat` reviewed как `reversible_mutation/reconcile`; core ждёт более новый
  reducer membership status, возвращает idempotent already-left без dispatch и сохраняет timeout
  uncertain. Daemon публикует strict `leave_chat` input/workflow; capability registry и docs
  regenerated. Durable semantics: [D-20260719-002](../decisions/decisions.md).
- Verification: deterministic terminal/idempotent/timeout tests green; workspace — 158 passed,
  0 failed, 3 ignored; clippy `-D warnings`, fmt, planning/workspace, secret-output и diff gates
  green. Returning live auth достигла `ready`, все три typed contracts обнаружены.
- Live: leave terminal `verified_left`; повторный `ensure_membership` вернул partial
  `request_pending`. Lease released, daemon `Draining -> Closed`; invite/title/chat ID/raw response
  не сохранены. Evidence:
  [`2026-07-19-p10-chat-membership-roundtrip.md`](../raw/2026-07-19-p10-chat-membership-roundtrip.md).
- Boundary: typed implementation и доказанный leave закрыты, но CHAT-010/P10 task не accepted до
  admin approval и terminal member proof; внешний blocker —
  [P-20260719-002](../problems/problems.md). Join не повторялся и chat-ID bypass не выполнялся.

## [2026-07-19] completed | W-20260719-008 | Async membership receipt/status закрыли CHAT-010

- Goal: не блокировать join до admin approval, вернуть немедленный typed status и принять поздний
  Telegram member update без повторной mutation.
- Implementation: `ensure_membership` разделяет submission/membership completeness и больше не
  сериализует raw result; read-only `membership_status` поддерживает chat ID/invite, fresh group
  probe и response-boundary update application. Journal policy вынесена из server; status не
  journaled, structural ratchets снижены до 2679/2146. Decision correction:
  [D-20260719-002](../decisions/decisions.md).
- Deterministic verification: pending → late `updateSupergroup/member` → status member, exact join
  count 1; closed mappings для current TDLib member statuses и честный unresolved. Workspace —
  163 passed, 0 failed, 3 ignored; clippy/fmt/planning/workspace/skeleton/registry/secret/diff gates
  green.
- Live: returning auth `ready`; discovery/describe подтвердили `membership_status`; один read-only
  status по owner fixture дал complete `member` server snapshot. Lease released, daemon
  `Draining -> Closed`; invite/title/chat ID/raw response не сохранены. Evidence:
  [`2026-07-19-p10-chat-async-membership-status.md`](../raw/2026-07-19-p10-chat-async-membership-status.md).
- Boundary: CHAT-010 accepted, [P-20260719-002](../problems/problems.md) resolved; общая P10 всё ещё
  pending по остальным domain/live-failure scenarios.

## [2026-07-19] completed | W-20260719-009 | A1 resolve применяет response boundary

- Goal: не маркировать reducer-derived supergroup fields как fresh server snapshot, пока updates,
  доставленные до `getChat/searchPublicChat` response, не применены.
- Sources: [`plans.md`](../../plans.md), [`chat-resolution-membership.md`](../../docs/chat-resolution-membership.md), runtime response-boundary contract и явное ТЗ пользователя.
- Actions: `resolve` теперь требует resync, получает correlation boundary, применяет ordered updates
  до него и только затем проецирует `ChatIdentity`; daemon caller обновлён через mutable runtime.
  Deterministic backend доставляет `updateSupergroup` до response и доказывает, что reducer usernames
  вошли в resolved identity.
- Verification: targeted regression green; `cargo test --workspace --jobs 2 -q` — 164 passed,
  0 failed, 3 ignored. Все `scripts/check-*.py` green под bundled Python 3.12.13; системный
  Python 3.9.6 не поддерживает pinned native guard `dataclass(slots=True)`, поэтому не использован
  для этой проверки. `cargo fmt`, `git diff --check` green.
- Next: A2 — связать journal classification с единственным workflow catalog.
