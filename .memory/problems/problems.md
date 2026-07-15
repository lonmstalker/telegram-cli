# Problem Journal

Active append-only problem lifecycle. Status changes добавляются новой entry с тем же `P-*` ID.

## [2026-07-15] open correction | P-20260715-005 | addChatMember correction увеличила open set до 125 methods

- Corrects: preceding 124-method current state, где incomplete `addChatMember` считался complete.
- Evidence: [addChatMember correction digest](../raw/2026-07-15-tdlib-add-chat-member-overclaim-correction.md); supported 65, terminal 68, open-set SHA-256 `ff2f1639bd2947b460ebac2d7a733e71556619db8804ebe49f7410e73cd13af6`.
- Status: open; zero-open gate не достигнут, 125 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: закрывать exact source families только после dispatcher/deeper-handler evidence; subtype/account-sensitive methods fail closed.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-022](../decisions/decisions.md).

## [2026-07-15] resolved update | P-20260715-009 | Reviewer сохраняет singular membership contract deferred

- Evidence: [supergroup subtype capability digest](../raw/2026-07-15-tdlib-supergroup-flag-capabilities.md); independent reviewer подтвердил, что self-join обходит `can_invite_users`, а non-self path требует right.
- Resolution: false-positive correction остаётся в силе; ни singular, ни size-one plural path не возвращены в supported set без typed self/non-self/cardinality invocation partition.
- Status: resolved как устранённый overclaim; missing singular capability остаётся частью [P-20260715-005](../problems/problems.md), а не новым ложным contract.
- Related decision: [D-20260715-023](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Ordinary-supergroup subtype contract уменьшил open set до 124 methods

- Evidence: [supergroup subtype capability digest](../raw/2026-07-15-tdlib-supergroup-flag-capabilities.md); exact supported set 66, terminal set 69, open-set SHA-256 `437c17ed2ccb09f23aa7eba6b04223e0b05a97ae55493d280fa18f28fe7ce796`.
- Transition: один method получает exact account/kind/subtype/right contract; оба self/cardinality-dependent invite methods остаются deferred; capability format становится `8`.
- Status: open; zero-open gate не достигнут, 124 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: продолжать reviewed source-family tasks; runtime subtype evidence обязано быть current-session, target/account/DC-bound и fail closed на gap/staleness.
- Archive link map после ротации: [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md).
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D-20260715-023](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Chat event log contract уменьшил open set до 123 methods

- Evidence: [chat event log capability digest](../raw/2026-07-15-tdlib-chat-event-log-capability.md); exact supported set 67, terminal set 70, open-set SHA-256 `a142adc309d4c392ae78f34437eb0568b23b4e69d0a576db335bab659b572b10`.
- Transition: `getChatEventLog` получил exact regular-user, supergroup/channel и administrator contract; capability format остался `8`.
- Status: open; zero-open gate не достигнут, 123 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: продолжать exact source-family tasks; runtime role evidence обязано быть current-session, target/account-bound и fail closed на gap/staleness.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D-20260715-024](../decisions/decisions.md).

## [2026-07-15] open | P-20260715-010 | unpinChatMessage contract пропускал deeper-handler branches

- Evidence: [unpinChatMessage correction digest](../raw/2026-07-15-tdlib-unpin-chat-message-overclaim-correction.md); pinned `DialogManager::can_pin_messages` запрещает secret chat, conditional по account для basic group и отделяет monoforum.
- Impact: old DNF мог дать false-positive для secret chat и bot/basic-group, а также ложно требовать ordinary right в monoforum.
- Status: open at discovery; real method ошибочно считался terminally dispositioned.
- Next check: удалить incomplete row до commit и закрепить pinned deferred regression.
- Related decision: [D-20260715-025](../decisions/decisions.md).

## [2026-07-15] resolved | P-20260715-010 | unpinChatMessage возвращён в deferred

- Evidence: real reviewed row удалён; pinned regression требует `SchemaDrift`; generic fixture переименован и не входит в pinned corpus; independent reviewer дал `APPROVED`.
- Resolution: false-positive coverage устранён без speculative grammar и hidden policy narrowing.
- Status: resolved for current implementation; missing complete capability остаётся частью [P-20260715-005](../problems/problems.md).
- Related decision: [D-20260715-025](../decisions/decisions.md).

## [2026-07-15] open correction | P-20260715-005 | unpinChatMessage correction увеличила open set до 124 methods

- Corrects: preceding 123-method current state, где incomplete `unpinChatMessage` считался complete.
- Evidence: [unpinChatMessage correction digest](../raw/2026-07-15-tdlib-unpin-chat-message-overclaim-correction.md); supported 66, terminal 69, open-set SHA-256 `ffd5fe2eed81664bc9e2d07d80582faf5a19531c553c36e92fd5096cfe759fb1`.
- Status: open; zero-open gate не достигнут, 124 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: добавлять complete rows только после dispatcher/deeper-handler review и exact invocation/account/kind partition.
- Related decisions: [D-20260715-010](../decisions/archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D-20260715-025](../decisions/decisions.md).

## [2026-07-15] archive link map | P-20260715-005 | Rotated historical decision target

- Immutable source entry: [runtime boolean option update](archive/2026-07-15--2026-07-15-008.md).
- Canonical decision target after rotation: [D-20260715-010](../decisions/archive/2026-07-15--2026-07-15-009.md). The archived relative link remains historical and is not rewritten.
- Status: no change; latest open boundary remains 124 methods in the preceding correction entry.

## [2026-07-15] open update | P-20260715-005 | Invite-link counts contract уменьшил open set до 123 methods

- Evidence: [invite-link counts digest](../raw/2026-07-15-tdlib-chat-invite-link-counts-capability.md); supported 67, terminal 70, open-set SHA-256 `38dd369d689f9924166f54934b1e4207ddfd9fec692e3f4219b76dac4ee19fbb`.
- Transition: `getChatInviteLinkCounts` получил exact regular-user, three-kind owner contract; format остался `8`.
- Status: open; 123 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: продолжать exact dispatcher/deeper-handler tasks; runtime owner/write evidence fail closed на stale/gap.
- Related decisions: [D-20260715-010](../decisions/archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D-20260715-026](../decisions/decisions.md).

## [2026-07-15] archive link map | P-20260715-005 | Rotated planning and owner targets

- Immutable [problem shard 009](archive/2026-07-15--2026-07-15-009.md) и ротируемая P006 entry в shard 010 ссылаются на [canonical D-20260715-017](../decisions/archive/2026-07-15--2026-07-15-019.md).
- Active historical owner update ссылается на [canonical D-20260715-018](../decisions/archive/2026-07-15--2026-07-15-020.md). Latest open boundary остаётся 123 methods.

## [2026-07-15] archive link correction | P-20260715-005 | Exact rotated entries

- Corrects preceding map: [shard 009](archive/2026-07-15--2026-07-15-009.md) содержит P006 resolved и ссылается на [D-20260715-017](../decisions/archive/2026-07-15--2026-07-15-019.md).
- [Shard 010](archive/2026-07-15--2026-07-15-010.md) содержит username-owner P005 update; canonical targets: [D-20260715-010](../decisions/archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D-20260715-018](../decisions/archive/2026-07-15--2026-07-15-020.md).
- Owner update больше не active; latest open boundary остаётся 123 methods.

## [2026-07-15] archive link map | P-20260715-005 | Invite-link creation shard

- [Shard 011](archive/2026-07-15--2026-07-15-011.md) содержит historical invite-link creation update; canonical targets: [D-20260715-010](../decisions/archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D-20260715-019](../decisions/decisions.md).
- Status не изменён; latest open boundary — 123 methods.

## [2026-07-15] open update | P-20260715-005 | Video chat RTMP access уменьшил open set до 122 methods

- Evidence: [video chat RTMP access digest](../raw/2026-07-15-tdlib-video-chat-rtmp-access-capability.md); supported 68, terminal 71, open-set SHA-256 `df35fcbf3d7ed48c81bba37beaeea8d407d8066ba4b90f1ff8c8bc9ce59e35da`.
- Transition: `getVideoChatRtmpUrl` получил exact regular-user, three-kind и `can_manage_video_chats` contract; format остался `8`.
- Status: open; 122 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: продолжать exact dispatcher/deeper-handler tasks; runtime dialog/right evidence fail closed на stale/gap.
- Related decisions: [D-20260715-010](../decisions/archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D-20260715-027](../decisions/decisions.md).

## [2026-07-15] archive link map | P-20260715-005 | Rotated setting decisions

- Shard 011 и active invite-link map: canonical [D-20260715-019](../decisions/archive/2026-07-15--2026-07-15-021.md).
- Shard 012: canonical D010 shard 009, D012 shard 011; D020 остаётся active. Historical links immutable; status остаётся open at 122.

## [2026-07-15] link correction | P-20260715-005 | Shard 012

- Shard 012: [D010](../decisions/archive/2026-07-15--2026-07-15-009.md), [D012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D020](../decisions/archive/2026-07-15--2026-07-15-022.md); open 122.

## [2026-07-15] link correction | P-20260715-005 | D020 split

- Shard-012 D020 resolves [base](../decisions/archive/2026-07-15--2026-07-15-022.md) + [accepted correction](../decisions/decisions.md); open 122.

## [2026-07-15] open update | P-20260715-005 | RTMP replacement уменьшил open set до 121

- Evidence: [replacement digest](../raw/2026-07-15-tdlib-video-chat-rtmp-replacement-capability.md); open SHA-256 `f12c4e511942b14979dc26a17bc4797ff05bbcaceda7f45625829960222faf0c`.
- Transition: exact regular-user, three-kind owner revoke contract; supported 69, terminal 72, format `8`.
- Status/next: open at 121; продолжать exact handler tasks, runtime owner/read evidence fail closed на stale/gap.
- Decision: [D-20260715-028](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Video-chat creation уменьшил open set до 120

- Evidence: [creation digest](../raw/2026-07-15-tdlib-video-chat-creation-capability.md); open SHA-256 `c525212cc279557aae39bd821e2c74d8912c01f0595e9f35f19f1259e7e4922d`.
- Transition: exact regular-user, three-kind `can_manage_video_chats` creation contract; supported 70, terminal 73, format `8`.
- Status/next: open at 120; продолжать exact handler tasks, runtime right/read evidence fail closed на stale/gap.
- Decision: [D-20260715-029](../decisions/decisions.md).

## [2026-07-15] archive link map | P-20260715-005 | Rotated ordinary-setting correction

- [Problem shard 015](archive/2026-07-15--2026-07-15-015.md) ссылается на D020; canonical resolve: [base](../decisions/archive/2026-07-15--2026-07-15-022.md) + [accepted correction](../decisions/archive/2026-07-15--2026-07-15-023.md).
- Status не изменён: latest open boundary — 120 methods.

## [2026-07-15] open | P-20260715-011 | deleteChatMessagesBySender allowed bot and direct-messages false positives

- Evidence: pinned dispatcher `CHECK_IS_USER`; deeper handler rejects monoforum before checking `can_delete_messages`; red bot descriptor returned `Ok`, broad DNF omitted subtype flag.
- Impact: bot or direct-messages supergroup could receive a false supported verdict.
- Next: replace generic row with exact account/kind/subtype/right contract before further coverage work.
- Decision: [D-20260715-030](../decisions/decisions.md).

## [2026-07-15] resolved | P-20260715-011 | Exact moderation contract replaces broad DNF

- Evidence: [correction digest](../raw/2026-07-15-tdlib-delete-chat-messages-by-sender-correction.md); bot-enabled and broad-supergroup descriptors now fail, exact regular-user non-direct-messages-supergroup descriptor passes; reviewer `APPROVED`.
- Resolution: semantic module pins signature/source/account/kind/subtype/right without changing 70/73/120 corpus counts.
- Status: resolved for static contract; runtime freshness remains future daemon work.
- Decision: [D-20260715-030](../decisions/decisions.md).

## [2026-07-15] archive link map | P-20260715-011 | Rotated P008 lifecycle

- [Problem shard 016](archive/2026-07-15--2026-07-15-016.md) содержит open/resolved P008; canonical [D021](../decisions/archive/2026-07-15--2026-07-15-023.md). Resolved status unchanged.

## [2026-07-15] open update | P-20260715-005 | Recent-reaction moderation reduces open set to 119

- Evidence: [capability digest](../raw/2026-07-15-tdlib-delete-recent-reactions-by-sender-capability.md); open SHA-256 `27dd1e3d3014e2d30880e69b8adf865969c4ec9a536fbf46c167835c5b1c6ca2`.
- Transition: exact regular-user/bot, basic-group/supergroup `can_delete_messages` contract; supported 71, terminal 74, format `8`.
- Status/next: open at 119; continue exact handler tasks, runtime write/right/sender evidence fails closed on stale/gap.
- Decision: [D-20260715-031](../decisions/decisions.md).

## [2026-07-15] archive link map | P-20260715-005 | Rotated setting update and D022

- [Problem shard 017](archive/2026-07-15--2026-07-15-017.md) contains historical 124-method setting update; canonical [D010](../decisions/archive/2026-07-15--2026-07-15-009.md), [D012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D021](../decisions/archive/2026-07-15--2026-07-15-023.md).
- Active P009 links and W026 shard resolve D022 at [decision shard 024](../decisions/archive/2026-07-15--2026-07-15-024.md). Latest P005 boundary remains 119.

## [2026-07-15] open update | P-20260715-005 | Channel gift notifications reduce open set to 118

- Evidence: [capability digest](../raw/2026-07-15-tdlib-chat-gift-notification-capability.md); open SHA-256 `090cf24de23ace4b7bc1a9b9115181afacdca75b23eff0d8506fc0efc5a6c29a`.
- Transition: exact regular-user, channel `can_post_messages` contract; supported 72, terminal 75, format `8`.
- Status/next: open at 118; continue exact handler tasks, runtime read/kind/right evidence fails closed on stale/gap.
- Decision: [D-20260715-032](../decisions/decisions.md).

## [2026-07-15] archive link map | P-20260715-009 | Rotated discovery and D023

- [Problem shard 018](archive/2026-07-15--2026-07-15-018.md) contains the P009 discovery entry; the resolved correction/update remain active above.
- P009 and ordinary-supergroup P005 links resolve accepted D023 at [decision shard 025](../decisions/archive/2026-07-15--2026-07-15-025.md); latest P005 boundary remains 118.

## [2026-07-15] archive link correction | P-20260715-009 | Discovery decision target

- Immutable problem shard 018 historical D022 link resolves to [decision shard 024](../decisions/archive/2026-07-15--2026-07-15-024.md); P009 status remains resolved.
- Immutable problem shard 019 resolved-entry D022 link resolves to the same decision shard 024; lifecycle remains resolved.
- Exact lifecycle correction: discovery is shard 018, first resolution is shard 019, and the later reviewer-confirmed resolved update remains active above.
