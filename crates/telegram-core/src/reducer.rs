//! Последовательное применение TDLib updates к core caches.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UpdateSequence(u64);

impl UpdateSequence {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionedValue {
    pub sequence: UpdateSequence,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedUpdateKind {
    Authorization,
    User,
    UserFullInfo,
    Chat,
    BasicGroup,
    BasicGroupFullInfo,
    Supergroup,
    SupergroupFullInfo,
    File,
    Connection,
    MessageSend,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedUpdate {
    pub sequence: UpdateSequence,
    pub kind: CachedUpdateKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageSendKey {
    pub chat_id: i64,
    pub old_message_id: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageSendState {
    Acknowledged,
    Succeeded { message: Value },
    Failed { message: Value, error: Value },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionedMessageSendState {
    pub sequence: UpdateSequence,
    pub state: MessageSendState,
}

#[derive(Debug, Default)]
pub struct StateReducer {
    sequence: u64,
    authorization: Option<VersionedValue>,
    users: BTreeMap<i64, VersionedValue>,
    user_full_info: BTreeMap<i64, VersionedValue>,
    chats: BTreeMap<i64, VersionedValue>,
    chat_online_member_counts: BTreeMap<i64, (UpdateSequence, i32)>,
    basic_groups: BTreeMap<i64, VersionedValue>,
    basic_group_full_info: BTreeMap<i64, VersionedValue>,
    supergroups: BTreeMap<i64, VersionedValue>,
    supergroup_full_info: BTreeMap<i64, VersionedValue>,
    files: BTreeMap<i32, VersionedValue>,
    connection: Option<VersionedValue>,
    message_sends: BTreeMap<MessageSendKey, VersionedMessageSendState>,
}

impl StateReducer {
    pub fn last_sequence(&self) -> Option<UpdateSequence> {
        (self.sequence != 0).then_some(UpdateSequence(self.sequence))
    }

    pub fn authorization(&self) -> Option<&VersionedValue> {
        self.authorization.as_ref()
    }

    pub fn user(&self, user_id: i64) -> Option<&VersionedValue> {
        self.users.get(&user_id)
    }

    pub fn user_full_info(&self, user_id: i64) -> Option<&VersionedValue> {
        self.user_full_info.get(&user_id)
    }

    pub fn chat(&self, chat_id: i64) -> Option<&VersionedValue> {
        self.chats.get(&chat_id)
    }

    pub fn chat_online_member_count(&self, chat_id: i64) -> Option<(UpdateSequence, i32)> {
        self.chat_online_member_counts.get(&chat_id).copied()
    }

    pub fn basic_group(&self, group_id: i64) -> Option<&VersionedValue> {
        self.basic_groups.get(&group_id)
    }

    pub fn basic_group_full_info(&self, group_id: i64) -> Option<&VersionedValue> {
        self.basic_group_full_info.get(&group_id)
    }

    pub fn supergroup(&self, group_id: i64) -> Option<&VersionedValue> {
        self.supergroups.get(&group_id)
    }

    pub fn supergroup_full_info(&self, group_id: i64) -> Option<&VersionedValue> {
        self.supergroup_full_info.get(&group_id)
    }

    pub fn file(&self, file_id: i32) -> Option<&VersionedValue> {
        self.files.get(&file_id)
    }

    pub fn connection(&self) -> Option<&VersionedValue> {
        self.connection.as_ref()
    }

    pub fn message_send(&self, key: MessageSendKey) -> Option<&VersionedMessageSendState> {
        self.message_sends.get(&key)
    }

    pub fn apply_transport_event(
        &mut self,
        event: &crate::transport::TdJsonEvent,
    ) -> Result<Option<AppliedUpdate>, ReducerError> {
        match event {
            crate::transport::TdJsonEvent::Update(update) => self.apply(update).map(Some),
            crate::transport::TdJsonEvent::UnmatchedResponse { .. }
            | crate::transport::TdJsonEvent::Fatal(_) => Ok(None),
        }
    }

    pub fn apply(&mut self, update: &Value) -> Result<AppliedUpdate, ReducerError> {
        let object = object(update, "update")?;
        let update_type = string(object, "@type")?;
        let sequence = UpdateSequence(
            self.sequence
                .checked_add(1)
                .ok_or(ReducerError::SequenceExhausted)?,
        );

        let kind = match update_type {
            "updateAuthorizationState" => self.apply_authorization(object, sequence)?,
            "updateUser" => {
                self.apply_entity(object, "user", "id", sequence, CachedUpdateKind::User)?
            }
            "updateUserStatus" => {
                self.patch_entity("user", object, "user_id", sequence, &["status"])?;
                CachedUpdateKind::User
            }
            "updateUserFullInfo" => self.apply_keyed_value(
                object,
                "user_id",
                "user_full_info",
                sequence,
                CachedUpdateKind::UserFullInfo,
            )?,
            "updateNewChat" => {
                self.apply_entity(object, "chat", "id", sequence, CachedUpdateKind::Chat)?
            }
            "updateChatPosition" => self.apply_chat_position(object, sequence)?,
            "updateChatAddedToList" => self.apply_chat_list(object, sequence, true)?,
            "updateChatRemovedFromList" => self.apply_chat_list(object, sequence, false)?,
            "updateChatReplyMarkup" => self.apply_chat_reply_markup(object, sequence)?,
            "updateChatOnlineMemberCount" => {
                let chat_id = integer(object, "chat_id")?;
                let count = integer32(object, "online_member_count")?;
                self.require_chat(chat_id)?;
                self.chat_online_member_counts
                    .insert(chat_id, (sequence, count));
                CachedUpdateKind::Chat
            }
            update_type if chat_direct_fields(update_type).is_some() => {
                self.patch_entity(
                    "chat",
                    object,
                    "chat_id",
                    sequence,
                    chat_direct_fields(update_type).unwrap_or_default(),
                )?;
                CachedUpdateKind::Chat
            }
            "updateBasicGroup" => self.apply_entity(
                object,
                "basic_group",
                "id",
                sequence,
                CachedUpdateKind::BasicGroup,
            )?,
            "updateBasicGroupFullInfo" => self.apply_keyed_value(
                object,
                "basic_group_id",
                "basic_group_full_info",
                sequence,
                CachedUpdateKind::BasicGroupFullInfo,
            )?,
            "updateSupergroup" => self.apply_entity(
                object,
                "supergroup",
                "id",
                sequence,
                CachedUpdateKind::Supergroup,
            )?,
            "updateSupergroupFullInfo" => self.apply_keyed_value(
                object,
                "supergroup_id",
                "supergroup_full_info",
                sequence,
                CachedUpdateKind::SupergroupFullInfo,
            )?,
            "updateFile" => self.apply_file(object, sequence)?,
            "updateConnectionState" => {
                let state = required_value(object, "state")?.clone();
                object_ref(&state, "state")?;
                self.connection = Some(VersionedValue {
                    sequence,
                    value: state,
                });
                CachedUpdateKind::Connection
            }
            "updateMessageSendAcknowledged" => self.apply_message_acknowledged(object, sequence)?,
            "updateMessageSendSucceeded" => self.apply_message_terminal(object, sequence, true)?,
            "updateMessageSendFailed" => self.apply_message_terminal(object, sequence, false)?,
            _ => CachedUpdateKind::Unknown,
        };

        self.sequence = sequence.0;
        Ok(AppliedUpdate { sequence, kind })
    }

    fn apply_authorization(
        &mut self,
        update: &Map<String, Value>,
        sequence: UpdateSequence,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let state = required_value(update, "authorization_state")?.clone();
        crate::authorization::parse_authorization_state(&state)
            .map_err(|_| ReducerError::InvalidField("authorization_state"))?;
        self.authorization = Some(VersionedValue {
            sequence,
            value: state,
        });
        Ok(CachedUpdateKind::Authorization)
    }

    fn apply_entity(
        &mut self,
        update: &Map<String, Value>,
        field: &'static str,
        id_field: &'static str,
        sequence: UpdateSequence,
        kind: CachedUpdateKind,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let value = required_value(update, field)?.clone();
        let value_object = object_ref(&value, field)?;
        let expected_type = match field {
            "user" => "user",
            "chat" => "chat",
            "basic_group" => "basicGroup",
            "supergroup" => "supergroup",
            _ => return Err(ReducerError::InvalidField(field)),
        };
        require_type(value_object, expected_type, field)?;
        let id = integer(value_object, id_field)?;
        match kind {
            CachedUpdateKind::User => self.users.insert(id, VersionedValue { sequence, value }),
            CachedUpdateKind::Chat => self.chats.insert(id, VersionedValue { sequence, value }),
            CachedUpdateKind::BasicGroup => self
                .basic_groups
                .insert(id, VersionedValue { sequence, value }),
            CachedUpdateKind::Supergroup => self
                .supergroups
                .insert(id, VersionedValue { sequence, value }),
            _ => return Err(ReducerError::InvalidField(field)),
        };
        Ok(kind)
    }

    fn apply_keyed_value(
        &mut self,
        update: &Map<String, Value>,
        id_field: &'static str,
        value_field: &'static str,
        sequence: UpdateSequence,
        kind: CachedUpdateKind,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let id = integer(update, id_field)?;
        let value = required_value(update, value_field)?.clone();
        let value_object = object_ref(&value, value_field)?;
        let expected_type = match value_field {
            "user_full_info" => "userFullInfo",
            "basic_group_full_info" => "basicGroupFullInfo",
            "supergroup_full_info" => "supergroupFullInfo",
            _ => return Err(ReducerError::InvalidField(value_field)),
        };
        require_type(value_object, expected_type, value_field)?;
        match kind {
            CachedUpdateKind::UserFullInfo => self
                .user_full_info
                .insert(id, VersionedValue { sequence, value }),
            CachedUpdateKind::BasicGroupFullInfo => self
                .basic_group_full_info
                .insert(id, VersionedValue { sequence, value }),
            CachedUpdateKind::SupergroupFullInfo => self
                .supergroup_full_info
                .insert(id, VersionedValue { sequence, value }),
            _ => return Err(ReducerError::InvalidField(value_field)),
        };
        Ok(kind)
    }

    fn patch_entity(
        &mut self,
        entity: &'static str,
        update: &Map<String, Value>,
        id_field: &'static str,
        sequence: UpdateSequence,
        fields: &[&'static str],
    ) -> Result<(), ReducerError> {
        let id = integer(update, id_field)?;
        let cached = match entity {
            "user" => self.users.get_mut(&id),
            "chat" => self.chats.get_mut(&id),
            _ => None,
        }
        .ok_or(ReducerError::MissingBaseEntity { entity, id })?;
        let target = cached
            .value
            .as_object_mut()
            .ok_or(ReducerError::InvalidField(entity))?;
        let patches = fields
            .iter()
            .map(|field| Ok((*field, required_value(update, field)?.clone())))
            .collect::<Result<Vec<_>, ReducerError>>()?;
        for (field, value) in patches {
            target.insert(field.to_owned(), value);
        }
        cached.sequence = sequence;
        Ok(())
    }

    fn apply_chat_position(
        &mut self,
        update: &Map<String, Value>,
        sequence: UpdateSequence,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let chat_id = integer(update, "chat_id")?;
        let position = required_value(update, "position")?.clone();
        let position_object = object_ref(&position, "position")?;
        let list = required_value(position_object, "list")?.clone();
        let order = integer(position_object, "order")?;
        {
            let positions = self.chat_array_mut(chat_id, "positions")?;
            positions.retain(|known| known.get("list") != Some(&list));
            if order != 0 {
                positions.push(position);
            }
        }
        self.require_chat_mut(chat_id)?.sequence = sequence;
        Ok(CachedUpdateKind::Chat)
    }

    fn apply_chat_list(
        &mut self,
        update: &Map<String, Value>,
        sequence: UpdateSequence,
        add: bool,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let chat_id = integer(update, "chat_id")?;
        let list = required_value(update, "chat_list")?.clone();
        object_ref(&list, "chat_list")?;
        {
            let lists = self.chat_array_mut(chat_id, "chat_lists")?;
            lists.retain(|known| known != &list);
            if add {
                lists.push(list);
            }
        }
        self.require_chat_mut(chat_id)?.sequence = sequence;
        Ok(CachedUpdateKind::Chat)
    }

    fn apply_chat_reply_markup(
        &mut self,
        update: &Map<String, Value>,
        sequence: UpdateSequence,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let chat_id = integer(update, "chat_id")?;
        let message_id = match required_value(update, "reply_markup_message")? {
            Value::Null => Value::String("0".to_owned()),
            message => {
                let message_object = object_ref(message, "reply_markup_message")?;
                require_type(message_object, "message", "reply_markup_message")?;
                let value = required_value(message_object, "id")?;
                integer_value(value, "id")?;
                value.clone()
            }
        };
        let cached = self.require_chat_mut(chat_id)?;
        object_mut(&mut cached.value, "chat")?
            .insert("reply_markup_message_id".to_owned(), message_id);
        cached.sequence = sequence;
        Ok(CachedUpdateKind::Chat)
    }

    fn apply_file(
        &mut self,
        update: &Map<String, Value>,
        sequence: UpdateSequence,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let value = required_value(update, "file")?.clone();
        let value_object = object_ref(&value, "file")?;
        require_type(value_object, "file", "file")?;
        let id = integer32(value_object, "id")?;
        self.files.insert(id, VersionedValue { sequence, value });
        Ok(CachedUpdateKind::File)
    }

    fn apply_message_acknowledged(
        &mut self,
        update: &Map<String, Value>,
        sequence: UpdateSequence,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let key = MessageSendKey {
            chat_id: integer(update, "chat_id")?,
            old_message_id: integer(update, "message_id")?,
        };
        if matches!(
            self.message_sends.get(&key).map(|known| &known.state),
            Some(MessageSendState::Succeeded { .. } | MessageSendState::Failed { .. })
        ) {
            return Err(ReducerError::TerminalMessageSendState);
        }
        self.message_sends.insert(
            key,
            VersionedMessageSendState {
                sequence,
                state: MessageSendState::Acknowledged,
            },
        );
        Ok(CachedUpdateKind::MessageSend)
    }

    fn apply_message_terminal(
        &mut self,
        update: &Map<String, Value>,
        sequence: UpdateSequence,
        succeeded: bool,
    ) -> Result<CachedUpdateKind, ReducerError> {
        let message = required_value(update, "message")?.clone();
        let message_object = object_ref(&message, "message")?;
        require_type(message_object, "message", "message")?;
        let key = MessageSendKey {
            chat_id: integer(message_object, "chat_id")?,
            old_message_id: integer(update, "old_message_id")?,
        };
        if matches!(
            self.message_sends.get(&key).map(|known| &known.state),
            Some(MessageSendState::Succeeded { .. } | MessageSendState::Failed { .. })
        ) {
            return Err(ReducerError::TerminalMessageSendState);
        }
        let state = if succeeded {
            MessageSendState::Succeeded { message }
        } else {
            let error = required_value(update, "error")?.clone();
            require_type(object_ref(&error, "error")?, "error", "error")?;
            MessageSendState::Failed { message, error }
        };
        self.message_sends
            .insert(key, VersionedMessageSendState { sequence, state });
        Ok(CachedUpdateKind::MessageSend)
    }

    fn require_chat(&self, chat_id: i64) -> Result<(), ReducerError> {
        self.chats
            .contains_key(&chat_id)
            .then_some(())
            .ok_or(ReducerError::MissingBaseEntity {
                entity: "chat",
                id: chat_id,
            })
    }

    fn require_chat_mut(&mut self, chat_id: i64) -> Result<&mut VersionedValue, ReducerError> {
        self.chats
            .get_mut(&chat_id)
            .ok_or(ReducerError::MissingBaseEntity {
                entity: "chat",
                id: chat_id,
            })
    }

    fn chat_array_mut(
        &mut self,
        chat_id: i64,
        field: &'static str,
    ) -> Result<&mut Vec<Value>, ReducerError> {
        let chat = self.require_chat_mut(chat_id)?;
        object_mut(&mut chat.value, "chat")?
            .get_mut(field)
            .ok_or(ReducerError::MissingField(field))?
            .as_array_mut()
            .ok_or(ReducerError::InvalidField(field))
    }
}

fn chat_direct_fields(update_type: &str) -> Option<&'static [&'static str]> {
    Some(match update_type {
        "updateChatTitle" => &["title"],
        "updateChatPhoto" => &["photo"],
        "updateChatAccentColors" => &[
            "accent_color_id",
            "background_custom_emoji_id",
            "upgraded_gift_colors",
            "profile_accent_color_id",
            "profile_background_custom_emoji_id",
        ],
        "updateChatPermissions" => &["permissions"],
        "updateChatLastMessage" => &["last_message", "positions"],
        "updateChatReadInbox" => &["last_read_inbox_message_id", "unread_count"],
        "updateChatReadOutbox" => &["last_read_outbox_message_id"],
        "updateChatActionBar" => &["action_bar"],
        "updateChatBusinessBotManageBar" => &["business_bot_manage_bar"],
        "updateChatAvailableReactions" => &["available_reactions"],
        "updateChatDraftMessage" => &["draft_message", "positions"],
        "updateChatEmojiStatus" => &["emoji_status"],
        "updateChatMessageSender" => &["message_sender_id"],
        "updateChatMessageAutoDeleteTime" => &["message_auto_delete_time"],
        "updateChatNotificationSettings" => &["notification_settings"],
        "updateChatPendingJoinRequests" => &["pending_join_requests"],
        "updateChatBackground" => &["background"],
        "updateChatTheme" => &["theme"],
        "updateChatUnreadMentionCount" => &["unread_mention_count"],
        "updateChatUnreadReactionCount" => &["unread_reaction_count"],
        "updateChatUnreadPollVoteCount" => &["unread_poll_vote_count"],
        "updateChatVideoChat" => &["video_chat"],
        "updateChatDefaultDisableNotification" => &["default_disable_notification"],
        "updateChatHasProtectedContent" => &["has_protected_content"],
        "updateChatIsTranslatable" => &["is_translatable"],
        "updateChatIsMarkedAsUnread" => &["is_marked_as_unread"],
        "updateChatViewAsTopics" => &["view_as_topics"],
        "updateChatBlockList" => &["block_list"],
        "updateChatHasScheduledMessages" => &["has_scheduled_messages"],
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReducerError {
    ExpectedObject(&'static str),
    MissingField(&'static str),
    InvalidField(&'static str),
    MissingBaseEntity { entity: &'static str, id: i64 },
    TerminalMessageSendState,
    SequenceExhausted,
}

impl fmt::Display for ReducerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedObject(field) => write!(formatter, "{field} must be an object"),
            Self::MissingField(field) => write!(formatter, "missing update field {field}"),
            Self::InvalidField(field) => write!(formatter, "invalid update field {field}"),
            Self::MissingBaseEntity { entity, id } => {
                write!(formatter, "missing base {entity} entity {id}")
            }
            Self::TerminalMessageSendState => {
                formatter.write_str("message send state is already terminal")
            }
            Self::SequenceExhausted => formatter.write_str("update sequence exhausted"),
        }
    }
}

impl std::error::Error for ReducerError {}

fn object<'a>(
    value: &'a Value,
    name: &'static str,
) -> Result<&'a Map<String, Value>, ReducerError> {
    value.as_object().ok_or(ReducerError::ExpectedObject(name))
}

fn object_ref<'a>(
    value: &'a Value,
    name: &'static str,
) -> Result<&'a Map<String, Value>, ReducerError> {
    object(value, name)
}

fn object_mut<'a>(
    value: &'a mut Value,
    name: &'static str,
) -> Result<&'a mut Map<String, Value>, ReducerError> {
    value
        .as_object_mut()
        .ok_or(ReducerError::ExpectedObject(name))
}

fn required_value<'a>(
    object: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a Value, ReducerError> {
    object.get(name).ok_or(ReducerError::MissingField(name))
}

fn string<'a>(object: &'a Map<String, Value>, name: &'static str) -> Result<&'a str, ReducerError> {
    required_value(object, name)?
        .as_str()
        .ok_or(ReducerError::InvalidField(name))
}

fn integer(object: &Map<String, Value>, name: &'static str) -> Result<i64, ReducerError> {
    integer_value(required_value(object, name)?, name)
}

fn integer_value(value: &Value, name: &'static str) -> Result<i64, ReducerError> {
    value
        .as_i64()
        .or_else(|| value.as_str()?.parse().ok())
        .ok_or(ReducerError::InvalidField(name))
}

fn integer32(object: &Map<String, Value>, name: &'static str) -> Result<i32, ReducerError> {
    i32::try_from(integer(object, name)?).map_err(|_| ReducerError::InvalidField(name))
}

fn require_type(
    object: &Map<String, Value>,
    expected: &str,
    field: &'static str,
) -> Result<(), ReducerError> {
    if string(object, "@type")? == expected {
        Ok(())
    } else {
        Err(ReducerError::InvalidField(field))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn representative_updates_fill_caches_in_call_order() {
        let mut reducer = StateReducer::default();
        let updates = [
            json!({"@type":"updateAuthorizationState","authorization_state":{"@type":"authorizationStateReady"}}),
            json!({"@type":"updateUser","user":{"@type":"user","id":"1","status":{"@type":"userStatusOffline"}}}),
            json!({"@type":"updateUserStatus","user_id":"1","status":{"@type":"userStatusOnline","expires":10}}),
            json!({"@type":"updateNewChat","chat":{"@type":"chat","id":"2","title":"old","positions":[],"chat_lists":[],"reply_markup_message_id":"0"}}),
            json!({"@type":"updateChatTitle","chat_id":"2","title":"new"}),
            json!({"@type":"updateChatPosition","chat_id":"2","position":{"@type":"chatPosition","list":{"@type":"chatListMain"},"order":"10","is_pinned":false,"source":null}}),
            json!({"@type":"updateBasicGroup","basic_group":{"@type":"basicGroup","id":"3"}}),
            json!({"@type":"updateSupergroup","supergroup":{"@type":"supergroup","id":"4"}}),
            json!({"@type":"updateFile","file":{"@type":"file","id":5}}),
            json!({"@type":"updateConnectionState","state":{"@type":"connectionStateReady"}}),
        ];
        for (index, update) in updates.into_iter().enumerate() {
            assert_eq!(
                reducer
                    .apply_transport_event(&crate::transport::TdJsonEvent::Update(update))
                    .unwrap()
                    .unwrap()
                    .sequence
                    .get(),
                u64::try_from(index + 1).unwrap()
            );
        }

        assert_eq!(reducer.authorization().unwrap().sequence.get(), 1);
        assert_eq!(
            reducer.user(1).unwrap().value["status"]["@type"],
            "userStatusOnline"
        );
        assert_eq!(reducer.chat(2).unwrap().value["title"], "new");
        assert_eq!(
            reducer.chat(2).unwrap().value["positions"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(reducer.basic_group(3).unwrap().sequence.get(), 7);
        assert_eq!(reducer.supergroup(4).unwrap().sequence.get(), 8);
        assert_eq!(reducer.file(5).unwrap().sequence.get(), 9);
        assert_eq!(reducer.connection().unwrap().sequence.get(), 10);
    }

    #[test]
    fn message_send_state_is_keyed_by_old_id_and_terminal() {
        let mut reducer = StateReducer::default();
        let key = MessageSendKey {
            chat_id: 7,
            old_message_id: 8,
        };
        reducer
            .apply(&json!({"@type":"updateMessageSendAcknowledged","chat_id":"7","message_id":"8"}))
            .unwrap();
        reducer
            .apply(&json!({"@type":"updateMessageSendSucceeded","message":{"@type":"message","id":"9","chat_id":"7"},"old_message_id":"8"}))
            .unwrap();
        assert!(matches!(
            reducer.message_send(key).unwrap().state,
            MessageSendState::Succeeded { .. }
        ));
        assert_eq!(
            reducer
                .apply(
                    &json!({"@type":"updateMessageSendAcknowledged","chat_id":"7","message_id":"8"})
                )
                .unwrap_err(),
            ReducerError::TerminalMessageSendState
        );
        assert_eq!(reducer.last_sequence().unwrap().get(), 2);

        let failed_key = MessageSendKey {
            chat_id: 7,
            old_message_id: 10,
        };
        reducer
            .apply(&json!({
                "@type":"updateMessageSendFailed",
                "message":{"@type":"message","id":"10","chat_id":"7"},
                "old_message_id":"10",
                "error":{"@type":"error","code":400,"message":"synthetic failure"}
            }))
            .unwrap();
        assert!(matches!(
            reducer.message_send(failed_key).unwrap().state,
            MessageSendState::Failed { .. }
        ));
    }

    #[test]
    fn partial_update_requires_base_entity_and_unknown_only_advances_order() {
        let mut reducer = StateReducer::default();
        assert!(matches!(
            reducer.apply(&json!({"@type":"updateChatTitle","chat_id":42,"title":"lost"})),
            Err(ReducerError::MissingBaseEntity {
                entity: "chat",
                id: 42
            })
        ));
        assert_eq!(reducer.last_sequence(), None);
        let applied = reducer
            .apply(&json!({"@type":"updateFutureConstructor","payload":{"kept_later":true}}))
            .unwrap();
        assert_eq!(applied.kind, CachedUpdateKind::Unknown);
        assert_eq!(applied.sequence.get(), 1);
    }
}
