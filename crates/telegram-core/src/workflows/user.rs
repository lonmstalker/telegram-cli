//! user workflows.

use super::*;

#[derive(Clone, Copy)]
pub enum UserTarget<'value> {
    SelfUser,
    Id(i64),
    PublicUsername(&'value str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivateFieldState {
    Unavailable,
    Redacted,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UserProfileView {
    pub user_id: i64,
    pub user: Value,
    pub full_info: Option<Value>,
    pub private_fields: BTreeMap<&'static str, PrivateFieldState>,
    pub sequence: Option<u64>,
    pub freshness: Freshness,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMutationOutcome {
    AlreadyApplied,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProfileNameReceipt {
    pub user_id: i64,
    pub outcome: ProfileMutationOutcome,
    pub sequence: Option<u64>,
    pub complete: bool,
}

pub fn user_profile(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    target: UserTarget<'_>,
    include_full_info: bool,
    deadline: Instant,
) -> Result<UserProfileView, ChatWorkflowError> {
    require_resynced(runtime)?;
    let (user, freshness) = match target {
        UserTarget::SelfUser => (
            call_and_apply(runtime, policy, "getMe", json!({"@type":"getMe"}), deadline)?
                .into_value(),
            Freshness::ServerSnapshot,
        ),
        UserTarget::Id(user_id) => match runtime.state().user(user_id) {
            Some(user) => (user.value.clone(), Freshness::OrderedUpdate),
            None => (
                call_and_apply(
                    runtime,
                    policy,
                    "getUser",
                    json!({"@type":"getUser","user_id":user_id}),
                    deadline,
                )?
                .into_value(),
                Freshness::ServerSnapshot,
            ),
        },
        UserTarget::PublicUsername(username) => {
            let chat = call_and_apply(
                runtime,
                policy,
                "searchPublicChat",
                json!({"@type":"searchPublicChat","username":username_value(username)?}),
                deadline,
            )?
            .into_value();
            let user_id = match chat["type"]["@type"].as_str() {
                Some("chatTypePrivate") => required_i64(&chat["type"], "user_id", "chat")?,
                _ => return Err(ChatWorkflowError::InvalidTarget),
            };
            match runtime.state().user(user_id) {
                Some(user) => (user.value.clone(), Freshness::OrderedUpdate),
                None => (
                    call_and_apply(
                        runtime,
                        policy,
                        "getUser",
                        json!({"@type":"getUser","user_id":user_id}),
                        deadline,
                    )?
                    .into_value(),
                    Freshness::ServerSnapshot,
                ),
            }
        }
    };
    if user["@type"] != "user" {
        return Err(ChatWorkflowError::UnexpectedResult { method: "getUser" });
    }
    let user_id = required_i64(&user, "id", "user")?;
    let full_info = include_full_info
        .then(|| {
            call_and_apply(
                runtime,
                policy,
                "getUserFullInfo",
                json!({"@type":"getUserFullInfo","user_id":user_id}),
                deadline,
            )
            .map(TdObject::into_value)
        })
        .transpose()?;
    let sequence = runtime
        .state()
        .user(user_id)
        .map(|value| value.sequence.get());
    Ok(redacted_profile(
        user_id, user, full_info, sequence, freshness,
    ))
}

pub fn update_profile_name(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    first_name: &str,
    last_name: &str,
    deadline: Instant,
) -> Result<ProfileNameReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    if !(1..=64).contains(&first_name.chars().count()) || last_name.chars().count() > 64 {
        return Err(ChatWorkflowError::InvalidProfileInput);
    }
    let current =
        call_and_apply(runtime, policy, "getMe", json!({"@type":"getMe"}), deadline)?.into_value();
    let user_id = required_i64(&current, "id", "getMe")?;
    if current["first_name"] == first_name && current["last_name"] == last_name {
        return Ok(ProfileNameReceipt {
            user_id,
            outcome: ProfileMutationOutcome::AlreadyApplied,
            sequence: runtime
                .state()
                .user(user_id)
                .map(|value| value.sequence.get()),
            complete: true,
        });
    }
    let baseline = last_sequence(runtime);
    expect_ok(
        call_and_apply(
            runtime,
            policy,
            "setName",
            json!({"@type":"setName","first_name":first_name,"last_name":last_name}),
            deadline,
        )?,
        "setName",
    )?;
    loop {
        if let Some(sequence) =
            matching_profile_name(runtime, user_id, first_name, last_name, baseline)
        {
            return Ok(ProfileNameReceipt {
                user_id,
                outcome: ProfileMutationOutcome::Verified,
                sequence: Some(sequence),
                complete: true,
            });
        }
        match runtime.next_event_until(deadline) {
            Ok(_) => {}
            Err(RuntimeError::DeadlineExceeded) => {
                return Ok(ProfileNameReceipt {
                    user_id,
                    outcome: ProfileMutationOutcome::Uncertain,
                    sequence: None,
                    complete: false,
                });
            }
            Err(error) => return Err(ChatWorkflowError::Runtime(error)),
        }
    }
}

fn redacted_profile(
    user_id: i64,
    user: Value,
    full_info: Option<Value>,
    sequence: Option<u64>,
    freshness: Freshness,
) -> UserProfileView {
    const USER_FIELDS: &[&str] = &[
        "id",
        "first_name",
        "last_name",
        "usernames",
        "status",
        "is_contact",
        "is_mutual_contact",
        "is_close_friend",
        "verification_status",
        "is_premium",
        "is_support",
        "restricts_new_chats",
        "paid_message_star_count",
        "have_access",
        "type",
    ];
    const FULL_INFO_FIELDS: &[&str] = &[
        "can_be_called",
        "supports_video_calls",
        "has_private_calls",
        "has_private_forwards",
        "has_restricted_voice_and_video_note_messages",
        "need_phone_number_privacy_exception",
        "bio",
        "personal_chat_id",
        "gift_count",
        "group_in_common_count",
        "incoming_paid_message_star_count",
        "outgoing_paid_message_star_count",
        "bot_info",
    ];
    let mut private_fields = BTreeMap::from([(
        "phone_number",
        private_field_state(user.get("phone_number")),
    )]);
    if let Some(info) = &full_info {
        for field in ["birthdate", "note", "business_info"] {
            private_fields.insert(field, private_field_state(info.get(field)));
        }
    }
    UserProfileView {
        user_id,
        user: selected_fields(&user, USER_FIELDS),
        full_info: full_info.map(|value| selected_fields(&value, FULL_INFO_FIELDS)),
        private_fields,
        sequence,
        freshness,
        complete: true,
    }
}

fn selected_fields(value: &Value, fields: &[&str]) -> Value {
    let fields = fields
        .iter()
        .filter_map(|field| {
            value
                .get(*field)
                .map(|value| ((*field).to_owned(), value.clone()))
        })
        .collect::<Map<_, _>>();
    Value::Object(fields)
}

fn private_field_state(value: Option<&Value>) -> PrivateFieldState {
    match value {
        None | Some(Value::Null) => PrivateFieldState::Unavailable,
        Some(Value::String(value)) if value.is_empty() => PrivateFieldState::Unavailable,
        _ => PrivateFieldState::Redacted,
    }
}

fn matching_profile_name(
    runtime: &CoreRuntime,
    user_id: i64,
    first_name: &str,
    last_name: &str,
    baseline: u64,
) -> Option<u64> {
    let user = runtime.state().user(user_id)?;
    (user.sequence.get() > baseline
        && user.value["first_name"] == first_name
        && user.value["last_name"] == last_name)
        .then_some(user.sequence.get())
}
