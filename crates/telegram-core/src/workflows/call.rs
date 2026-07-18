//! call workflows.

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupCallLeaveOutcome {
    AlreadyLeft,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GroupCallReceipt {
    pub group_call_id: i32,
    pub is_active: bool,
    pub is_joined: bool,
    pub need_rejoin: bool,
    pub outcome: Option<GroupCallLeaveOutcome>,
    pub cleanup_verified: bool,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

pub fn inspect_group_call(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    group_call_id: i32,
    deadline: Instant,
) -> Result<GroupCallReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    group_call_state(
        invoke(
            runtime,
            policy,
            "getGroupCall",
            json!({"@type":"getGroupCall","group_call_id":group_call_id}),
            deadline,
        )?
        .as_value(),
        None,
    )
}

pub fn leave_group_call(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    group_call_id: i32,
    deadline: Instant,
) -> Result<GroupCallReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    leave_group_call_with(group_call_id, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub(super) fn leave_group_call_with(
    group_call_id: i32,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<GroupCallReceipt, ChatWorkflowError> {
    if group_call_id <= 0 {
        return Err(ChatWorkflowError::InvalidGroupCall);
    }
    let before = call(
        "getGroupCall",
        json!({"@type":"getGroupCall","group_call_id":group_call_id}),
    )?;
    let before = group_call_state(before.as_value(), None)?;
    if !before.is_joined {
        return group_call_state(
            &json!({
                "@type":"groupCall","id":group_call_id,"is_active":before.is_active,
                "is_joined":false,"need_rejoin":before.need_rejoin
            }),
            Some(GroupCallLeaveOutcome::AlreadyLeft),
        );
    }
    match call(
        "leaveGroupCall",
        json!({"@type":"leaveGroupCall","group_call_id":group_call_id}),
    ) {
        Ok(response) => expect_ok(response, "leaveGroupCall")?,
        Err(error) if response_timed_out(&error) => {}
        Err(error) => return Err(error),
    }
    match call(
        "getGroupCall",
        json!({"@type":"getGroupCall","group_call_id":group_call_id}),
    ) {
        Ok(after) => {
            let joined = required_bool(after.as_value(), "is_joined", "getGroupCall")?;
            group_call_state(
                after.as_value(),
                Some(if joined {
                    GroupCallLeaveOutcome::Uncertain
                } else {
                    GroupCallLeaveOutcome::Verified
                }),
            )
        }
        Err(error) if response_timed_out(&error) => Ok(GroupCallReceipt {
            outcome: Some(GroupCallLeaveOutcome::Uncertain),
            cleanup_verified: false,
            complete: false,
            ..before
        }),
        Err(error) => Err(error),
    }
}

fn group_call_state(
    call: &Value,
    outcome: Option<GroupCallLeaveOutcome>,
) -> Result<GroupCallReceipt, ChatWorkflowError> {
    if call["@type"] != "groupCall" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getGroupCall",
        });
    }
    let is_joined = required_bool(call, "is_joined", "getGroupCall")?;
    let complete = !matches!(outcome, Some(GroupCallLeaveOutcome::Uncertain));
    Ok(GroupCallReceipt {
        group_call_id: required_i32(call, "id", "getGroupCall")?,
        is_active: required_bool(call, "is_active", "getGroupCall")?,
        is_joined,
        need_rejoin: required_bool(call, "need_rejoin", "getGroupCall")?,
        outcome,
        cleanup_verified: outcome.is_some() && !is_joined && complete,
        complete,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}
