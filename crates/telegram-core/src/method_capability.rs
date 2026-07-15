//! Статическая capability-модель TDLib method.
//!
//! Capability описывает проверяемую доступность Telegram API, но не разрешение
//! агенту выполнить вызов. Policy/risk остаются отдельным доменом.

use std::error::Error;
use std::fmt;

const MAX_ARGUMENT_NAME_BYTES: usize = 64;
pub const MAX_CLAUSES_PER_METHOD: usize = 16;
pub const MAX_ATOMS_PER_METHOD: usize = 32;
pub const MAX_PARAMETER_NOTICES_PER_METHOD: usize = 32;
pub const MAX_SYNCHRONOUS_VALUES_PER_METHOD: usize = 16;

macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $name:ident, $count:expr, $kind:literal, {
            $($variant:ident => $value:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: [Self; $count] = [$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ParseCapabilityValueError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(ParseCapabilityValueError::new($kind, value)),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

string_enum!(
    /// Тип авторизованного Telegram account.
    AccountKind,
    2,
    "account kind",
    {
        RegularUser => "regular_user",
        Bot => "bot",
    }
);

string_enum!(
    /// Entitlement именно текущего regular account, не получателя и не business connection.
    CurrentAccountEntitlement,
    2,
    "current-account entitlement",
    {
        Premium => "premium",
        Business => "business",
    }
);

string_enum!(
    /// Ограничение класса TDLib application.
    ApplicationRequirement,
    3,
    "application requirement",
    {
        Any => "any",
        Official => "official",
        OfficialMobile => "official_mobile",
    }
);

string_enum!(
    /// Telegram data-center environment, где method допустим.
    DcEnvironment,
    2,
    "DC environment",
    {
        Production => "production",
        Test => "test",
    }
);

string_enum!(
    /// Identifier space для chat-level runtime evidence.
    ChatTargetKind,
    2,
    "chat target kind",
    {
        ChatId => "chat_id",
        SupergroupId => "supergroup_id",
    }
);

string_enum!(
    /// Разрешённый runtime `ChatType` для конкретного chat target.
    ResolvedChatKind,
    5,
    "resolved chat kind",
    {
        Private => "private",
        BasicGroup => "basic_group",
        Supergroup => "supergroup",
        Channel => "channel",
        Secret => "secret",
    }
);

string_enum!(
    /// Exact pinned TDLib authorization-state inventory.
    ///
    /// `WaitTdlibParameters` также представляет созданный client до вызова
    /// `setTdlibParameters`; отдельная выдуманная pre-initialization state не
    /// нужна. Terminal states сохраняются в vocabulary для lifecycle evidence,
    /// но не входят в обычный requestable set генератора.
    AuthorizationState,
    13,
    "authorization state",
    {
        WaitTdlibParameters => "authorizationStateWaitTdlibParameters",
        WaitPhoneNumber => "authorizationStateWaitPhoneNumber",
        WaitPremiumPurchase => "authorizationStateWaitPremiumPurchase",
        WaitEmailAddress => "authorizationStateWaitEmailAddress",
        WaitEmailCode => "authorizationStateWaitEmailCode",
        WaitCode => "authorizationStateWaitCode",
        WaitOtherDeviceConfirmation => "authorizationStateWaitOtherDeviceConfirmation",
        WaitRegistration => "authorizationStateWaitRegistration",
        WaitPassword => "authorizationStateWaitPassword",
        Ready => "authorizationStateReady",
        LoggingOut => "authorizationStateLoggingOut",
        Closing => "authorizationStateClosing",
        Closed => "authorizationStateClosed",
    }
);

string_enum!(
    /// Проверяемое право администратора из `chatAdministratorRights`.
    ChatAdministratorRight,
    16,
    "chat administrator right",
    {
        CanManageChat => "can_manage_chat",
        CanChangeInfo => "can_change_info",
        CanPostMessages => "can_post_messages",
        CanEditMessages => "can_edit_messages",
        CanDeleteMessages => "can_delete_messages",
        CanInviteUsers => "can_invite_users",
        CanRestrictMembers => "can_restrict_members",
        CanPinMessages => "can_pin_messages",
        CanManageTopics => "can_manage_topics",
        CanPromoteMembers => "can_promote_members",
        CanManageVideoChats => "can_manage_video_chats",
        CanPostStories => "can_post_stories",
        CanEditStories => "can_edit_stories",
        CanDeleteStories => "can_delete_stories",
        CanManageDirectMessages => "can_manage_direct_messages",
        CanManageTags => "can_manage_tags",
    }
);

string_enum!(
    /// Проверяемое member permission из `chatPermissions`.
    ChatMemberRight,
    16,
    "chat member right",
    {
        CanSendBasicMessages => "can_send_basic_messages",
        CanSendAudios => "can_send_audios",
        CanSendDocuments => "can_send_documents",
        CanSendPhotos => "can_send_photos",
        CanSendVideos => "can_send_videos",
        CanSendVideoNotes => "can_send_video_notes",
        CanSendVoiceNotes => "can_send_voice_notes",
        CanSendPolls => "can_send_polls",
        CanSendOtherMessages => "can_send_other_messages",
        CanAddLinkPreviews => "can_add_link_previews",
        CanReactToMessages => "can_react_to_messages",
        CanEditTag => "can_edit_tag",
        CanChangeInfo => "can_change_info",
        CanInviteUsers => "can_invite_users",
        CanPinMessages => "can_pin_messages",
        CanCreateTopics => "can_create_topics",
    }
);

string_enum!(
    /// Проверяемое delegated право из `businessBotRights`.
    BusinessBotRight,
    14,
    "business bot right",
    {
        CanReply => "can_reply",
        CanReadMessages => "can_read_messages",
        CanDeleteSentMessages => "can_delete_sent_messages",
        CanDeleteAllMessages => "can_delete_all_messages",
        CanEditName => "can_edit_name",
        CanEditBio => "can_edit_bio",
        CanEditProfilePhoto => "can_edit_profile_photo",
        CanEditUsername => "can_edit_username",
        CanViewGiftsAndStars => "can_view_gifts_and_stars",
        CanSellGifts => "can_sell_gifts",
        CanChangeGiftSettings => "can_change_gift_settings",
        CanTransferAndUpgradeGifts => "can_transfer_and_upgrade_gifts",
        CanTransferStars => "can_transfer_stars",
        CanManageStories => "can_manage_stories",
    }
);

string_enum!(
    /// Action capability из exact `messageProperties.can_*` vocabulary.
    MessageCapability,
    36,
    "message capability",
    {
        CanAddOffer => "can_add_offer",
        CanAddTasks => "can_add_tasks",
        CanBeApproved => "can_be_approved",
        CanBeCopied => "can_be_copied",
        CanBeCopiedToSecretChat => "can_be_copied_to_secret_chat",
        CanBeDeclined => "can_be_declined",
        CanBeDeletedOnlyForSelf => "can_be_deleted_only_for_self",
        CanBeDeletedForAllUsers => "can_be_deleted_for_all_users",
        CanBeEdited => "can_be_edited",
        CanBeForwarded => "can_be_forwarded",
        CanBePaid => "can_be_paid",
        CanBePinned => "can_be_pinned",
        CanBeReplied => "can_be_replied",
        CanBeRepliedInAnotherChat => "can_be_replied_in_another_chat",
        CanBeSaved => "can_be_saved",
        CanBeSharedInStory => "can_be_shared_in_story",
        CanDeleteReactions => "can_delete_reactions",
        CanEditMedia => "can_edit_media",
        CanEditSchedulingState => "can_edit_scheduling_state",
        CanEditSuggestedPostInfo => "can_edit_suggested_post_info",
        CanGetAuthor => "can_get_author",
        CanGetEmbeddingCode => "can_get_embedding_code",
        CanGetLink => "can_get_link",
        CanGetMediaTimestampLinks => "can_get_media_timestamp_links",
        CanGetMessageThread => "can_get_message_thread",
        CanGetPollVoteStatistics => "can_get_poll_vote_statistics",
        CanGetReadDate => "can_get_read_date",
        CanGetStatistics => "can_get_statistics",
        CanGetVideoAdvertisements => "can_get_video_advertisements",
        CanGetViewers => "can_get_viewers",
        CanMarkTasksAsDone => "can_mark_tasks_as_done",
        CanRecognizeSpeech => "can_recognize_speech",
        CanReportChat => "can_report_chat",
        CanReportReactions => "can_report_reactions",
        CanReportSupergroupSpam => "can_report_supergroup_spam",
        CanSetFactCheck => "can_set_fact_check",
    }
);

string_enum!(
    /// Проверяемое Bool-поле exact `groupCall` runtime snapshot.
    ///
    /// Vocabulary содержит все `can_*` поля и ownership branch. Kind хранится
    /// отдельным типом, а lifecycle prerequisites принадлежат своему слою.
    GroupCallProperty,
    7,
    "group call property",
    {
        CanBeManaged => "can_be_managed",
        CanDeleteMessages => "can_delete_messages",
        CanEnableVideo => "can_enable_video",
        CanSendMessages => "can_send_messages",
        CanToggleAreMessagesAllowed => "can_toggle_are_messages_allowed",
        CanToggleMuteNewParticipants => "can_toggle_mute_new_participants",
        IsOwned => "is_owned",
    }
);

string_enum!(
    /// Взаимоисключающий resolved kind exact `groupCall` snapshot.
    ResolvedGroupCallKind,
    3,
    "resolved group call kind",
    {
        VideoChat => "video_chat",
        LiveStory => "live_story",
        Unbound => "unbound",
    }
);

string_enum!(
    /// Action capability из exact `groupCallMessage.can_*` vocabulary.
    GroupCallMessageCapability,
    1,
    "group call message capability",
    {
        CanBeDeleted => "can_be_deleted",
    }
);

string_enum!(
    /// `supergroupFullInfo` Bool-поле, на которое ссылается pinned method documentation.
    ///
    /// Это vocabulary method-level source family, а не все `can_*` поля
    /// constructor. Static atom не доказывает freshness snapshot: её обязан
    /// fail-closed проверять будущий runtime-слой.
    SupergroupFullInfoProperty,
    8,
    "supergroup full-info property",
    {
        CanEnablePaidMessages => "can_enable_paid_messages",
        CanGetMembers => "can_get_members",
        CanGetRevenueStatistics => "can_get_revenue_statistics",
        CanGetStarRevenueStatistics => "can_get_star_revenue_statistics",
        CanGetStatistics => "can_get_statistics",
        CanHideMembers => "can_hide_members",
        CanSetLocation => "can_set_location",
        CanToggleAggressiveAntiSpam => "can_toggle_aggressive_anti_spam",
    }
);

string_enum!(
    /// Bool option из pinned method documentation, допустимый в closed grammar.
    ///
    /// Само наличие в vocabulary не делает method complete: exact reviewed
    /// contract отдельно выбирает option и source, которые можно потребить.
    /// Это не произвольное имя из `getOption`.
    /// Static atom не доказывает текущее значение option: missing, wrong-typed
    /// или stale evidence обязан отклонять будущий runtime-слой.
    RuntimeBooleanOption,
    3,
    "runtime boolean option",
    {
        CanSetNewChatPrivacySettings => "can_set_new_chat_privacy_settings",
        CanUseTextEntitiesInStoryCaption => "can_use_text_entities_in_story_caption",
        CanWithdrawChatRevenue => "can_withdraw_chat_revenue",
    }
);

/// Ссылка на именованный argument TDLib method.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArgumentRef(String);

impl ArgumentRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for ArgumentRef {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut bytes = value.bytes();
        if value.is_empty()
            || value.len() > MAX_ARGUMENT_NAME_BYTES
            || !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
            || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InvalidIdentifier,
                format!("invalid bounded argument name {value:?}"),
            ));
        }
        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<&String> for ArgumentRef {
    type Error = CapabilityModelError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Семантическая цель chat-level runtime evidence.
///
/// TDLib использует два разных identifier space: `chat_id` адресует chat,
/// тогда как `supergroup_id` адресует underlying supergroup/channel object.
/// Одинаковый wire type `int53` не делает их взаимозаменяемыми.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChatTargetRef {
    kind: ChatTargetKind,
    argument: ArgumentRef,
}

impl ChatTargetRef {
    pub fn kind(&self) -> ChatTargetKind {
        self.kind
    }

    pub fn argument(&self) -> &ArgumentRef {
        &self.argument
    }
}

impl TryFrom<ArgumentRef> for ChatTargetRef {
    type Error = CapabilityModelError;

    fn try_from(argument: ArgumentRef) -> Result<Self, Self::Error> {
        let kind = match argument.as_str() {
            "chat_id" => ChatTargetKind::ChatId,
            "supergroup_id" => ChatTargetKind::SupergroupId,
            value => {
                return Err(CapabilityModelError::new(
                    CapabilityModelErrorKind::InvalidSemanticArgument,
                    format!("chat target must be named chat_id or supergroup_id, got {value:?}"),
                ));
            }
        };
        Ok(Self { kind, argument })
    }
}

impl TryFrom<&str> for ChatTargetRef {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(ArgumentRef::try_from(value)?)
    }
}

impl TryFrom<&String> for ChatTargetRef {
    type Error = CapabilityModelError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Exact scalar `message_id` argument role.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MessageIdRef(ArgumentRef);

impl MessageIdRef {
    pub fn argument(&self) -> &ArgumentRef {
        &self.0
    }
}

impl TryFrom<ArgumentRef> for MessageIdRef {
    type Error = CapabilityModelError;

    fn try_from(argument: ArgumentRef) -> Result<Self, Self::Error> {
        if argument.as_str() != "message_id" {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InvalidSemanticArgument,
                format!(
                    "single-message target must be named message_id, got {:?}",
                    argument.as_str()
                ),
            ));
        }
        Ok(Self(argument))
    }
}

impl TryFrom<&str> for MessageIdRef {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(ArgumentRef::try_from(value)?)
    }
}

/// Exact vector `message_ids` argument role.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MessageIdsRef(ArgumentRef);

impl MessageIdsRef {
    pub fn argument(&self) -> &ArgumentRef {
        &self.0
    }
}

impl TryFrom<ArgumentRef> for MessageIdsRef {
    type Error = CapabilityModelError;

    fn try_from(argument: ArgumentRef) -> Result<Self, Self::Error> {
        if argument.as_str() != "message_ids" {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InvalidSemanticArgument,
                format!(
                    "multi-message target must be named message_ids, got {:?}",
                    argument.as_str()
                ),
            ));
        }
        Ok(Self(argument))
    }
}

impl TryFrom<&str> for MessageIdsRef {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(ArgumentRef::try_from(value)?)
    }
}

/// Exact `group_call_id:int32` argument role.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GroupCallIdRef(ArgumentRef);

impl GroupCallIdRef {
    pub fn argument(&self) -> &ArgumentRef {
        &self.0
    }
}

impl TryFrom<ArgumentRef> for GroupCallIdRef {
    type Error = CapabilityModelError;

    fn try_from(argument: ArgumentRef) -> Result<Self, Self::Error> {
        if argument.as_str() != "group_call_id" {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InvalidSemanticArgument,
                format!(
                    "group-call target must be named group_call_id, got {:?}",
                    argument.as_str()
                ),
            ));
        }
        Ok(Self(argument))
    }
}

impl TryFrom<&str> for GroupCallIdRef {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(ArgumentRef::try_from(value)?)
    }
}

/// Exact `message_ids:vector<int32>` argument role for group-call messages.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GroupCallMessageIdsRef(ArgumentRef);

impl GroupCallMessageIdsRef {
    pub fn argument(&self) -> &ArgumentRef {
        &self.0
    }
}

impl TryFrom<ArgumentRef> for GroupCallMessageIdsRef {
    type Error = CapabilityModelError;

    fn try_from(argument: ArgumentRef) -> Result<Self, Self::Error> {
        if argument.as_str() != "message_ids" {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InvalidSemanticArgument,
                format!(
                    "group-call message collection must be named message_ids, got {:?}",
                    argument.as_str()
                ),
            ));
        }
        Ok(Self(argument))
    }
}

impl TryFrom<&str> for GroupCallMessageIdsRef {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(ArgumentRef::try_from(value)?)
    }
}

/// Group-call-message evidence с явной universal cardinality.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GroupCallMessageSubjectRef {
    Each {
        group_call: GroupCallIdRef,
        messages: GroupCallMessageIdsRef,
    },
}

impl GroupCallMessageSubjectRef {
    pub fn group_call(&self) -> &GroupCallIdRef {
        match self {
            Self::Each { group_call, .. } => group_call,
        }
    }

    pub fn message_argument(&self) -> &ArgumentRef {
        match self {
            Self::Each { messages, .. } => messages.argument(),
        }
    }
}

/// Typed condition на resolved group-call kind.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GroupCallKindCondition {
    group_call: GroupCallIdRef,
    kind: ResolvedGroupCallKind,
}

impl GroupCallKindCondition {
    pub fn new(group_call: GroupCallIdRef, kind: ResolvedGroupCallKind) -> Self {
        Self { group_call, kind }
    }

    pub fn group_call(&self) -> &GroupCallIdRef {
        &self.group_call
    }

    pub fn kind(&self) -> ResolvedGroupCallKind {
        self.kind
    }
}

/// Message-property evidence target with explicit scalar/universal cardinality.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MessageSubjectRef {
    One {
        chat: ChatTargetRef,
        message: MessageIdRef,
    },
    Each {
        chat: ChatTargetRef,
        messages: MessageIdsRef,
    },
}

impl MessageSubjectRef {
    pub fn chat(&self) -> &ChatTargetRef {
        match self {
            Self::One { chat, .. } | Self::Each { chat, .. } => chat,
        }
    }

    pub fn message_argument(&self) -> &ArgumentRef {
        match self {
            Self::One { message, .. } => message.argument(),
            Self::Each { messages, .. } => messages.argument(),
        }
    }
}

/// Условие на resolved chat kind с сохранением identifier space target.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChatKindCondition {
    target: ChatTargetRef,
    kind: ResolvedChatKind,
}

impl ChatKindCondition {
    pub fn try_new(
        target: ChatTargetRef,
        kind: ResolvedChatKind,
    ) -> Result<Self, CapabilityModelError> {
        if target.kind() == ChatTargetKind::SupergroupId
            && !matches!(
                kind,
                ResolvedChatKind::Supergroup | ResolvedChatKind::Channel
            )
        {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::IncompatibleChatKindTarget,
                format!(
                    "supergroup_id can resolve only to supergroup or channel, not {}",
                    kind.as_str()
                ),
            ));
        }
        Ok(Self { target, kind })
    }

    pub fn target(&self) -> &ChatTargetRef {
        &self.target
    }

    pub fn kind(&self) -> ResolvedChatKind {
        self.kind
    }
}

/// Exact `forum_topic_id` argument role.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ForumTopicRef(ArgumentRef);

impl ForumTopicRef {
    pub fn argument(&self) -> &ArgumentRef {
        &self.0
    }
}

impl TryFrom<ArgumentRef> for ForumTopicRef {
    type Error = CapabilityModelError;

    fn try_from(argument: ArgumentRef) -> Result<Self, Self::Error> {
        if argument.as_str() != "forum_topic_id" {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InvalidSemanticArgument,
                format!(
                    "forum topic target must be named forum_topic_id, got {:?}",
                    argument.as_str()
                ),
            ));
        }
        Ok(Self(argument))
    }
}

impl TryFrom<&str> for ForumTopicRef {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(ArgumentRef::try_from(value)?)
    }
}

/// Exact business-connection argument role used by TDLib methods.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BusinessConnectionRef(ArgumentRef);

impl BusinessConnectionRef {
    pub fn argument(&self) -> &ArgumentRef {
        &self.0
    }
}

impl TryFrom<ArgumentRef> for BusinessConnectionRef {
    type Error = CapabilityModelError;

    fn try_from(argument: ArgumentRef) -> Result<Self, Self::Error> {
        if argument.as_str() != "business_connection_id" {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InvalidSemanticArgument,
                format!(
                    "business connection runtime evidence must be named business_connection_id, got {:?}",
                    argument.as_str()
                ),
            ));
        }
        Ok(Self(argument))
    }
}

impl TryFrom<&str> for BusinessConnectionRef {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(ArgumentRef::try_from(value)?)
    }
}

/// Ограниченный exact string literal для condition на method parameter.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParameterStringValue(String);

impl ParameterStringValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for ParameterStringValue {
    type Error = CapabilityModelError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        const MAX_VALUE_BYTES: usize = 256;
        if value.is_empty() || value.len() > MAX_VALUE_BYTES || value.chars().any(char::is_control)
        {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InvalidParameterValue,
                format!("invalid bounded string parameter value {value:?}"),
            ));
        }
        Ok(Self(value.to_owned()))
    }
}

/// Дополнительный путь через `td_json_client_execute`.
///
/// Обычный client request остаётся доступен независимо от этого поля. Условный
/// вариант нужен, например, для `getOption(name)`, который synchronous только
/// при `name = version | commit_hash`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SynchronousCapability {
    Never,
    Always,
    StringParameterValues(SynchronousStringValues),
}

impl SynchronousCapability {
    pub fn for_string_values(
        parameter: ArgumentRef,
        mut values: Vec<ParameterStringValue>,
    ) -> Result<Self, CapabilityModelError> {
        if values.is_empty() {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::EmptySet,
                "conditional synchronous capability needs at least one value",
            ));
        }
        if values.len() > MAX_SYNCHRONOUS_VALUES_PER_METHOD {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::ResourceLimit,
                format!(
                    "conditional synchronous capability exceeds the {MAX_SYNCHRONOUS_VALUES_PER_METHOD}-value cap"
                ),
            ));
        }
        canonicalize_unique(&mut values, "synchronous parameter value")?;
        Ok(Self::StringParameterValues(SynchronousStringValues {
            parameter,
            values,
        }))
    }
}

/// Canonical non-empty value condition для synchronous execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynchronousStringValues {
    parameter: ArgumentRef,
    values: Vec<ParameterStringValue>,
}

impl SynchronousStringValues {
    pub fn parameter(&self) -> &ArgumentRef {
        &self.parameter
    }

    pub fn values(&self) -> &[ParameterStringValue] {
        &self.values
    }
}

/// Runtime evidence, которое capability evaluator обязан получить отдельно.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RuntimeRequirement {
    ChatKind(ChatKindCondition),
    ChatAdministrator {
        target: ChatTargetRef,
    },
    ChatAdministratorRight {
        target: ChatTargetRef,
        right: ChatAdministratorRight,
    },
    ChatMemberRight {
        target: ChatTargetRef,
        right: ChatMemberRight,
    },
    ChatOwner {
        target: ChatTargetRef,
    },
    TopicCreator {
        target: ChatTargetRef,
        topic: ForumTopicRef,
    },
    BusinessConnectionEnabled {
        connection: BusinessConnectionRef,
    },
    BusinessConnectionRight {
        connection: BusinessConnectionRef,
        right: BusinessBotRight,
    },
    MessageCapability {
        subject: MessageSubjectRef,
        capability: MessageCapability,
    },
    GroupCallKind(GroupCallKindCondition),
    GroupCallProperty {
        group_call: GroupCallIdRef,
        property: GroupCallProperty,
    },
    GroupCallMessageCapability {
        subject: GroupCallMessageSubjectRef,
        capability: GroupCallMessageCapability,
    },
    SupergroupFullInfoProperty {
        target: ChatTargetRef,
        property: SupergroupFullInfoProperty,
    },
    BooleanOptionEnabled {
        option: RuntimeBooleanOption,
    },
}

impl RuntimeRequirement {
    pub fn argument_refs(&self) -> Vec<&ArgumentRef> {
        match self {
            Self::ChatKind(condition) => vec![condition.target().argument()],
            Self::ChatAdministrator { target }
            | Self::ChatAdministratorRight { target, .. }
            | Self::ChatMemberRight { target, .. }
            | Self::ChatOwner { target } => vec![target.argument()],
            Self::BusinessConnectionEnabled { connection }
            | Self::BusinessConnectionRight { connection, .. } => vec![connection.argument()],
            Self::MessageCapability { subject, .. } => {
                vec![subject.chat().argument(), subject.message_argument()]
            }
            Self::GroupCallProperty { group_call, .. } => vec![group_call.argument()],
            Self::GroupCallKind(condition) => vec![condition.group_call().argument()],
            Self::GroupCallMessageCapability { subject, .. } => {
                vec![subject.group_call().argument(), subject.message_argument()]
            }
            Self::SupergroupFullInfoProperty { target, .. } => vec![target.argument()],
            Self::BooleanOptionEnabled { .. } => Vec::new(),
            Self::TopicCreator { target, topic } => {
                vec![target.argument(), topic.argument()]
            }
        }
    }

    fn supports_account(&self, account: AccountKind) -> bool {
        !matches!(
            (account, self),
            (
                AccountKind::RegularUser,
                Self::BusinessConnectionEnabled { .. } | Self::BusinessConnectionRight { .. }
            ) | (AccountKind::Bot, Self::ChatOwner { .. })
                | (
                    AccountKind::Bot,
                    Self::GroupCallKind(_)
                        | Self::GroupCallProperty { .. }
                        | Self::GroupCallMessageCapability { .. }
                )
        )
    }
}

/// Исполнимые alternatives: OR из clauses, внутри clause — AND.
///
/// Пустой набор создаётся только через [`Self::always`] и означает отсутствие
/// runtime gate. `try_new` принимает только непустую DNF, чтобы `[]` не имел
/// одновременно значения "always" и malformed policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementAlternatives {
    clauses: Vec<Vec<RuntimeRequirement>>,
}

impl RequirementAlternatives {
    pub fn always() -> Self {
        Self {
            clauses: Vec::new(),
        }
    }

    pub fn try_new(
        mut clauses: Vec<Vec<RuntimeRequirement>>,
    ) -> Result<Self, CapabilityModelError> {
        if clauses.is_empty() {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::EmptySet,
                "use RequirementAlternatives::always() for an unconditional method",
            ));
        }
        if clauses.len() > MAX_CLAUSES_PER_METHOD {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::ResourceLimit,
                format!("runtime alternatives exceed the {MAX_CLAUSES_PER_METHOD}-clause cap"),
            ));
        }
        let atom_count = clauses
            .iter()
            .fold(0usize, |count, clause| count.saturating_add(clause.len()));
        if atom_count > MAX_ATOMS_PER_METHOD {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::ResourceLimit,
                format!("runtime alternatives exceed the {MAX_ATOMS_PER_METHOD}-atom cap"),
            ));
        }
        for clause in &mut clauses {
            if clause.is_empty() {
                return Err(CapabilityModelError::new(
                    CapabilityModelErrorKind::EmptyClause,
                    "runtime requirement clause must not be empty",
                ));
            }
            canonicalize_unique(clause, "runtime requirement")?;
            let contradictory_chat_kind = clause.iter().enumerate().any(|(left_index, left)| {
                let RuntimeRequirement::ChatKind(left) = left else {
                    return false;
                };
                clause.iter().skip(left_index + 1).any(|right| {
                    matches!(
                        right,
                        RuntimeRequirement::ChatKind(right)
                            if left.target() == right.target() && left.kind() != right.kind()
                    )
                })
            });
            let contradictory_group_call_kind =
                clause.iter().enumerate().any(|(left_index, left)| {
                    let RuntimeRequirement::GroupCallKind(left) = left else {
                        return false;
                    };
                    clause.iter().skip(left_index + 1).any(|right| {
                        matches!(
                            right,
                            RuntimeRequirement::GroupCallKind(right)
                                if left.group_call() == right.group_call()
                                    && left.kind() != right.kind()
                        )
                    })
                });
            if contradictory_chat_kind || contradictory_group_call_kind {
                return Err(CapabilityModelError::new(
                    CapabilityModelErrorKind::ContradictoryClause,
                    "runtime requirement clause assigns multiple kinds to one target",
                ));
            }
        }
        canonicalize_unique(&mut clauses, "runtime requirement clause")?;
        if clauses.iter().enumerate().any(|(left_index, left)| {
            clauses.iter().enumerate().any(|(right_index, right)| {
                left_index != right_index
                    && left.len() < right.len()
                    && left
                        .iter()
                        .all(|requirement| right.binary_search(requirement).is_ok())
            })
        }) {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::RedundantClause,
                "runtime alternatives contain a clause absorbed by a less restrictive clause",
            ));
        }
        Ok(Self { clauses })
    }

    pub fn clauses(&self) -> &[Vec<RuntimeRequirement>] {
        &self.clauses
    }

    pub fn is_always(&self) -> bool {
        self.clauses.is_empty()
    }

    fn supports_account(&self, account: AccountKind) -> bool {
        self.is_always()
            || self.clauses.iter().any(|clause| {
                clause
                    .iter()
                    .all(|requirement| requirement.supports_account(account))
            })
    }
}

/// Ось ограничения отдельных значений parameter.
///
/// Это классификация для будущей проверки конкретного значения, а не утверждение,
/// что весь method недоступен без указанной capability.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ParameterGate {
    Account(AccountKind),
    CurrentAccountEntitlement(CurrentAccountEntitlement),
    Application(ApplicationRequirement),
    DcEnvironment(DcEnvironment),
}

/// Method parameter содержит отдельные gated значения, но method целиком не gated.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ParameterCapabilityNotice {
    parameter: ArgumentRef,
    gate: ParameterGate,
}

impl ParameterCapabilityNotice {
    pub fn try_new(
        parameter: ArgumentRef,
        gate: ParameterGate,
    ) -> Result<Self, CapabilityModelError> {
        if gate == ParameterGate::Application(ApplicationRequirement::Any) {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::VacuousParameterGate,
                "parameter-level application gate must be official or official_mobile",
            ));
        }
        Ok(Self { parameter, gate })
    }

    pub fn parameter(&self) -> &ArgumentRef {
        &self.parameter
    }

    pub fn gate(&self) -> ParameterGate {
        self.gate
    }
}

/// Canonical static capability descriptor одного exact method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityDescriptor {
    synchronous: SynchronousCapability,
    ready_accounts: Vec<AccountKind>,
    authorization_states: Vec<AuthorizationState>,
    current_account_entitlements: Vec<CurrentAccountEntitlement>,
    application: ApplicationRequirement,
    dc_environments: Vec<DcEnvironment>,
    runtime_requirements: RequirementAlternatives,
    parameter_notices: Vec<ParameterCapabilityNotice>,
}

impl CapabilityDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        synchronous: SynchronousCapability,
        mut ready_accounts: Vec<AccountKind>,
        mut authorization_states: Vec<AuthorizationState>,
        mut current_account_entitlements: Vec<CurrentAccountEntitlement>,
        application: ApplicationRequirement,
        mut dc_environments: Vec<DcEnvironment>,
        runtime_requirements: RequirementAlternatives,
        mut parameter_notices: Vec<ParameterCapabilityNotice>,
    ) -> Result<Self, CapabilityModelError> {
        for (count, cap, name) in [
            (
                ready_accounts.len(),
                AccountKind::ALL.len(),
                "ready account",
            ),
            (
                authorization_states.len(),
                AuthorizationState::ALL.len(),
                "authorization state",
            ),
            (
                current_account_entitlements.len(),
                CurrentAccountEntitlement::ALL.len(),
                "current-account entitlement",
            ),
            (
                dc_environments.len(),
                DcEnvironment::ALL.len(),
                "DC environment",
            ),
        ] {
            if count > cap {
                return Err(CapabilityModelError::new(
                    CapabilityModelErrorKind::ResourceLimit,
                    format!("{name} set exceeds its closed {cap}-value vocabulary"),
                ));
            }
        }
        if parameter_notices.len() > MAX_PARAMETER_NOTICES_PER_METHOD {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::ResourceLimit,
                format!(
                    "parameter notices exceed the {MAX_PARAMETER_NOTICES_PER_METHOD}-notice cap"
                ),
            ));
        }
        canonicalize_unique(&mut ready_accounts, "ready account")?;
        canonicalize_unique(&mut authorization_states, "authorization state")?;
        canonicalize_unique(
            &mut current_account_entitlements,
            "current-account entitlement",
        )?;
        canonicalize_nonempty_unique(&mut dc_environments, "DC environment")?;
        canonicalize_unique(&mut parameter_notices, "parameter capability notice")?;

        if authorization_states.is_empty() {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::EmptySet,
                "client invocation needs at least one authorization state",
            ));
        }
        if authorization_states.iter().any(|state| {
            matches!(
                state,
                AuthorizationState::LoggingOut
                    | AuthorizationState::Closing
                    | AuthorizationState::Closed
            )
        }) {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::TerminalAuthorizationState,
                "terminal authorization states are lifecycle evidence, not method request capability",
            ));
        }

        let allows_ready = authorization_states.contains(&AuthorizationState::Ready);
        if allows_ready == ready_accounts.is_empty() {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::InconsistentReadyAccounts,
                "ready_accounts must be non-empty exactly when authorizationStateReady is allowed",
            ));
        }
        if !current_account_entitlements.is_empty()
            && ready_accounts.as_slice() != [AccountKind::RegularUser]
        {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::IncompatibleEntitlement,
                "current-account Premium/Business requires exactly regular_user at Ready",
            ));
        }
        if !runtime_requirements.is_always() && !allows_ready {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::RuntimeRequirementBeforeReady,
                "runtime account evidence is only meaningful at authorizationStateReady",
            ));
        }
        if ready_accounts
            .iter()
            .any(|account| !runtime_requirements.supports_account(*account))
        {
            return Err(CapabilityModelError::new(
                CapabilityModelErrorKind::IncompatibleRuntimeRequirement,
                "runtime alternatives are unsatisfiable for an allowed account kind",
            ));
        }
        for notice in &parameter_notices {
            validate_parameter_gate(
                notice.gate(),
                &ready_accounts,
                &current_account_entitlements,
                application,
                &dc_environments,
            )?;
        }

        Ok(Self {
            synchronous,
            ready_accounts,
            authorization_states,
            current_account_entitlements,
            application,
            dc_environments,
            runtime_requirements,
            parameter_notices,
        })
    }

    pub fn synchronous(&self) -> &SynchronousCapability {
        &self.synchronous
    }

    pub fn ready_accounts(&self) -> &[AccountKind] {
        &self.ready_accounts
    }

    pub fn authorization_states(&self) -> &[AuthorizationState] {
        &self.authorization_states
    }

    pub fn current_account_entitlements(&self) -> &[CurrentAccountEntitlement] {
        &self.current_account_entitlements
    }

    pub fn application(&self) -> ApplicationRequirement {
        self.application
    }

    pub fn dc_environments(&self) -> &[DcEnvironment] {
        &self.dc_environments
    }

    pub fn runtime_requirements(&self) -> &RequirementAlternatives {
        &self.runtime_requirements
    }

    pub fn parameter_notices(&self) -> &[ParameterCapabilityNotice] {
        &self.parameter_notices
    }
}

fn validate_parameter_gate(
    gate: ParameterGate,
    ready_accounts: &[AccountKind],
    entitlements: &[CurrentAccountEntitlement],
    application: ApplicationRequirement,
    dc_environments: &[DcEnvironment],
) -> Result<(), CapabilityModelError> {
    let (reachable, already_required) = match gate {
        ParameterGate::Account(account) => (
            ready_accounts.contains(&account),
            ready_accounts == [account],
        ),
        ParameterGate::CurrentAccountEntitlement(entitlement) => (
            ready_accounts.contains(&AccountKind::RegularUser),
            entitlements.contains(&entitlement),
        ),
        ParameterGate::Application(required) => {
            let reachable = match application {
                ApplicationRequirement::Any => true,
                ApplicationRequirement::Official => matches!(
                    required,
                    ApplicationRequirement::Official | ApplicationRequirement::OfficialMobile
                ),
                ApplicationRequirement::OfficialMobile => matches!(
                    required,
                    ApplicationRequirement::Official | ApplicationRequirement::OfficialMobile
                ),
            };
            let already_required = application == required
                || (application == ApplicationRequirement::OfficialMobile
                    && required == ApplicationRequirement::Official);
            (reachable, already_required)
        }
        ParameterGate::DcEnvironment(environment) => (
            dc_environments.contains(&environment),
            dc_environments == [environment],
        ),
    };
    if !reachable {
        return Err(CapabilityModelError::new(
            CapabilityModelErrorKind::IncompatibleParameterGate,
            "parameter gate is unreachable inside the method-level capability",
        ));
    }
    if already_required {
        return Err(CapabilityModelError::new(
            CapabilityModelErrorKind::RedundantParameterGate,
            "parameter gate is already required by the whole method",
        ));
    }
    Ok(())
}

fn canonicalize_nonempty_unique<T: Ord>(
    values: &mut [T],
    name: &str,
) -> Result<(), CapabilityModelError> {
    if values.is_empty() {
        return Err(CapabilityModelError::new(
            CapabilityModelErrorKind::EmptySet,
            format!("{name} set must not be empty"),
        ));
    }
    canonicalize_unique(values, name)
}

fn canonicalize_unique<T: Ord>(values: &mut [T], name: &str) -> Result<(), CapabilityModelError> {
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CapabilityModelError::new(
            CapabilityModelErrorKind::DuplicateValue,
            format!("duplicate {name}"),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityModelErrorKind {
    ResourceLimit,
    EmptySet,
    EmptyClause,
    DuplicateValue,
    InvalidIdentifier,
    InvalidSemanticArgument,
    IncompatibleChatKindTarget,
    InconsistentReadyAccounts,
    IncompatibleEntitlement,
    IncompatibleRuntimeRequirement,
    RuntimeRequirementBeforeReady,
    VacuousParameterGate,
    InvalidParameterValue,
    RedundantClause,
    ContradictoryClause,
    IncompatibleParameterGate,
    RedundantParameterGate,
    TerminalAuthorizationState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityModelError {
    kind: CapabilityModelErrorKind,
    detail: String,
}

impl CapabilityModelError {
    fn new(kind: CapabilityModelErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    pub fn kind(&self) -> CapabilityModelErrorKind {
        self.kind
    }
}

impl fmt::Display for CapabilityModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for CapabilityModelError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseCapabilityValueError {
    kind: &'static str,
    value: String,
}

impl ParseCapabilityValueError {
    fn new(kind: &'static str, value: &str) -> Self {
        Self {
            kind,
            value: value.to_owned(),
        }
    }
}

impl fmt::Display for ParseCapabilityValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown {} {:?}", self.kind, self.value)
    }
}

impl Error for ParseCapabilityValueError {}

#[cfg(test)]
mod tests;
