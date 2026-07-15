# TDLib ChatKind capability semantics

Дата: 2026-07-15.

## Scope

Этот checkpoint добавляет первый closed predicate для условных chat-level runtime requirements. Он не является общей поддержкой всех capability-like signals и не создаёт runtime truth: generated artifact по-прежнему описывает только требуемое evidence.

## Exact model and schema evidence

- `ResolvedChatKind` закрыт пятью значениями: `private`, `basic_group`, `supergroup`, `channel`, `secret`.
- `ChatKindCondition` связывает kind с typed `ChatTargetRef`. `supergroup_id` допускает только `supergroup` или `channel`; `chat_id` может разрешиться в любой из пяти kinds.
- Две разные kind для одного target внутри AND-clause дают `ContradictoryClause`; существующие caps остаются 16 clauses и 32 atoms на method.
- Generator сверяет exact canonical signatures четырёх pinned `ChatType` constructors. `channel` не является отдельным constructor: это `chatTypeSupergroup.is_channel:Bool`; изменение типа, порядка полей, отсутствие или extra constructor дают `SchemaDrift`.
- Capability policy/canonical artifact format поднят до `2`; policy format `1` отклоняется. Vendor manifest и owner policy сохраняют свои независимые format `1`.

## Exact reviewed contracts

- `deleteChatMessagesBySender`: `supergroup AND can_delete_messages`.
- `addChatMember`: отдельные branches `basic_group|supergroup|channel AND can_invite_users`; private/secret не принимаются.
- `upgradeBasicGroupChatToSupergroupChat`: `basic_group AND owner`, regular-user boundary сохранён.
- `setSupergroupStickerSet`: `supergroup AND can_change_info` на `supergroup_id`.
- `toggleForumTopicIsClosed`: `supergroup` присутствует в обеих branches: `can_manage_topics` или `topic_creator`.
- `unpinChatMessage`: `private` или `secret` без extra right; `basic_group|supergroup AND can_pin_messages`; `channel AND can_edit_messages`.

## Corpus disposition evidence

- Recognizer не расширялся: exact signal set остаётся 193 methods, SHA-256 `cbe074623352b1b4e970af939aed6297e7ce37366d7fd5ad7cedcf1a36848706`.
- Exact supported real set теперь содержит 6 methods, SHA-256 `ea3222e73264dc7188935067c81fdb459ef3566a5081a65e6660b47f48e899a9`.
- `unpinChatMessage` переведён из open в supported только после exact five-branch DNF. Остальные 187 methods остаются fail-closed, open-set SHA-256 `beea6c14d42a85c8ec6bd3fe322b3d05fa7e3b7f916d134877bea54746e13c03`.
- Method выходит из open set только после полного consumption распознанного current contract; частичная поддержка по-прежнему не считается disposition.

## TDD, verification and resources

- Red: core test потребовал closed vocabulary/target compatibility/contradiction rejection; generator tests потребовали DTO parsing и exact pinned conditional contract, а exhaustive match не компилировался без canonical serialization.
- Green/refactor: exact DTO/domain/canonical path, schema pin и six-method DNF реализованы; `ChatType` validator упрощён до сравнения canonical signature sets.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test --locked --offline --workspace --all-targets --jobs 2 -- --test-threads=2`: generator 41/41, core 21/21, остальные targets green.
- Clippy `-D warnings`, fmt, `git diff --check`, workspace boundary и exact TDLib pin green. Два independent final reviews вернули `Approved` без findings.
- Новых dependencies, threads, subprocesses, network/runtime resources или temp instances нет. `target` после проверок — 145 MiB; фоновых Cargo/Rust/product процессов не осталось.
- Source SHA-256: `method_capability.rs` — `9a4142a8b0e8ead0765ce8f2262d507f394e5d57731d2a63343e195e8bda1f54`; `method_capability/tests.rs` — `7afb7f32656818037527bc1d2651caa1eefcbe0833bf2d8ea2bc784b426b1b79`; `capability.rs` — `714608d367c22bf2b8f00d3274f90bdf7b9fa75df24164104f943902859cea08`; `capability/tests.rs` — `81ebfd0e8a529912f6839ad72dc013e90cf360ecbe8415c677fa3871a91e6f28`.

## Boundary

- 187 open runtime-signal methods не считаются capability coverage.
- Per-signal disposition artifact, `MessageProperties`/object-field/option predicates, полный 1010-method capability policy/artifact и runtime evaluator остаются open.
- Risk/prerequisite/retry classification, generated registry/runtime и live acceptance этим checkpoint не реализованы.
