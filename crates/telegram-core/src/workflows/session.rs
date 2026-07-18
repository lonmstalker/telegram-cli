//! session workflows.

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationScope {
    PrivateChats,
    GroupChats,
    ChannelChats,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NotificationSettingsPatch {
    pub mute_for: Option<i32>,
    pub sound_id: Option<i64>,
    pub show_preview: Option<bool>,
    pub use_default_mute_stories: Option<bool>,
    pub mute_stories: Option<bool>,
    pub story_sound_id: Option<i64>,
    pub show_story_poster: Option<bool>,
    pub disable_pinned_message_notifications: Option<bool>,
    pub disable_mention_notifications: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NotificationSettingsSnapshot {
    pub scope: NotificationScope,
    pub mute_for: i32,
    pub sound_id: i64,
    pub show_preview: bool,
    pub use_default_mute_stories: bool,
    pub mute_stories: bool,
    pub story_sound_id: i64,
    pub show_story_poster: bool,
    pub disable_pinned_message_notifications: bool,
    pub disable_mention_notifications: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsMutationOutcome {
    AlreadyConverged,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NotificationSettingsReceipt {
    pub settings: NotificationSettingsSnapshot,
    pub changed_fields: Vec<&'static str>,
    pub outcome: SettingsMutationOutcome,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionSummary {
    pub session_id: i64,
    pub is_current: bool,
    pub is_password_pending: bool,
    pub is_unconfirmed: bool,
    pub is_official_application: bool,
    pub last_active_date: i32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActiveSessionsSnapshot {
    pub sessions: Vec<SessionSummary>,
    pub inactive_session_ttl_days: i32,
    pub sensitive_metadata_redacted: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionTerminationPlan {
    pub session_id: i64,
    pub risk: RiskClass,
    pub retry: RetryClass,
    pub plan_hash: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTerminationOutcome {
    AlreadyTerminated,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionTerminationReceipt {
    pub session_id: i64,
    pub outcome: SessionTerminationOutcome,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

pub fn notification_settings(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    scope: NotificationScope,
    deadline: Instant,
) -> Result<NotificationSettingsSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    read_notification_settings(scope, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub fn set_notification_settings(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    scope: NotificationScope,
    patch: NotificationSettingsPatch,
    deadline: Instant,
) -> Result<NotificationSettingsReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    set_notification_settings_with(scope, patch, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub fn active_sessions(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    deadline: Instant,
) -> Result<ActiveSessionsSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    active_sessions_with(|method, request| invoke(runtime, policy, method, request, deadline))
}

pub fn plan_terminate_session(
    session_id: i64,
) -> Result<SessionTerminationPlan, ChatWorkflowError> {
    let request = terminate_session_request(session_id)?;
    let request = ValidatedRequest::from_value(request)
        .map_err(|_| ChatWorkflowError::InvalidSessionTarget)?;
    let preview =
        PlanPreview::for_request(&request).map_err(|_| ChatWorkflowError::InvalidSessionTarget)?;
    Ok(SessionTerminationPlan {
        session_id,
        risk: preview.risk,
        retry: preview.retry,
        plan_hash: preview.hash.to_hex(),
    })
}

pub fn terminate_session_request(session_id: i64) -> Result<Value, ChatWorkflowError> {
    if session_id <= 0 {
        return Err(ChatWorkflowError::InvalidSessionTarget);
    }
    Ok(json!({"@type":"terminateSession","session_id":session_id}))
}

pub fn apply_terminate_session(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    session_id: i64,
    deadline: Instant,
) -> Result<SessionTerminationReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    terminate_session_with(session_id, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

impl NotificationScope {
    fn tdjson(self) -> Value {
        let kind = match self {
            Self::PrivateChats => "notificationSettingsScopePrivateChats",
            Self::GroupChats => "notificationSettingsScopeGroupChats",
            Self::ChannelChats => "notificationSettingsScopeChannelChats",
        };
        json!({"@type":kind})
    }
}

impl NotificationSettingsSnapshot {
    fn from_tdjson(scope: NotificationScope, settings: &Value) -> Result<Self, ChatWorkflowError> {
        if settings["@type"] != "scopeNotificationSettings" {
            return Err(ChatWorkflowError::UnexpectedResult {
                method: "getScopeNotificationSettings",
            });
        }
        Ok(Self {
            scope,
            mute_for: required_i32(settings, "mute_for", "getScopeNotificationSettings")?,
            sound_id: required_i64(settings, "sound_id", "getScopeNotificationSettings")?,
            show_preview: required_bool(settings, "show_preview", "getScopeNotificationSettings")?,
            use_default_mute_stories: required_bool(
                settings,
                "use_default_mute_stories",
                "getScopeNotificationSettings",
            )?,
            mute_stories: required_bool(settings, "mute_stories", "getScopeNotificationSettings")?,
            story_sound_id: required_i64(
                settings,
                "story_sound_id",
                "getScopeNotificationSettings",
            )?,
            show_story_poster: required_bool(
                settings,
                "show_story_poster",
                "getScopeNotificationSettings",
            )?,
            disable_pinned_message_notifications: required_bool(
                settings,
                "disable_pinned_message_notifications",
                "getScopeNotificationSettings",
            )?,
            disable_mention_notifications: required_bool(
                settings,
                "disable_mention_notifications",
                "getScopeNotificationSettings",
            )?,
            observed_at: SystemTime::now(),
            freshness: Freshness::ServerSnapshot,
        })
    }

    fn tdjson(&self) -> Value {
        json!({
            "@type":"scopeNotificationSettings",
            "mute_for":self.mute_for,
            "sound_id":self.sound_id,
            "show_preview":self.show_preview,
            "use_default_mute_stories":self.use_default_mute_stories,
            "mute_stories":self.mute_stories,
            "story_sound_id":self.story_sound_id,
            "show_story_poster":self.show_story_poster,
            "disable_pinned_message_notifications":self.disable_pinned_message_notifications,
            "disable_mention_notifications":self.disable_mention_notifications
        })
    }

    fn same_values(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.mute_for == other.mute_for
            && self.sound_id == other.sound_id
            && self.show_preview == other.show_preview
            && self.use_default_mute_stories == other.use_default_mute_stories
            && self.mute_stories == other.mute_stories
            && self.story_sound_id == other.story_sound_id
            && self.show_story_poster == other.show_story_poster
            && self.disable_pinned_message_notifications
                == other.disable_pinned_message_notifications
            && self.disable_mention_notifications == other.disable_mention_notifications
    }
}

fn read_notification_settings(
    scope: NotificationScope,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<NotificationSettingsSnapshot, ChatWorkflowError> {
    fetch_notification_settings(scope, &mut call)
}

fn fetch_notification_settings(
    scope: NotificationScope,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<NotificationSettingsSnapshot, ChatWorkflowError> {
    let settings = call(
        "getScopeNotificationSettings",
        json!({"@type":"getScopeNotificationSettings","scope":scope.tdjson()}),
    )?;
    NotificationSettingsSnapshot::from_tdjson(scope, settings.as_value())
}

pub(super) fn set_notification_settings_with(
    scope: NotificationScope,
    patch: NotificationSettingsPatch,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<NotificationSettingsReceipt, ChatWorkflowError> {
    if patch.mute_for.is_some_and(|value| value < 0)
        || patch.sound_id.is_some_and(|value| value < 0)
        || patch.story_sound_id.is_some_and(|value| value < 0)
    {
        return Err(ChatWorkflowError::InvalidNotificationSettings);
    }
    let current = fetch_notification_settings(scope, &mut call)?;
    let mut desired = current.clone();
    let mut changed_fields = Vec::new();
    macro_rules! apply {
        ($field:ident) => {
            if let Some(value) = patch.$field {
                if desired.$field != value {
                    desired.$field = value;
                    changed_fields.push(stringify!($field));
                }
            }
        };
    }
    apply!(mute_for);
    apply!(sound_id);
    apply!(show_preview);
    apply!(use_default_mute_stories);
    apply!(mute_stories);
    apply!(story_sound_id);
    apply!(show_story_poster);
    apply!(disable_pinned_message_notifications);
    apply!(disable_mention_notifications);
    if changed_fields.is_empty() {
        return Ok(NotificationSettingsReceipt {
            settings: current,
            changed_fields,
            outcome: SettingsMutationOutcome::AlreadyConverged,
            complete: true,
        });
    }
    match call(
        "setScopeNotificationSettings",
        json!({
            "@type":"setScopeNotificationSettings",
            "scope":scope.tdjson(),
            "notification_settings":desired.tdjson()
        }),
    ) {
        Ok(response) => expect_ok(response, "setScopeNotificationSettings")?,
        Err(error) if response_timed_out(&error) => {}
        Err(error) => return Err(error),
    }
    match fetch_notification_settings(scope, &mut call) {
        Ok(settings) => {
            let complete = settings.same_values(&desired);
            Ok(NotificationSettingsReceipt {
                settings,
                changed_fields,
                outcome: if complete {
                    SettingsMutationOutcome::Verified
                } else {
                    SettingsMutationOutcome::Uncertain
                },
                complete,
            })
        }
        Err(_) => Ok(NotificationSettingsReceipt {
            settings: current,
            changed_fields,
            outcome: SettingsMutationOutcome::Uncertain,
            complete: false,
        }),
    }
}

pub(super) fn active_sessions_with(
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<ActiveSessionsSnapshot, ChatWorkflowError> {
    fetch_active_sessions(&mut call)
}

fn fetch_active_sessions(
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<ActiveSessionsSnapshot, ChatWorkflowError> {
    let result = call("getActiveSessions", json!({"@type":"getActiveSessions"}))?;
    if result.as_value()["@type"] != "sessions" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getActiveSessions",
        });
    }
    let sessions =
        result.as_value()["sessions"]
            .as_array()
            .ok_or(ChatWorkflowError::InvalidResult {
                method: "getActiveSessions",
                field: "sessions",
            })?;
    let mut seen = BTreeSet::new();
    let sessions = sessions
        .iter()
        .map(|session| {
            if session["@type"] != "session" {
                return Err(ChatWorkflowError::UnexpectedResult {
                    method: "getActiveSessions",
                });
            }
            let session_id = required_i64(session, "id", "getActiveSessions")?;
            if !seen.insert(session_id) {
                return Err(ChatWorkflowError::InvalidResult {
                    method: "getActiveSessions",
                    field: "sessions[].id",
                });
            }
            Ok(SessionSummary {
                session_id,
                is_current: required_bool(session, "is_current", "getActiveSessions")?,
                is_password_pending: required_bool(
                    session,
                    "is_password_pending",
                    "getActiveSessions",
                )?,
                is_unconfirmed: required_bool(session, "is_unconfirmed", "getActiveSessions")?,
                is_official_application: required_bool(
                    session,
                    "is_official_application",
                    "getActiveSessions",
                )?,
                last_active_date: required_i32(session, "last_active_date", "getActiveSessions")?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ActiveSessionsSnapshot {
        sessions,
        inactive_session_ttl_days: required_i32(
            result.as_value(),
            "inactive_session_ttl_days",
            "getActiveSessions",
        )?,
        sensitive_metadata_redacted: true,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

pub(super) fn terminate_session_with(
    session_id: i64,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<SessionTerminationReceipt, ChatWorkflowError> {
    let _ = terminate_session_request(session_id)?;
    let before = fetch_active_sessions(&mut call)?;
    let Some(target) = before
        .sessions
        .iter()
        .find(|session| session.session_id == session_id)
    else {
        return Ok(session_termination_receipt(
            session_id,
            SessionTerminationOutcome::AlreadyTerminated,
        ));
    };
    if target.is_current {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "non_current_session",
        });
    }
    match call("terminateSession", terminate_session_request(session_id)?) {
        Ok(response) => expect_ok(response, "terminateSession")?,
        Err(error) if response_timed_out(&error) => {}
        Err(error) => return Err(error),
    }
    let outcome = match fetch_active_sessions(&mut call) {
        Ok(after)
            if after
                .sessions
                .iter()
                .all(|session| session.session_id != session_id) =>
        {
            SessionTerminationOutcome::Verified
        }
        Ok(_) | Err(_) => SessionTerminationOutcome::Uncertain,
    };
    Ok(session_termination_receipt(session_id, outcome))
}

fn session_termination_receipt(
    session_id: i64,
    outcome: SessionTerminationOutcome,
) -> SessionTerminationReceipt {
    SessionTerminationReceipt {
        session_id,
        outcome,
        complete: outcome != SessionTerminationOutcome::Uncertain,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    }
}
