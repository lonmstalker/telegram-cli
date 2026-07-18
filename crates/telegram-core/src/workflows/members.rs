//! members workflows.

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MembersQuery {
    pub supergroup_id: i64,
    pub count: usize,
    pub page_limit: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MembersSnapshot {
    pub members: Vec<Value>,
    pub pages: usize,
    pub total_count: i32,
    pub boundary: PageBoundary,
    pub complete: bool,
    pub capability_sequence: u64,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StatisticsSnapshot {
    pub statistics: Value,
    pub graph_lineage: BTreeMap<String, Vec<String>>,
    pub unresolved_tokens: Vec<String>,
    pub complete: bool,
    pub capability_sequence: u64,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResourceStatisticsSnapshot {
    pub files_size: i64,
    pub file_count: i32,
    pub database_size: i64,
    pub language_pack_database_size: i64,
    pub log_size: i64,
    pub network_since_date: i32,
    pub sent_bytes: i64,
    pub received_bytes: i64,
    pub database_report_redacted: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

pub fn supergroup_members(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: MembersQuery,
    deadline: Instant,
) -> Result<MembersSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    let capability = runtime
        .state()
        .supergroup_full_info(query.supergroup_id)
        .ok_or(ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "supergroupFullInfo",
        })?;
    let allowed =
        capability.value["can_get_members"]
            .as_bool()
            .ok_or(ChatWorkflowError::InvalidResult {
                method: "supergroupFullInfo",
                field: "can_get_members",
            })?;
    supergroup_members_with(query, allowed, capability.sequence.get(), |request| {
        invoke(runtime, policy, "getSupergroupMembers", request, deadline)
    })
}

pub fn chat_statistics(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    is_dark: bool,
    deadline: Instant,
) -> Result<StatisticsSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    let chat = runtime
        .state()
        .chat(chat_id)
        .ok_or(ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "chat",
        })?;
    let supergroup_id = match chat.value["type"]["@type"].as_str() {
        Some("chatTypeSupergroup") => required_i64(&chat.value["type"], "supergroup_id", "chat")?,
        _ => {
            return Err(ChatWorkflowError::CapabilityDenied {
                capability: "can_get_statistics",
            });
        }
    };
    let capability = runtime.state().supergroup_full_info(supergroup_id).ok_or(
        ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "supergroupFullInfo",
        },
    )?;
    let allowed = capability.value["can_get_statistics"].as_bool().ok_or(
        ChatWorkflowError::InvalidResult {
            method: "supergroupFullInfo",
            field: "can_get_statistics",
        },
    )?;
    chat_statistics_with(
        chat_id,
        is_dark,
        allowed,
        capability.sequence.get(),
        |method, request| invoke(runtime, policy, method, request, deadline),
    )
}

pub fn resource_statistics(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    only_current_network: bool,
    deadline: Instant,
) -> Result<ResourceStatisticsSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    resource_statistics_with(only_current_network, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub(super) fn supergroup_members_with(
    query: MembersQuery,
    allowed: bool,
    capability_sequence: u64,
    mut call: impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<MembersSnapshot, ChatWorkflowError> {
    if !allowed {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "can_get_members",
        });
    }
    if query.count == 0 || !(1..=200).contains(&query.page_limit) {
        return Err(ChatWorkflowError::InvalidPageOptions);
    }

    let mut offset = 0_i32;
    let mut pages = 0;
    let mut total_count: i32;
    let mut members = Vec::new();
    let mut seen = BTreeSet::new();
    let boundary = loop {
        let response = call(json!({
            "@type":"getSupergroupMembers",
            "supergroup_id":query.supergroup_id,
            "filter":null,
            "offset":offset,
            "limit":query.page_limit
        }))?;
        pages += 1;
        if response.as_value()["@type"] != "chatMembers" {
            return Err(ChatWorkflowError::UnexpectedResult {
                method: "getSupergroupMembers",
            });
        }
        total_count = i32::try_from(required_i64(
            response.as_value(),
            "total_count",
            "getSupergroupMembers",
        )?)
        .map_err(|_| ChatWorkflowError::InvalidResult {
            method: "getSupergroupMembers",
            field: "total_count",
        })?;
        let page =
            response.as_value()["members"]
                .as_array()
                .ok_or(ChatWorkflowError::InvalidResult {
                    method: "getSupergroupMembers",
                    field: "members",
                })?;
        for member in page {
            if member["@type"] != "chatMember" {
                return Err(ChatWorkflowError::InvalidResult {
                    method: "getSupergroupMembers",
                    field: "members[].@type",
                });
            }
            let key = serde_json::to_string(&member["member_id"]).map_err(|_| {
                ChatWorkflowError::InvalidResult {
                    method: "getSupergroupMembers",
                    field: "members[].member_id",
                }
            })?;
            if seen.insert(key) {
                members.push(member.clone());
                if members.len() == query.count {
                    break;
                }
            }
        }
        offset = offset
            .checked_add(i32::try_from(page.len()).map_err(|_| {
                ChatWorkflowError::InvalidResult {
                    method: "getSupergroupMembers",
                    field: "members",
                }
            })?)
            .ok_or(ChatWorkflowError::InvalidPageOptions)?;
        if members.len() == query.count {
            break PageBoundary::Count;
        }
        if offset >= total_count {
            break PageBoundary::Exhausted;
        }
        if page.is_empty() {
            break PageBoundary::NoProgress;
        }
    };
    Ok(MembersSnapshot {
        members,
        pages,
        total_count,
        boundary,
        complete: boundary != PageBoundary::NoProgress,
        capability_sequence,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

pub(super) fn chat_statistics_with(
    chat_id: i64,
    is_dark: bool,
    allowed: bool,
    capability_sequence: u64,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StatisticsSnapshot, ChatWorkflowError> {
    if !allowed {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "can_get_statistics",
        });
    }
    let mut statistics = call(
        "getChatStatistics",
        json!({"@type":"getChatStatistics","chat_id":chat_id,"is_dark":is_dark}),
    )?
    .into_value();
    if !matches!(
        statistics["@type"].as_str(),
        Some("chatStatisticsSupergroup" | "chatStatisticsChannel")
    ) {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getChatStatistics",
        });
    }

    let mut initial_tokens = BTreeSet::new();
    collect_async_tokens(&statistics, &mut initial_tokens)?;
    let mut resolved = BTreeMap::<String, Value>::new();
    let mut unresolved = BTreeSet::new();
    let mut graph_lineage = BTreeMap::new();
    for initial in initial_tokens {
        let mut token = initial.clone();
        let mut lineage = Vec::new();
        let terminal = loop {
            if let Some(graph) = resolved.get(&token) {
                break Some(graph.clone());
            }
            if lineage.contains(&token) {
                break None;
            }
            lineage.push(token.clone());
            let graph = match call(
                "getStatisticalGraph",
                json!({"@type":"getStatisticalGraph","chat_id":chat_id,"token":token,"x":0}),
            ) {
                Ok(graph) => graph.into_value(),
                Err(ChatWorkflowError::Call(RawApiError::Transport(
                    TransportError::ResponseTimeout,
                ))) => break None,
                Err(error) => return Err(error),
            };
            match graph["@type"].as_str() {
                Some("statisticalGraphData" | "statisticalGraphError") => {
                    for seen in &lineage {
                        resolved.insert(seen.clone(), graph.clone());
                    }
                    break Some(graph);
                }
                Some("statisticalGraphAsync") => {
                    token = required_string(&graph, "token", "getStatisticalGraph")?.to_owned();
                }
                _ => {
                    return Err(ChatWorkflowError::UnexpectedResult {
                        method: "getStatisticalGraph",
                    });
                }
            }
        };
        if let Some(graph) = terminal {
            replace_async_graph(&mut statistics, &initial, &graph);
        } else {
            unresolved.insert(initial.clone());
        }
        graph_lineage.insert(initial, lineage);
    }
    Ok(StatisticsSnapshot {
        statistics,
        graph_lineage,
        complete: unresolved.is_empty(),
        unresolved_tokens: unresolved.into_iter().collect(),
        capability_sequence,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

pub(super) fn resource_statistics_with(
    only_current_network: bool,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<ResourceStatisticsSnapshot, ChatWorkflowError> {
    let storage = call(
        "getStorageStatisticsFast",
        json!({"@type":"getStorageStatisticsFast"}),
    )?;
    if storage.as_value()["@type"] != "storageStatisticsFast" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getStorageStatisticsFast",
        });
    }
    let database = call(
        "getDatabaseStatistics",
        json!({"@type":"getDatabaseStatistics"}),
    )?;
    if database.as_value()["@type"] != "databaseStatistics" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getDatabaseStatistics",
        });
    }
    let _ = required_string(database.as_value(), "statistics", "getDatabaseStatistics")?;
    let network = call(
        "getNetworkStatistics",
        json!({"@type":"getNetworkStatistics","only_current":only_current_network}),
    )?;
    if network.as_value()["@type"] != "networkStatistics" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getNetworkStatistics",
        });
    }
    let entries =
        network.as_value()["entries"]
            .as_array()
            .ok_or(ChatWorkflowError::InvalidResult {
                method: "getNetworkStatistics",
                field: "entries",
            })?;
    let mut sent_bytes = 0_i64;
    let mut received_bytes = 0_i64;
    for entry in entries {
        if !matches!(
            entry["@type"].as_str(),
            Some("networkStatisticsEntryFile" | "networkStatisticsEntryCall")
        ) {
            return Err(ChatWorkflowError::UnexpectedResult {
                method: "getNetworkStatistics",
            });
        }
        let sent = required_i64(entry, "sent_bytes", "getNetworkStatistics")?;
        let received = required_i64(entry, "received_bytes", "getNetworkStatistics")?;
        if sent < 0 || received < 0 {
            return Err(ChatWorkflowError::InvalidResult {
                method: "getNetworkStatistics",
                field: "entries",
            });
        }
        sent_bytes = sent_bytes
            .checked_add(sent)
            .ok_or(ChatWorkflowError::InvalidResult {
                method: "getNetworkStatistics",
                field: "sent_bytes",
            })?;
        received_bytes =
            received_bytes
                .checked_add(received)
                .ok_or(ChatWorkflowError::InvalidResult {
                    method: "getNetworkStatistics",
                    field: "received_bytes",
                })?;
    }
    Ok(ResourceStatisticsSnapshot {
        files_size: required_i64(storage.as_value(), "files_size", "getStorageStatisticsFast")?,
        file_count: required_i32(storage.as_value(), "file_count", "getStorageStatisticsFast")?,
        database_size: required_i64(
            storage.as_value(),
            "database_size",
            "getStorageStatisticsFast",
        )?,
        language_pack_database_size: required_i64(
            storage.as_value(),
            "language_pack_database_size",
            "getStorageStatisticsFast",
        )?,
        log_size: required_i64(storage.as_value(), "log_size", "getStorageStatisticsFast")?,
        network_since_date: required_i32(network.as_value(), "since_date", "getNetworkStatistics")?,
        sent_bytes,
        received_bytes,
        database_report_redacted: true,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

fn collect_async_tokens(
    value: &Value,
    tokens: &mut BTreeSet<String>,
) -> Result<(), ChatWorkflowError> {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_async_tokens(value, tokens)?;
            }
        }
        Value::Object(object)
            if object.get("@type").and_then(Value::as_str) == Some("statisticalGraphAsync") =>
        {
            tokens.insert(required_string(value, "token", "getChatStatistics")?.to_owned());
        }
        Value::Object(object) => {
            for value in object.values() {
                collect_async_tokens(value, tokens)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn replace_async_graph(value: &mut Value, token: &str, graph: &Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                replace_async_graph(value, token, graph);
            }
        }
        Value::Object(object)
            if object.get("@type").and_then(Value::as_str) == Some("statisticalGraphAsync")
                && object.get("token").and_then(Value::as_str) == Some(token) =>
        {
            *value = graph.clone();
        }
        Value::Object(object) => {
            for value in object.values_mut() {
                replace_async_graph(value, token, graph);
            }
        }
        _ => {}
    }
}
