//! Strict daemon adapters for chat workflow inputs.

use serde::Deserialize;
use telegram_core::reducer::ChatList;
use telegram_core::workflows::{ChatTarget, MembershipTarget};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProfileNameInput {
    pub(super) first_name: String,
    pub(super) last_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChatTitleInput {
    pub(super) chat_id: i64,
    pub(super) title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LeaveChatInput {
    pub(super) chat_id: i64,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum TargetInput {
    Id { chat_id: i64 },
    PublicUsername { username: String },
    PublicLink { url: String },
}

impl TargetInput {
    pub(super) fn target(&self) -> ChatTarget<'_> {
        match self {
            Self::Id { chat_id } => ChatTarget::Id(*chat_id),
            Self::PublicUsername { username } => ChatTarget::PublicUsername(username),
            Self::PublicLink { url } => ChatTarget::PublicLink(url),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InvitePreviewInput {
    pub(super) url: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum MembershipInput {
    ChatId { chat_id: i64 },
    InviteLink { url: String },
}

impl MembershipInput {
    pub(super) fn target(&self) -> MembershipTarget<'_> {
        match self {
            Self::ChatId { chat_id } => MembershipTarget::ChatId(*chat_id),
            Self::InviteLink { url } => MembershipTarget::InviteLink(url),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChatListInput {
    pub(super) list: ChatListKind,
    pub(super) limit: i32,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ChatListKind {
    Main,
    Archive,
    Folder { folder_id: i32 },
}

impl From<ChatListKind> for ChatList {
    fn from(value: ChatListKind) -> Self {
        match value {
            ChatListKind::Main => Self::Main,
            ChatListKind::Archive => Self::Archive,
            ChatListKind::Folder { folder_id } => Self::Folder(folder_id),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InspectInput {
    pub(super) target: TargetInput,
    pub(super) open: bool,
}
