//! proxy workflows.

use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProxySummary {
    pub proxy_id: i32,
    pub is_enabled: bool,
    pub proxy_type: String,
    pub endpoint_redacted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProxySnapshot {
    pub proxies: Vec<ProxySummary>,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyTransitionOutcome {
    AlreadyApplied,
    Verified,
    ConnectivityDiverged,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProxyTransitionReceipt {
    pub desired_proxy_id: Option<i32>,
    pub rollback_proxy_id: Option<i32>,
    pub outcome: ProxyTransitionOutcome,
    pub endpoint_redacted: bool,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

pub fn proxy_status(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    deadline: Instant,
) -> Result<ProxySnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    let (response, _) = invoke_ordered(
        runtime,
        policy,
        "getProxies",
        get_proxies_request(),
        deadline,
    )?;
    proxy_snapshot(response.as_value())
}

pub fn set_proxy_enabled(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    proxy_id: Option<i32>,
    deadline: Instant,
) -> Result<ProxyTransitionReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    set_proxy_enabled_with(proxy_id, |method, request| {
        invoke_ordered(runtime, policy, method, request, deadline)
    })
}

pub(super) fn set_proxy_enabled_with(
    desired_proxy_id: Option<i32>,
    mut call: impl FnMut(
        &'static str,
        Value,
    ) -> Result<(TdObject, Option<ConnectionObservation>), ChatWorkflowError>,
) -> Result<ProxyTransitionReceipt, ChatWorkflowError> {
    if desired_proxy_id.is_some_and(|proxy_id| proxy_id <= 0) {
        return Err(ChatWorkflowError::InvalidProxyTarget);
    }
    let (before, before_connection) = call("getProxies", get_proxies_request())?;
    let before = proxy_snapshot(before.as_value())?;
    if desired_proxy_id.is_some_and(|proxy_id| {
        before
            .proxies
            .iter()
            .all(|proxy| proxy.proxy_id != proxy_id)
    }) {
        return Err(ChatWorkflowError::InvalidProxyTarget);
    }
    let enabled_before = enabled_proxy_id(&before.proxies)?;
    if enabled_before == desired_proxy_id {
        return Ok(proxy_transition_receipt(
            desired_proxy_id,
            enabled_before,
            ProxyTransitionOutcome::AlreadyApplied,
        ));
    }
    let (method, request) = desired_proxy_id.map_or_else(
        || ("disableProxy", json!({"@type":"disableProxy"})),
        |proxy_id| {
            (
                "enableProxy",
                json!({"@type":"enableProxy","proxy_id":proxy_id}),
            )
        },
    );
    match call(method, request) {
        Ok((response, _)) => expect_ok(response, method)?,
        Err(error) if response_timed_out(&error) => {}
        Err(error) => return Err(error),
    }
    let Ok((after, after_connection)) = call("getProxies", get_proxies_request()) else {
        return Ok(proxy_transition_receipt(
            desired_proxy_id,
            enabled_before,
            ProxyTransitionOutcome::Uncertain,
        ));
    };
    let after = proxy_snapshot(after.as_value())?;
    let state_matches = enabled_proxy_id(&after.proxies)? == desired_proxy_id;
    let connection_ready = after_connection.is_some_and(|connection| {
        connection.ready
            && connection.sequence
                > before_connection
                    .map(|connection| connection.sequence)
                    .unwrap_or_default()
    });
    let outcome = match (state_matches, connection_ready) {
        (true, true) => ProxyTransitionOutcome::Verified,
        (true, false) => ProxyTransitionOutcome::ConnectivityDiverged,
        (false, _) => ProxyTransitionOutcome::Uncertain,
    };
    Ok(proxy_transition_receipt(
        desired_proxy_id,
        enabled_before,
        outcome,
    ))
}

pub(super) fn proxy_snapshot(value: &Value) -> Result<ProxySnapshot, ChatWorkflowError> {
    if value["@type"] != "addedProxies" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getProxies",
        });
    }
    let values = value["proxies"]
        .as_array()
        .ok_or(ChatWorkflowError::InvalidResult {
            method: "getProxies",
            field: "proxies",
        })?;
    let mut seen = BTreeSet::new();
    let proxies = values
        .iter()
        .map(|value| {
            if value["@type"] != "addedProxy" || value["proxy"]["@type"] != "proxy" {
                return Err(ChatWorkflowError::UnexpectedResult {
                    method: "getProxies",
                });
            }
            let proxy_id = required_i32(value, "id", "getProxies")?;
            if proxy_id <= 0 || !seen.insert(proxy_id) {
                return Err(ChatWorkflowError::InvalidResult {
                    method: "getProxies",
                    field: "proxies[].id",
                });
            }
            Ok(ProxySummary {
                proxy_id,
                is_enabled: required_bool(value, "is_enabled", "getProxies")?,
                proxy_type: required_string(&value["proxy"]["type"], "@type", "getProxies")?
                    .to_owned(),
                endpoint_redacted: true,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ProxySnapshot {
        proxies,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

fn enabled_proxy_id(proxies: &[ProxySummary]) -> Result<Option<i32>, ChatWorkflowError> {
    let mut enabled = proxies.iter().filter(|proxy| proxy.is_enabled);
    let first = enabled.next().map(|proxy| proxy.proxy_id);
    if enabled.next().is_some() {
        return Err(ChatWorkflowError::InvalidResult {
            method: "getProxies",
            field: "proxies[].is_enabled",
        });
    }
    Ok(first)
}

fn proxy_transition_receipt(
    desired_proxy_id: Option<i32>,
    rollback_proxy_id: Option<i32>,
    outcome: ProxyTransitionOutcome,
) -> ProxyTransitionReceipt {
    ProxyTransitionReceipt {
        desired_proxy_id,
        rollback_proxy_id,
        outcome,
        endpoint_redacted: true,
        complete: matches!(
            outcome,
            ProxyTransitionOutcome::AlreadyApplied | ProxyTransitionOutcome::Verified
        ),
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    }
}

fn get_proxies_request() -> Value {
    json!({"@type":"getProxies"})
}
