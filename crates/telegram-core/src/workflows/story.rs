//! story workflows.

use super::*;

#[derive(Clone, Copy)]
pub enum StoryPrivacy<'value> {
    Everyone(&'value [i64]),
    Contacts(&'value [i64]),
    CloseFriends,
    SelectedUsers(&'value [i64]),
}

#[derive(Clone, Copy)]
pub enum StoryAction<'value> {
    PostPhoto {
        chat_id: i64,
        photo_file_id: i32,
        caption: &'value str,
        privacy: StoryPrivacy<'value>,
        active_period: i32,
        is_posted_to_chat_page: bool,
        protect_content: bool,
    },
    Delete {
        story_poster_chat_id: i64,
        story_id: i32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryMutationKind {
    PostPhoto,
    Delete,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StoryMutationPlan {
    pub action: StoryMutationKind,
    pub story_poster_chat_id: i64,
    pub story_id: Option<i32>,
    pub risk: RiskClass,
    pub retry: RetryClass,
    pub plan_hash: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryMutationOutcome {
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StoryMutationReceipt {
    pub action: StoryMutationKind,
    pub story_poster_chat_id: i64,
    pub story_id: Option<i32>,
    pub candidate_story_ids: Vec<i32>,
    pub outcome: StoryMutationOutcome,
    pub cleanup_verified: bool,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

pub fn plan_story_mutation(
    action: StoryAction<'_>,
) -> Result<StoryMutationPlan, ChatWorkflowError> {
    let request = story_mutation_request(action)?;
    let request = ValidatedRequest::from_value(request)
        .map_err(|_| ChatWorkflowError::InvalidStoryMutation)?;
    let preview =
        PlanPreview::for_request(&request).map_err(|_| ChatWorkflowError::InvalidStoryMutation)?;
    Ok(StoryMutationPlan {
        action: action.kind(),
        story_poster_chat_id: action.chat_id(),
        story_id: action.story_id(),
        risk: preview.risk,
        retry: preview.retry,
        plan_hash: preview.hash.to_hex(),
    })
}

pub fn story_mutation_request(action: StoryAction<'_>) -> Result<Value, ChatWorkflowError> {
    action.validate()?;
    Ok(match action {
        StoryAction::PostPhoto {
            chat_id,
            photo_file_id,
            caption,
            privacy,
            active_period,
            is_posted_to_chat_page,
            protect_content,
        } => json!({
            "@type":"postStory",
            "chat_id":chat_id,
            "content":{
                "@type":"inputStoryContentPhoto",
                "photo":{"@type":"inputFileId","id":photo_file_id},
                "added_sticker_file_ids":[]
            },
            "areas":null,
            "caption":{"@type":"formattedText","text":caption,"entities":[]},
            "privacy_settings":privacy.tdjson(),
            "album_ids":[],
            "active_period":active_period,
            "from_story_full_id":null,
            "is_posted_to_chat_page":is_posted_to_chat_page,
            "protect_content":protect_content
        }),
        StoryAction::Delete {
            story_poster_chat_id,
            story_id,
        } => json!({
            "@type":"deleteStory",
            "story_poster_chat_id":story_poster_chat_id,
            "story_id":story_id
        }),
    })
}

pub fn apply_story_mutation(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    action: StoryAction<'_>,
    deadline: Instant,
) -> Result<StoryMutationReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    story_mutation_with(action, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

impl StoryAction<'_> {
    fn validate(self) -> Result<(), ChatWorkflowError> {
        if self.chat_id() <= 0 || self.story_id().is_some_and(|story_id| story_id <= 0) {
            return Err(ChatWorkflowError::InvalidStoryMutation);
        }
        if let Self::PostPhoto {
            photo_file_id,
            caption,
            privacy,
            active_period,
            ..
        } = self
        {
            if photo_file_id <= 0
                || caption.chars().count() > 1024
                || !matches!(active_period, 21_600 | 43_200 | 86_400 | 172_800)
                || !privacy.valid()
            {
                return Err(ChatWorkflowError::InvalidStoryMutation);
            }
        }
        Ok(())
    }

    fn kind(self) -> StoryMutationKind {
        match self {
            Self::PostPhoto { .. } => StoryMutationKind::PostPhoto,
            Self::Delete { .. } => StoryMutationKind::Delete,
        }
    }

    fn chat_id(self) -> i64 {
        match self {
            Self::PostPhoto { chat_id, .. } => chat_id,
            Self::Delete {
                story_poster_chat_id,
                ..
            } => story_poster_chat_id,
        }
    }

    fn story_id(self) -> Option<i32> {
        match self {
            Self::PostPhoto { .. } => None,
            Self::Delete { story_id, .. } => Some(story_id),
        }
    }
}

impl StoryPrivacy<'_> {
    fn valid(self) -> bool {
        match self {
            Self::Everyone(excluded) | Self::Contacts(excluded) => {
                excluded.iter().all(|user_id| *user_id > 0)
            }
            Self::CloseFriends => true,
            Self::SelectedUsers(user_ids) => {
                !user_ids.is_empty() && user_ids.iter().all(|user_id| *user_id > 0)
            }
        }
    }

    fn tdjson(self) -> Value {
        match self {
            Self::Everyone(except_user_ids) => json!({
                "@type":"storyPrivacySettingsEveryone",
                "except_user_ids":except_user_ids
            }),
            Self::Contacts(except_user_ids) => json!({
                "@type":"storyPrivacySettingsContacts",
                "except_user_ids":except_user_ids
            }),
            Self::CloseFriends => json!({"@type":"storyPrivacySettingsCloseFriends"}),
            Self::SelectedUsers(user_ids) => json!({
                "@type":"storyPrivacySettingsSelectedUsers",
                "user_ids":user_ids
            }),
        }
    }
}

pub(super) fn story_mutation_with(
    action: StoryAction<'_>,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StoryMutationReceipt, ChatWorkflowError> {
    action.validate()?;
    match action {
        StoryAction::PostPhoto { photo_file_id, .. } => {
            require_uploaded_file(photo_file_id, &mut call)?;
            let story = match call("postStory", story_mutation_request(action)?) {
                Ok(story) => story,
                Err(error) if response_timed_out(&error) => {
                    let candidates =
                        active_story_ids(action.chat_id(), &mut call).unwrap_or_default();
                    return Ok(story_receipt(
                        action,
                        None,
                        candidates,
                        StoryMutationOutcome::Uncertain,
                    ));
                }
                Err(error) => return Err(error),
            };
            let story_id = story_identity(story.as_value(), action.chat_id())?;
            match call(
                "getStory",
                json!({
                    "@type":"getStory",
                    "story_poster_chat_id":action.chat_id(),
                    "story_id":story_id,
                    "only_local":false
                }),
            ) {
                Ok(story) => {
                    let verified = story_identity(story.as_value(), action.chat_id())? == story_id
                        && !required_bool(story.as_value(), "is_being_posted", "getStory")?;
                    Ok(story_receipt(
                        action,
                        Some(story_id),
                        vec![story_id],
                        if verified {
                            StoryMutationOutcome::Verified
                        } else {
                            StoryMutationOutcome::Uncertain
                        },
                    ))
                }
                Err(error) if response_timed_out(&error) => Ok(story_receipt(
                    action,
                    Some(story_id),
                    vec![story_id],
                    StoryMutationOutcome::Uncertain,
                )),
                Err(error) => Err(error),
            }
        }
        StoryAction::Delete { story_id, .. } => {
            let story = call(
                "getStory",
                json!({
                    "@type":"getStory",
                    "story_poster_chat_id":action.chat_id(),
                    "story_id":story_id,
                    "only_local":false
                }),
            )?;
            let _ = story_identity(story.as_value(), action.chat_id())?;
            if !required_bool(story.as_value(), "can_be_deleted", "getStory")? {
                return Err(ChatWorkflowError::CapabilityDenied {
                    capability: "can_delete_story",
                });
            }
            let confirmed = match call("deleteStory", story_mutation_request(action)?) {
                Ok(response) => {
                    expect_ok(response, "deleteStory")?;
                    true
                }
                Err(error) if response_timed_out(&error) => false,
                Err(error) => return Err(error),
            };
            let candidates =
                active_story_ids(action.chat_id(), &mut call).unwrap_or_else(|_| vec![story_id]);
            Ok(story_receipt(
                action,
                Some(story_id),
                candidates.clone(),
                if confirmed && !candidates.contains(&story_id) {
                    StoryMutationOutcome::Verified
                } else {
                    StoryMutationOutcome::Uncertain
                },
            ))
        }
    }
}

fn story_identity(story: &Value, poster_chat_id: i64) -> Result<i32, ChatWorkflowError> {
    if story["@type"] != "story"
        || required_i64(story, "poster_chat_id", "story")? != poster_chat_id
    {
        return Err(ChatWorkflowError::UnexpectedResult { method: "story" });
    }
    required_i32(story, "id", "story")
}

fn active_story_ids(
    chat_id: i64,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<Vec<i32>, ChatWorkflowError> {
    let active = call(
        "getChatActiveStories",
        json!({"@type":"getChatActiveStories","chat_id":chat_id}),
    )?;
    if active.as_value()["@type"] != "chatActiveStories"
        || required_i64(active.as_value(), "chat_id", "getChatActiveStories")? != chat_id
    {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getChatActiveStories",
        });
    }
    active.as_value()["stories"]
        .as_array()
        .ok_or(ChatWorkflowError::InvalidResult {
            method: "getChatActiveStories",
            field: "stories",
        })?
        .iter()
        .map(|story| required_i32(story, "story_id", "getChatActiveStories"))
        .collect()
}

fn story_receipt(
    action: StoryAction<'_>,
    story_id: Option<i32>,
    candidate_story_ids: Vec<i32>,
    outcome: StoryMutationOutcome,
) -> StoryMutationReceipt {
    let complete = outcome == StoryMutationOutcome::Verified;
    StoryMutationReceipt {
        action: action.kind(),
        story_poster_chat_id: action.chat_id(),
        story_id,
        candidate_story_ids,
        outcome,
        cleanup_verified: action.kind() == StoryMutationKind::Delete && complete,
        complete,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    }
}
