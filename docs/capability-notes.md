# Reviewed TDLib capability contracts

Справочник результатов ручного ревью capability-требований методов закреплённой схемы
TDLib 1.8.66 (commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`, `vendor/tdlib/td_api.tl`).

Происхождение: стартовые контракты были закодированы в удалённом documentation-recognizer engine
(`tools/tdlib-registry-gen/src/capability*`, см. git history до этого коммита) и выгружены
из него машинно перед удалением. Ревью опиралось на pinned schema documentation и pinned
TDLib C++ sources (`Requests.cpp` и доменные менеджеры).

Добавления P4 ревьюятся вручную по мере появления фактического workflow consumer. Для
первого chat slice read-resolvers и membership calls намеренно ограничены основным
`regular_user` account: более широкий bot scope остаётся default-deny до отдельного evidence.

Canonical machine-readable source после P3 —
[`tools/tdlib-registry-gen/capabilities.json`](../tools/tdlib-registry-gen/capabilities.json).
Таблица ниже сохраняет human-readable evidence account/runtime review; generator не
парсит Markdown и не выводит contracts из текста схемы.

Как читать:

- **Accounts** — статический account scope метода: `regular_user`, `bot` или оба.
- **Runtime requirements** — DNF-условие поверх свежего TDLib-состояния: `OR`-ветки, внутри — `AND`-атомы.
  Атомы ссылаются на аргументы запроса (`chat_id`, `message_id`, …) и проверяются в runtime:
  `ChatKind` — тип чата; `ChatAdministratorRight`/`ChatMemberRight` — право текущего аккаунта;
  `ChatOwner`/`ChatAdministrator` — статус; `SupergroupFlag` — булев флаг supergroup;
  `MessageCapability` — поле `messageProperties` конкретного сообщения; `GroupCall*` — аналоги
  для групповых звонков; `BooleanOptionEnabled` — runtime option.
- Всё, чего нет в таблице, — **default-deny** до ревью (правило `plans.md`).
- `risk`: `read`, `presence`, `send`, `reversible_mutation`, `admin`, `destructive`,
  `financial`, `auth_security`.
- `retry`: `safe_read` разрешает обычный read retry; `convergent` — повтор exact
  desired state; `reconcile` требует проверки server state вместо blind retry;
  `never` запрещает automatic retry.
- Generator создаёт descriptor для каждого pinned method. Reviewed rows получают эти
  поля, остальные — только `DefaultDeny` без угаданной классификации.

## Reviewed contracts (105)

| Method | Accounts | Runtime requirements |
|---|---|---|
| `addChecklistTasks` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanAddTasks }` |
| `addOffer` | regular_user, bot | `(MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanAddOffer }) OR (MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanEditSuggestedPostInfo })` |
| `approveSuggestedPost` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBeApproved }` |
| `banGroupCallParticipants` | regular_user, bot | `GroupCallKind { group_call: group_call_id, kind: Unbound } AND GroupCallProperty { group_call: group_call_id, property: IsOwned }` |
| `checkChatInviteLink` | regular_user | `AuthorizationState { state: Ready }` |
| `cancelDownloadFile` | regular_user | `FileKnown { target: file_id }` |
| `closeChat` | regular_user | `ChatKnown { target: chat_id }` |
| `closeWebApp` | regular_user | `WebAppLaunchKnown { launch: web_app_launch_id }` |
| `createChatInviteLink` | regular_user, bot | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatAdministratorRight { target: chat_id, right: CanInviteUsers }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministratorRight { target: chat_id, right: CanInviteUsers }) OR (ChatKind { target: chat_id, kind: Channel } AND ChatAdministratorRight { target: chat_id, right: CanInviteUsers })` |
| `createVideoChat` | regular_user | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatAdministratorRight { target: chat_id, right: CanManageVideoChats }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministratorRight { target: chat_id, right: CanManageVideoChats }) OR (ChatKind { target: chat_id, kind: Channel } AND ChatAdministratorRight { target: chat_id, right: CanManageVideoChats })` |
| `declineSuggestedPost` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBeDeclined }` |
| `deleteAllRecentMessageReactionsFromSender` | regular_user, bot | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatAdministratorRight { target: chat_id, right: CanDeleteMessages }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministratorRight { target: chat_id, right: CanDeleteMessages })` |
| `deleteChatMessagesBySender` | regular_user | `ChatKind { target: chat_id, kind: Supergroup } AND SupergroupFlag { target: chat_id, flag: IsDirectMessagesGroup, value: false } AND ChatAdministratorRight { target: chat_id, right: CanDeleteMessages }` |
| `deleteGroupCallMessages` | regular_user, bot | `GroupCallKind { group_call: group_call_id, kind: LiveStory } AND GroupCallMessageCapability { subject: Each { group_call: group_call_id, messages: message_ids }, capability: CanBeDeleted }` |
| `deleteGroupCallMessagesBySender` | regular_user, bot | `GroupCallKind { group_call: group_call_id, kind: LiveStory } AND GroupCallProperty { group_call: group_call_id, property: CanDeleteMessages }` |
| `deleteMessageReactionsFromSender` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanDeleteReactions }` |
| `disableAllSupergroupUsernames` | regular_user | `(ChatKind { target: supergroup_id, kind: Supergroup } AND ChatOwner { target: supergroup_id }) OR (ChatKind { target: supergroup_id, kind: Channel } AND ChatOwner { target: supergroup_id })` |
| `downloadFile` | regular_user | `FileKnown { target: file_id }` |
| `editMessageCaption` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBeEdited }` |
| `editMessageChecklist` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBeEdited }` |
| `editMessageLiveLocation` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBeEdited }` |
| `editMessageMedia` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanEditMedia }` |
| `editMessageReplyMarkup` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBeEdited }` |
| `editMessageSchedulingState` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanEditSchedulingState }` |
| `editMessageText` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBeEdited }` |
| `endGroupCall` | regular_user, bot | `(GroupCallKind { group_call: group_call_id, kind: VideoChat } AND GroupCallProperty { group_call: group_call_id, property: CanBeManaged }) OR (GroupCallKind { group_call: group_call_id, kind: LiveStory } AND GroupCallProperty { group_call: group_call_id, property: CanBeManaged }) OR (GroupCallKind { group_call: group_call_id, kind: Unbound } AND GroupCallProperty { group_call: group_call_id, property: IsOwned })` |
| `endGroupCallRecording` | regular_user, bot | `GroupCallKind { group_call: group_call_id, kind: VideoChat } AND GroupCallProperty { group_call: group_call_id, property: CanBeManaged }` |
| `getBasicGroupFullInfo` | regular_user | `BasicGroupKnown { target: basic_group_id }` |
| `getChat` | regular_user | `AuthorizationState { state: Ready }` |
| `getChatHistory` | regular_user | `ChatKnown { target: chat_id }` |
| `getChatBoosts` | regular_user | `ChatAdministrator { target: chat_id }` |
| `getChatEventLog` | regular_user | `(ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministrator { target: chat_id }) OR (ChatKind { target: chat_id, kind: Channel } AND ChatAdministrator { target: chat_id })` |
| `getChatInviteLinkCounts` | regular_user | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatOwner { target: chat_id }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatOwner { target: chat_id }) OR (ChatKind { target: chat_id, kind: Channel } AND ChatOwner { target: chat_id })` |
| `getChatStatistics` | regular_user | `SupergroupFullInfoProperty { target: chat_id, property: CanGetStatistics }` |
| `getCurrentState` | regular_user | `AuthorizationState { state: Ready }` |
| `getForumTopic` | regular_user, bot | `ChatKnown { target: chat_id }` |
| `getForumTopics` | regular_user, bot | `ChatKnown { target: chat_id }` |
| `getFile` | regular_user | `FileKnown { target: file_id }` |
| `getMe` | regular_user, bot | `AuthorizationState { state: Ready }` |
| `getMessageAuthor` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetAuthor }` |
| `getMessageEmbeddingCode` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetEmbeddingCode }` |
| `getMessagePublicForwards` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetStatistics }` |
| `getMessageReadDate` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetReadDate }` |
| `getMessageStatistics` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetStatistics }` |
| `getMessageThread` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetMessageThread }` |
| `getMessageThreadHistory` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetMessageThread }` |
| `getMessageViewers` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetViewers }` |
| `getPollVoteStatistics` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetPollVoteStatistics }` |
| `getStatisticalGraph` | regular_user | `ChatKnown { target: chat_id }` |
| `getSupergroupFullInfo` | regular_user | `SupergroupKnown { target: supergroup_id }` |
| `getSupergroupMembers` | regular_user | `SupergroupFullInfoProperty { target: supergroup_id, property: CanGetMembers }` |
| `getUser` | regular_user, bot | `AuthorizationState { state: Ready }` |
| `getUserChatBoosts` | bot | `ChatAdministrator { target: chat_id }` |
| `getUserFullInfo` | regular_user | `UserKnown { target: user_id }` |
| `getVideoChatRtmpUrl` | regular_user | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatAdministratorRight { target: chat_id, right: CanManageVideoChats }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministratorRight { target: chat_id, right: CanManageVideoChats }) OR (ChatKind { target: chat_id, kind: Channel } AND ChatAdministratorRight { target: chat_id, right: CanManageVideoChats })` |
| `getVideoMessageAdvertisements` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanGetVideoAdvertisements }` |
| `joinChat` | regular_user | `ChatKnown { target: chat_id }` |
| `joinChatByInviteLink` | regular_user | `AuthorizationState { state: Ready }` |
| `loadChats` | regular_user | `AuthorizationState { state: Ready }` |
| `markChecklistTasksAsDone` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanMarkTasksAsDone }` |
| `openChat` | regular_user | `ChatKnown { target: chat_id }` |
| `openWebApp` | regular_user | `ChatKnown { target: chat_id } AND BotUserKnown { target: bot_user_id }` |
| `pinChatMessage` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBePinned }` |
| `recognizeSpeech` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanRecognizeSpeech }` |
| `reorderSupergroupActiveUsernames` | regular_user | `(ChatKind { target: supergroup_id, kind: Supergroup } AND ChatOwner { target: supergroup_id }) OR (ChatKind { target: supergroup_id, kind: Channel } AND ChatOwner { target: supergroup_id })` |
| `replacePrimaryChatInviteLink` | regular_user, bot | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatAdministratorRight { target: chat_id, right: CanInviteUsers }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministratorRight { target: chat_id, right: CanInviteUsers }) OR (ChatKind { target: chat_id, kind: Channel } AND ChatAdministratorRight { target: chat_id, right: CanInviteUsers })` |
| `replaceVideoChatRtmpUrl` | regular_user | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatOwner { target: chat_id }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatOwner { target: chat_id }) OR (ChatKind { target: chat_id, kind: Channel } AND ChatOwner { target: chat_id })` |
| `reportMessageReactions` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanReportReactions }` |
| `reportSupergroupSpam` | regular_user, bot | `ChatKind { target: supergroup_id, kind: Supergroup } AND ChatAdministrator { target: supergroup_id } AND MessageCapability { subject: Each { chat: supergroup_id, messages: message_ids }, capability: CanReportSupergroupSpam }` |
| `revokeGroupCallInviteLink` | regular_user, bot | `(GroupCallKind { group_call: group_call_id, kind: VideoChat } AND GroupCallProperty { group_call: group_call_id, property: CanBeManaged }) OR (GroupCallKind { group_call: group_call_id, kind: Unbound } AND GroupCallProperty { group_call: group_call_id, property: IsOwned })` |
| `searchChatMessages` | regular_user | `ChatKnown { target: chat_id } AND MessageDatabaseEnabled` |
| `searchPublicChat` | regular_user | `AuthorizationState { state: Ready }` |
| `sendBotStartMessage` | regular_user | `ChatKnown { target: chat_id } AND BotUserKnown { target: bot_user_id }` |
| `sendGroupCallMessage` | regular_user, bot | `GroupCallProperty { group_call: group_call_id, property: CanSendMessages }` |
| `sendMessage` | regular_user, bot | `ChatKnown { target: chat_id }` |
| `setName` | regular_user | `AuthorizationState { state: Ready }` |
| `setChatDescription` | regular_user, bot | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatMemberRight { target: chat_id, right: CanChangeInfo }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatMemberRight { target: chat_id, right: CanChangeInfo }) OR (ChatKind { target: chat_id, kind: Channel } AND ChatMemberRight { target: chat_id, right: CanChangeInfo })` |
| `setChatTitle` | regular_user | `ChatMemberRight { target: chat_id, right: CanChangeInfo }` |
| `setChatLocation` | regular_user | `SupergroupFullInfoProperty { target: chat_id, property: CanSetLocation }` |
| `setChatPaidMessageStarCount` | regular_user | `ChatAdministratorRight { target: chat_id, right: CanRestrictMembers } AND SupergroupFullInfoProperty { target: chat_id, property: CanEnablePaidMessages }` |
| `setChatPermissions` | regular_user, bot | `(ChatKind { target: chat_id, kind: BasicGroup } AND ChatAdministratorRight { target: chat_id, right: CanRestrictMembers }) OR (ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministratorRight { target: chat_id, right: CanRestrictMembers })` |
| `setChatSlowModeDelay` | regular_user | `ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministratorRight { target: chat_id, right: CanRestrictMembers }` |
| `setGroupCallPaidMessageStarCount` | regular_user, bot | `GroupCallKind { group_call: group_call_id, kind: LiveStory } AND GroupCallProperty { group_call: group_call_id, property: CanBeManaged }` |
| `setMessageFactCheck` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanSetFactCheck }` |
| `setNewChatPrivacySettings` | regular_user | `BooleanOptionEnabled { option: CanSetNewChatPrivacySettings }` |
| `setSupergroupMainProfileTab` | regular_user | `ChatKind { target: supergroup_id, kind: Channel } AND ChatAdministratorRight { target: supergroup_id, right: CanChangeInfo }` |
| `setSupergroupStickerSet` | regular_user, bot | `ChatKind { target: supergroup_id, kind: Supergroup } AND ChatAdministratorRight { target: supergroup_id, right: CanChangeInfo }` |
| `setSupergroupUnrestrictBoostCount` | regular_user, bot | `ChatKind { target: supergroup_id, kind: Supergroup } AND ChatAdministratorRight { target: supergroup_id, right: CanRestrictMembers }` |
| `setSupergroupUsername` | regular_user | `(ChatKind { target: supergroup_id, kind: Supergroup } AND ChatOwner { target: supergroup_id }) OR (ChatKind { target: supergroup_id, kind: Channel } AND ChatOwner { target: supergroup_id })` |
| `setVideoChatTitle` | regular_user, bot | `GroupCallKind { group_call: group_call_id, kind: VideoChat } AND GroupCallProperty { group_call: group_call_id, property: CanBeManaged }` |
| `startGroupCallRecording` | regular_user, bot | `GroupCallKind { group_call: group_call_id, kind: VideoChat } AND GroupCallProperty { group_call: group_call_id, property: CanBeManaged }` |
| `stopPoll` | regular_user, bot | `MessageCapability { subject: One { chat: chat_id, message: message_id }, capability: CanBeEdited }` |
| `toggleChatGiftNotifications` | regular_user | `ChatKind { target: chat_id, kind: Channel } AND ChatAdministratorRight { target: chat_id, right: CanPostMessages }` |
| `toggleForumTopicIsClosed` | regular_user, bot | `(ChatKind { target: chat_id, kind: Supergroup } AND ChatAdministratorRight { target: chat_id, right: CanManageTopics }) OR (ChatKind { target: chat_id, kind: Supergroup } AND TopicCreator { target: chat_id, topic: forum_topic_id })` |
| `toggleGroupCallAreMessagesAllowed` | regular_user, bot | `GroupCallProperty { group_call: group_call_id, property: CanToggleAreMessagesAllowed }` |
| `toggleSupergroupHasAggressiveAntiSpamEnabled` | regular_user | `SupergroupFullInfoProperty { target: supergroup_id, property: CanToggleAggressiveAntiSpam }` |
| `toggleSupergroupHasHiddenMembers` | regular_user | `SupergroupFullInfoProperty { target: supergroup_id, property: CanHideMembers }` |
| `toggleSupergroupIsAllHistoryAvailable` | regular_user | `ChatKind { target: supergroup_id, kind: Supergroup } AND ChatMemberRight { target: supergroup_id, right: CanChangeInfo }` |
| `toggleSupergroupJoinToSendMessages` | regular_user | `ChatKind { target: supergroup_id, kind: Supergroup } AND SupergroupFlag { target: supergroup_id, flag: IsBroadcastGroup, value: false } AND SupergroupFlag { target: supergroup_id, flag: IsDirectMessagesGroup, value: false } AND ChatAdministratorRight { target: supergroup_id, right: CanRestrictMembers }` |
| `toggleSupergroupSignMessages` | regular_user | `ChatKind { target: supergroup_id, kind: Channel } AND ChatMemberRight { target: supergroup_id, right: CanChangeInfo }` |
| `toggleSupergroupUsernameIsActive` | regular_user | `(ChatKind { target: supergroup_id, kind: Supergroup } AND ChatOwner { target: supergroup_id }) OR (ChatKind { target: supergroup_id, kind: Channel } AND ChatOwner { target: supergroup_id })` |
| `toggleVideoChatMuteNewParticipants` | regular_user, bot | `GroupCallKind { group_call: group_call_id, kind: VideoChat } AND GroupCallProperty { group_call: group_call_id, property: CanToggleMuteNewParticipants }` |
| `upgradeBasicGroupChatToSupergroupChat` | regular_user | `ChatKind { target: chat_id, kind: BasicGroup } AND ChatOwner { target: chat_id }` |
| `uploadStickerFile` | regular_user, bot | `AuthorizationState { state: Ready } AND ScopedLocalOrRemoteFile { target: sticker }` |
| `viewMessages` | regular_user | `ChatKnown { target: chat_id }` |

## Recognized but unclassified (115)

`addChatMember`, `addChatMembers`, `addGiftCollectionGifts`, `addPollOption`, `addStoryAlbumStories`, `banChatMember`, `canPostStory`, `createChatSubscriptionInviteLink`, `createForumTopic`, `createGiftCollection`, `createStoryAlbum`, `deleteAllRevokedChatInviteLinks`, `deleteChat`, `deleteChatHistory`, `deleteForumTopic`, `deleteGiftCollection`, `deleteMessages`, `deletePollOption`, `deleteRevokedChatInviteLink`, `deleteStory`, `deleteStoryAlbum`, `editChatInviteLink`, `editChatSubscriptionInviteLink`, `editForumTopic`, `editQuickReplyMessage`, `editStory`, `editStoryCover`, `forwardMessages`, `getBotInfoDescription`, `getBotInfoShortDescription`, `getBotName`, `getChatArchivedStories`, `getChatInviteLink`, `getChatInviteLinkMembers`, `getChatInviteLinks`, `getChatJoinRequests`, `getChatOwnerAfterLeaving`, `getChatRevenueStatistics`, `getChatRevenueTransactions`, `getChatRevenueWithdrawalUrl`, `getChatStoryInteractions`, `getLiveStoryRtmpUrl`, `getMessageAddedReactions`, `getMessageImportConfirmationText`, `getMessageLink`, `getPollVoters`, `getReceivedGifts`, `getStarRevenueStatistics`, `getStarTransactions`, `getStoryPublicForwards`, `getStoryStatistics`, `getSupergroupMembers`, `getUpgradedGiftWithdrawalUrl`, `getVideoChatInviteLink`, `importMessages`, `postStory`, `processChatJoinRequests`, `readdQuickReplyShortcutMessages`, `removeGiftCollectionGifts`, `removeStoryAlbumStories`, `reorderBotActiveUsernames`, `reorderGiftCollectionGifts`, `reorderGiftCollections`, `reorderStoryAlbumStories`, `reorderStoryAlbums`, `replaceLiveStoryRtmpUrl`, `reportChat`, `reportChatPhoto`, `reportSupergroupAntiSpamFalsePositive`, `resendMessages`, `resetAuthenticationEmailAddress`, `revokeChatInviteLink`, `searchChatMembers`, `sellGift`, `sendBotStartMessage`, `setBotInfoDescription`, `setBotInfoShortDescription`, `setBotName`, `setChatAccentColor`, `setChatAvailableReactions`, `setChatBackground`, `setChatDirectMessagesGroup`, `setChatDiscussionGroup`, `setChatEmojiStatus`, `setChatMemberStatus`, `setChatMemberTag`, `setChatMessageAutoDeleteTime`, `setChatPhoto`, `setChatPinnedStories`, `setChatProfileAccentColor`, `setGiftCollectionName`, `setPinnedForumTopics`, `setPinnedGifts`, `setStoryAlbumName`, `setStoryPrivacySettings`, `setSupergroupCustomEmojiStickerSet`, `shareChatWithBot`, `startLiveStory`, `toggleBotIsAddedToAttachmentMenu`, `toggleBotUsernameIsActive`, `toggleChatHasProtectedContent`, `toggleForumTopicIsPinned`, `toggleGeneralForumTopicIsHidden`, `toggleGiftIsSaved`, `toggleGroupCallParticipantIsHandRaised`, `toggleStoryIsPostedToChatPage`, `toggleSupergroupCanHaveSponsoredMessages`, `toggleSupergroupHasAutomaticTranslation`, `toggleSupergroupIsBroadcastGroup`, `toggleSupergroupIsForum`, `toggleSupergroupJoinByRequest`, `transferChatOwnership`, `unpinAllChatMessages`, `unpinAllForumTopicMessages`, `unpinChatMessage`

Причина: в документации метода распознан permission-signal, но полный контракт не доказан
по pinned sources. Известные результаты ревью:

- `addChatMember`, `addChatMembers` — dispatcher требует regular user; channel path запрещает
  direct-messages supergroup; singular self-join обходит invite right.
- `unpinChatMessage` — handler запрещает secret chat, для basic group зависит от account
  (bot appointed-admin guard), отдельно обрабатывает monoforum и проверяет message state.
- `setChatPhoto` — скрытый bot/basic-group appointed-admin guard; `setChatTitle` поэтому
  разрешён только regular-user consumer с runtime `can_change_info` proof.
- `postStory`, `getChatRevenueWithdrawalUrl` — смешанные caption/owner/full-info semantics.
- Остальные — семейства с недоказанными ветками: own/other invite links (9 методов),
  owner-prerequisite (13), `supergroupFullInfo`-свойства (7), `messageProperties` mixed (4).

## Остальные методы схемы (~820)

В их документации permission-signals не распознаны. Механические маркеры
(`for bots only`, `for Telegram Premium users only`, synchronous, auth-state) выводятся
из текста схемы напрямую и здесь не дублируются.
