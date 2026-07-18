//! sticker workflows.

use super::*;

#[derive(Clone, Copy)]
pub enum StickerFormat {
    Webp,
    Tgs,
    Webm,
}

#[derive(Clone, Copy)]
pub enum CustomEmojiSetAction<'value> {
    Create {
        user_id: i64,
        title: &'value str,
        name: &'value str,
        format: StickerFormat,
        sticker_file_id: i32,
        emojis: &'value str,
        needs_repainting: bool,
    },
    Add {
        user_id: i64,
        set_id: i64,
        name: &'value str,
        format: StickerFormat,
        sticker_file_id: i32,
        emojis: &'value str,
    },
    Delete {
        set_id: i64,
        name: &'value str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StickerSetMutationKind {
    Create,
    Add,
    Delete,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StickerSetMutationPlan {
    pub action: StickerSetMutationKind,
    pub set_id: Option<i64>,
    pub name: String,
    pub risk: RiskClass,
    pub retry: RetryClass,
    pub plan_hash: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StickerSetMutationOutcome {
    AlreadyApplied,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StickerSetMutationReceipt {
    pub action: StickerSetMutationKind,
    pub set_id: Option<i64>,
    pub name: String,
    pub sticker_count: usize,
    pub outcome: StickerSetMutationOutcome,
    pub cleanup_verified: bool,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

pub fn plan_custom_emoji_set(
    action: CustomEmojiSetAction<'_>,
) -> Result<StickerSetMutationPlan, ChatWorkflowError> {
    let request = custom_emoji_set_request(action)?;
    let request = ValidatedRequest::from_value(request)
        .map_err(|_| ChatWorkflowError::InvalidStickerSetMutation)?;
    let preview = PlanPreview::for_request(&request)
        .map_err(|_| ChatWorkflowError::InvalidStickerSetMutation)?;
    Ok(StickerSetMutationPlan {
        action: action.kind(),
        set_id: action.set_id(),
        name: action.name().to_owned(),
        risk: preview.risk,
        retry: preview.retry,
        plan_hash: preview.hash.to_hex(),
    })
}

pub fn custom_emoji_set_request(
    action: CustomEmojiSetAction<'_>,
) -> Result<Value, ChatWorkflowError> {
    action.validate()?;
    Ok(match action {
        CustomEmojiSetAction::Create {
            user_id,
            title,
            name,
            format,
            sticker_file_id,
            emojis,
            needs_repainting,
        } => json!({
            "@type":"createNewStickerSet",
            "user_id":user_id,
            "title":title,
            "name":name,
            "sticker_type":{"@type":"stickerTypeCustomEmoji"},
            "needs_repainting":needs_repainting,
            "stickers":[new_sticker(sticker_file_id, format, emojis)],
            "source":""
        }),
        CustomEmojiSetAction::Add {
            user_id,
            name,
            format,
            sticker_file_id,
            emojis,
            ..
        } => json!({
            "@type":"addStickerToSet",
            "user_id":user_id,
            "name":name,
            "sticker":new_sticker(sticker_file_id, format, emojis)
        }),
        CustomEmojiSetAction::Delete { name, .. } => {
            json!({"@type":"deleteStickerSet","name":name})
        }
    })
}

pub fn apply_custom_emoji_set(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    action: CustomEmojiSetAction<'_>,
    deadline: Instant,
) -> Result<StickerSetMutationReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    custom_emoji_set_with(action, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

impl StickerFormat {
    pub(super) fn tdjson(self) -> Value {
        let kind = match self {
            Self::Webp => "stickerFormatWebp",
            Self::Tgs => "stickerFormatTgs",
            Self::Webm => "stickerFormatWebm",
        };
        json!({"@type":kind})
    }
}

fn new_sticker(file_id: i32, format: StickerFormat, emojis: &str) -> Value {
    json!({
        "@type":"newSticker",
        "sticker":{"@type":"inputFileId","id":file_id},
        "format":format.tdjson(),
        "emojis":emojis,
        "mask_position":null,
        "keywords":[]
    })
}

pub(super) fn custom_emoji_set_with(
    action: CustomEmojiSetAction<'_>,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StickerSetMutationReceipt, ChatWorkflowError> {
    action.validate()?;
    match action {
        CustomEmojiSetAction::Create {
            sticker_file_id, ..
        } => {
            require_uploaded_file(sticker_file_id, &mut call)?;
            create_custom_emoji_set(action, sticker_file_id, &mut call)
        }
        CustomEmojiSetAction::Add {
            set_id,
            sticker_file_id,
            ..
        } => {
            require_uploaded_file(sticker_file_id, &mut call)?;
            add_custom_emoji_sticker(action, set_id, sticker_file_id, &mut call)
        }
        CustomEmojiSetAction::Delete { set_id, .. } => {
            delete_custom_emoji_set(action, set_id, &mut call)
        }
    }
}

fn create_custom_emoji_set(
    action: CustomEmojiSetAction<'_>,
    sticker_file_id: i32,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StickerSetMutationReceipt, ChatWorkflowError> {
    if !sticker_name_available(action.name(), call)? {
        let set = search_sticker_set(action.name(), call)?;
        let (set_id, count, contains_file) =
            custom_emoji_set_state(set.as_value(), action.name(), sticker_file_id)?;
        return if contains_file {
            Ok(sticker_set_receipt(
                action,
                Some(set_id),
                count,
                StickerSetMutationOutcome::AlreadyApplied,
            ))
        } else {
            Err(ChatWorkflowError::InvalidStickerSetMutation)
        };
    }

    let response = match call("createNewStickerSet", custom_emoji_set_request(action)?) {
        Ok(response) => response,
        Err(error) if response_timed_out(&error) => {
            return reconcile_created_custom_emoji_set(action, sticker_file_id, call);
        }
        Err(error) => return Err(error),
    };
    let (set_id, count, _) =
        custom_emoji_set_state(response.as_value(), action.name(), sticker_file_id)?;
    match call(
        "getStickerSet",
        json!({"@type":"getStickerSet","set_id":set_id}),
    ) {
        Ok(set) => {
            let (set_id, count, contains_file) =
                custom_emoji_set_state(set.as_value(), action.name(), sticker_file_id)?;
            Ok(sticker_set_receipt(
                action,
                Some(set_id),
                count,
                if contains_file {
                    StickerSetMutationOutcome::Verified
                } else {
                    StickerSetMutationOutcome::Uncertain
                },
            ))
        }
        Err(error) if response_timed_out(&error) => Ok(sticker_set_receipt(
            action,
            Some(set_id),
            count,
            StickerSetMutationOutcome::Uncertain,
        )),
        Err(error) => Err(error),
    }
}

fn reconcile_created_custom_emoji_set(
    action: CustomEmojiSetAction<'_>,
    sticker_file_id: i32,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StickerSetMutationReceipt, ChatWorkflowError> {
    match sticker_name_available(action.name(), call) {
        Ok(false) => {
            let set = search_sticker_set(action.name(), call)?;
            let (set_id, count, contains_file) =
                custom_emoji_set_state(set.as_value(), action.name(), sticker_file_id)?;
            Ok(sticker_set_receipt(
                action,
                Some(set_id),
                count,
                if contains_file {
                    StickerSetMutationOutcome::Verified
                } else {
                    StickerSetMutationOutcome::Uncertain
                },
            ))
        }
        Ok(true) | Err(_) => Ok(sticker_set_receipt(
            action,
            None,
            0,
            StickerSetMutationOutcome::Uncertain,
        )),
    }
}

fn add_custom_emoji_sticker(
    action: CustomEmojiSetAction<'_>,
    set_id: i64,
    sticker_file_id: i32,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StickerSetMutationReceipt, ChatWorkflowError> {
    let before = call(
        "getStickerSet",
        json!({"@type":"getStickerSet","set_id":set_id}),
    )?;
    let (_, count, contains_file) =
        custom_emoji_set_state(before.as_value(), action.name(), sticker_file_id)?;
    if contains_file {
        return Ok(sticker_set_receipt(
            action,
            Some(set_id),
            count,
            StickerSetMutationOutcome::AlreadyApplied,
        ));
    }

    match call("addStickerToSet", custom_emoji_set_request(action)?) {
        Ok(response) => expect_ok(response, "addStickerToSet")?,
        Err(error) if response_timed_out(&error) => {}
        Err(error) => return Err(error),
    }
    match call(
        "getStickerSet",
        json!({"@type":"getStickerSet","set_id":set_id}),
    ) {
        Ok(set) => {
            let (_, count, contains_file) =
                custom_emoji_set_state(set.as_value(), action.name(), sticker_file_id)?;
            Ok(sticker_set_receipt(
                action,
                Some(set_id),
                count,
                if contains_file {
                    StickerSetMutationOutcome::Verified
                } else {
                    StickerSetMutationOutcome::Uncertain
                },
            ))
        }
        Err(error) if response_timed_out(&error) => Ok(sticker_set_receipt(
            action,
            Some(set_id),
            count,
            StickerSetMutationOutcome::Uncertain,
        )),
        Err(error) => Err(error),
    }
}

fn delete_custom_emoji_set(
    action: CustomEmojiSetAction<'_>,
    set_id: i64,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StickerSetMutationReceipt, ChatWorkflowError> {
    if sticker_name_available(action.name(), call)? {
        return Ok(sticker_set_receipt(
            action,
            Some(set_id),
            0,
            StickerSetMutationOutcome::AlreadyApplied,
        ));
    }
    let set = call(
        "getStickerSet",
        json!({"@type":"getStickerSet","set_id":set_id}),
    )?;
    let _ = custom_emoji_set_state(set.as_value(), action.name(), 0)?;
    match call("deleteStickerSet", custom_emoji_set_request(action)?) {
        Ok(response) => expect_ok(response, "deleteStickerSet")?,
        Err(error) if response_timed_out(&error) => {}
        Err(error) => return Err(error),
    }
    let outcome = match sticker_name_available(action.name(), call) {
        Ok(true) => StickerSetMutationOutcome::Verified,
        Ok(false) | Err(_) => StickerSetMutationOutcome::Uncertain,
    };
    Ok(sticker_set_receipt(action, Some(set_id), 0, outcome))
}

fn sticker_name_available(
    name: &str,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<bool, ChatWorkflowError> {
    let result = call(
        "checkStickerSetName",
        json!({"@type":"checkStickerSetName","name":name}),
    )?;
    match result.as_value()["@type"].as_str() {
        Some("checkStickerSetNameResultOk") => Ok(true),
        Some("checkStickerSetNameResultNameOccupied") => Ok(false),
        Some("checkStickerSetNameResultNameInvalid") => {
            Err(ChatWorkflowError::InvalidStickerSetMutation)
        }
        _ => Err(ChatWorkflowError::UnexpectedResult {
            method: "checkStickerSetName",
        }),
    }
}

fn search_sticker_set(
    name: &str,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<TdObject, ChatWorkflowError> {
    call(
        "searchStickerSet",
        json!({"@type":"searchStickerSet","name":name,"ignore_cache":true}),
    )
}

fn custom_emoji_set_state(
    set: &Value,
    name: &str,
    sticker_file_id: i32,
) -> Result<(i64, usize, bool), ChatWorkflowError> {
    if set["@type"] != "stickerSet"
        || set["name"].as_str() != Some(name)
        || set["sticker_type"]["@type"] != "stickerTypeCustomEmoji"
    {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "sticker set verification",
        });
    }
    if !required_bool(set, "is_owned", "sticker set verification")? {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "sticker_set_owner",
        });
    }
    let stickers = set["stickers"]
        .as_array()
        .ok_or(ChatWorkflowError::InvalidResult {
            method: "sticker set verification",
            field: "stickers",
        })?;
    let contains_file = sticker_file_id > 0
        && stickers.iter().any(|sticker| {
            sticker["sticker"]["id"]
                .as_i64()
                .is_some_and(|id| id == i64::from(sticker_file_id))
        });
    Ok((
        required_i64(set, "id", "sticker set verification")?,
        stickers.len(),
        contains_file,
    ))
}

fn sticker_set_receipt(
    action: CustomEmojiSetAction<'_>,
    set_id: Option<i64>,
    sticker_count: usize,
    outcome: StickerSetMutationOutcome,
) -> StickerSetMutationReceipt {
    let complete = outcome != StickerSetMutationOutcome::Uncertain;
    StickerSetMutationReceipt {
        action: action.kind(),
        set_id,
        name: action.name().to_owned(),
        sticker_count,
        outcome,
        cleanup_verified: action.kind() == StickerSetMutationKind::Delete && complete,
        complete,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    }
}

impl<'value> CustomEmojiSetAction<'value> {
    fn validate(self) -> Result<(), ChatWorkflowError> {
        let valid_name = |name: &str| {
            (1..=64).contains(&name.len())
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        };
        if !valid_name(self.name()) || self.set_id().is_some_and(|set_id| set_id <= 0) {
            return Err(ChatWorkflowError::InvalidStickerSetMutation);
        }
        match self {
            Self::Create {
                user_id,
                title,
                sticker_file_id,
                emojis,
                ..
            } => {
                if user_id <= 0
                    || !(1..=64).contains(&title.chars().count())
                    || sticker_file_id <= 0
                    || emojis.is_empty()
                    || emojis.chars().count() > 64
                {
                    return Err(ChatWorkflowError::InvalidStickerSetMutation);
                }
            }
            Self::Add {
                user_id,
                sticker_file_id,
                emojis,
                ..
            } => {
                if user_id <= 0
                    || sticker_file_id <= 0
                    || emojis.is_empty()
                    || emojis.chars().count() > 64
                {
                    return Err(ChatWorkflowError::InvalidStickerSetMutation);
                }
            }
            Self::Delete { .. } => {}
        }
        Ok(())
    }

    fn kind(self) -> StickerSetMutationKind {
        match self {
            Self::Create { .. } => StickerSetMutationKind::Create,
            Self::Add { .. } => StickerSetMutationKind::Add,
            Self::Delete { .. } => StickerSetMutationKind::Delete,
        }
    }

    fn set_id(self) -> Option<i64> {
        match self {
            Self::Create { .. } => None,
            Self::Add { set_id, .. } | Self::Delete { set_id, .. } => Some(set_id),
        }
    }

    fn name(self) -> &'value str {
        match self {
            Self::Create { name, .. } | Self::Add { name, .. } | Self::Delete { name, .. } => name,
        }
    }
}
