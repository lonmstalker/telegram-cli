//! Опциональный MCP-adapter к daemon protocol.

use serde::Serialize;
use serde_json::{Map, Value, json};
use std::process::ExitCode;
use telegram_protocol::DaemonRequest;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
}

#[derive(Debug, PartialEq)]
pub enum ToolCall {
    Daemon(DaemonRequest),
    AuthWait { challenge_id: u64, timeout_ms: u64 },
}

#[derive(Debug, PartialEq, Eq)]
pub enum AdapterError {
    UnknownTool,
    InvalidArguments,
}

fn tool(name: &'static str, description: &'static str, input_schema: Value) -> Tool {
    Tool {
        name,
        description,
        input_schema,
    }
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

pub fn tools() -> Vec<Tool> {
    let empty = || object(json!({}), &[]);
    let action = |values: &[&str]| json!({ "type": "string", "enum": values });
    let string = || json!({ "type": "string" });
    let integer = || json!({ "type": "integer", "minimum": 0 });
    vec![
        tool(
            "session",
            "Status and scoped daemon leases; this adapter never opens TDLib.",
            object(
                json!({
                    "action": action(&["status", "acquire", "heartbeat", "release"]),
                    "scopes": { "type": "array", "items": string() },
                    "ttl_ms": integer(),
                    "lease_id": string()
                }),
                &["action"],
            ),
        ),
        tool(
            "auth.begin",
            "Begin or resume brokered login metadata.",
            empty(),
        ),
        tool("auth.status", "Read brokered login metadata.", empty()),
        tool(
            "auth.wait",
            "Wait for a challenge transition; credentials are submitted outside MCP.",
            object(
                json!({ "challenge_id": integer(), "timeout_ms": integer() }),
                &["challenge_id", "timeout_ms"],
            ),
        ),
        tool(
            "schema",
            "Inspect the pinned generated TDLib registry on demand.",
            object(
                json!({
                    "action": action(&["version", "capabilities", "search", "describe"]),
                    "query": string(),
                    "name": string()
                }),
                &["action"],
            ),
        ),
        tool(
            "workflow",
            "List, describe or run a curated workflow; request constructors stay in core.",
            object(
                json!({
                    "action": action(&["list", "describe", "run"]),
                    "lease_id": string(),
                    "workflow": string(),
                    "input": { "type": "object" },
                    "approval": { "type": "object" }
                }),
                &["action"],
            ),
        ),
        tool(
            "call",
            "Preview or run one schema-validated raw TDLib request.",
            object(
                json!({
                    "action": action(&["preview", "run"]),
                    "lease_id": string(),
                    "request": { "type": "object" },
                    "approval": { "type": "object" }
                }),
                &["action", "request"],
            ),
        ),
        tool(
            "events",
            "Read ordered daemon events after an optional cursor.",
            object(
                json!({ "lease_id": string(), "after": integer() }),
                &["lease_id"],
            ),
        ),
    ]
}

fn object_arguments(value: Value) -> Result<Map<String, Value>, AdapterError> {
    value
        .as_object()
        .cloned()
        .ok_or(AdapterError::InvalidArguments)
}

fn action(arguments: &mut Map<String, Value>) -> Result<String, AdapterError> {
    arguments
        .remove("action")
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or(AdapterError::InvalidArguments)
}

fn daemon(
    mut arguments: Map<String, Value>,
    request_type: &str,
    principal: Option<&str>,
) -> Result<ToolCall, AdapterError> {
    if matches!(
        request_type,
        "session_status"
            | "login_status"
            | "schema_version"
            | "schema_capabilities"
            | "workflow_list"
    ) && !arguments.is_empty()
    {
        return Err(AdapterError::InvalidArguments);
    }
    if arguments
        .insert("type".to_owned(), json!(request_type))
        .is_some()
        || principal.is_some_and(|value| {
            arguments
                .insert("principal".to_owned(), json!(value))
                .is_some()
        })
    {
        return Err(AdapterError::InvalidArguments);
    }
    serde_json::from_value(Value::Object(arguments))
        .map(ToolCall::Daemon)
        .map_err(|_| AdapterError::InvalidArguments)
}

pub fn translate(name: &str, arguments: Value, principal: &str) -> Result<ToolCall, AdapterError> {
    let mut arguments = object_arguments(arguments)?;
    match name {
        "session" => {
            let request_type = match action(&mut arguments)?.as_str() {
                "status" => "session_status",
                "acquire" => "lease_acquire",
                "heartbeat" => "lease_heartbeat",
                "release" => "lease_release",
                _ => return Err(AdapterError::InvalidArguments),
            };
            daemon(arguments, request_type, Some(principal))
        }
        "auth.begin" | "auth.status" => daemon(arguments, "login_status", None),
        "auth.wait" => {
            let challenge_id = arguments.remove("challenge_id").and_then(|v| v.as_u64());
            let timeout_ms = arguments.remove("timeout_ms").and_then(|v| v.as_u64());
            match (challenge_id, timeout_ms, arguments.is_empty()) {
                (Some(challenge_id), Some(timeout_ms), true) => Ok(ToolCall::AuthWait {
                    challenge_id,
                    timeout_ms,
                }),
                _ => Err(AdapterError::InvalidArguments),
            }
        }
        "schema" => {
            let request_type = match action(&mut arguments)?.as_str() {
                "version" => "schema_version",
                "capabilities" => "schema_capabilities",
                "search" => "schema_search",
                "describe" => "schema_describe",
                _ => return Err(AdapterError::InvalidArguments),
            };
            daemon(arguments, request_type, None)
        }
        "workflow" => {
            let action = action(&mut arguments)?;
            let (request_type, principal) = match action.as_str() {
                "list" => ("workflow_list", None),
                "describe" => ("workflow_describe", None),
                "run" => ("workflow_run", Some(principal)),
                _ => return Err(AdapterError::InvalidArguments),
            };
            daemon(arguments, request_type, principal)
        }
        "call" => {
            let action = action(&mut arguments)?;
            let (request_type, principal) = match action.as_str() {
                "preview" => ("td_preview", None),
                "run" => ("td_call", Some(principal)),
                _ => return Err(AdapterError::InvalidArguments),
            };
            daemon(arguments, request_type, principal)
        }
        "events" => daemon(arguments, "events_watch", Some(principal)),
        _ => Err(AdapterError::UnknownTool),
    }
}

fn main() -> ExitCode {
    eprintln!("telegram-mcp: runtime ещё не реализован");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;
    use telegram_protocol::{LeaseId, RiskScope};

    #[test]
    fn exposes_small_on_demand_surface() {
        let tools = tools();
        assert_eq!(
            tools.iter().map(|tool| tool.name).collect::<Vec<_>>(),
            [
                "session",
                "auth.begin",
                "auth.status",
                "auth.wait",
                "schema",
                "workflow",
                "call",
                "events"
            ]
        );
        assert!(
            tools
                .iter()
                .all(|tool| tool.input_schema["type"] == "object")
        );
    }

    #[test]
    fn maps_tool_families_to_shared_protocol() {
        assert_eq!(
            translate(
                "schema",
                json!({"action": "search", "query": "history"}),
                "agent"
            ),
            Ok(ToolCall::Daemon(DaemonRequest::SchemaSearch {
                query: "history".to_owned()
            }))
        );
        assert_eq!(
            translate(
                "workflow",
                json!({
                    "action": "run",
                    "lease_id": "lease",
                    "workflow": "chat_history",
                    "input": {"chat_id": 7}
                }),
                "agent"
            ),
            Ok(ToolCall::Daemon(DaemonRequest::WorkflowRun {
                lease_id: LeaseId::new("lease".to_owned()),
                principal: "agent".to_owned(),
                workflow: "chat_history".to_owned(),
                input: json!({"chat_id": 7}),
                approval: None
            }))
        );
        assert_eq!(
            translate(
                "call",
                json!({
                    "action": "run",
                    "lease_id": "lease",
                    "request": {"@type": "getMe"}
                }),
                "agent"
            ),
            Ok(ToolCall::Daemon(DaemonRequest::TdCall {
                lease_id: LeaseId::new("lease".to_owned()),
                principal: "agent".to_owned(),
                request: json!({"@type": "getMe"}),
                approval: None
            }))
        );
    }

    #[test]
    fn auth_tools_accept_metadata_only() {
        assert_eq!(
            translate("auth.begin", json!({"phone_number": "canary"}), "agent"),
            Err(AdapterError::InvalidArguments)
        );
        assert_eq!(
            translate(
                "auth.wait",
                json!({"challenge_id": 42, "timeout_ms": 1_000}),
                "agent"
            ),
            Ok(ToolCall::AuthWait {
                challenge_id: 42,
                timeout_ms: 1_000
            })
        );
    }

    #[test]
    fn principal_comes_from_transport_context() {
        assert_eq!(
            translate(
                "session",
                json!({
                    "action": "acquire",
                    "scopes": ["read"],
                    "ttl_ms": 10_000,
                    "principal": "forged"
                }),
                "authenticated"
            ),
            Err(AdapterError::InvalidArguments)
        );
        assert_eq!(
            translate(
                "session",
                json!({"action": "acquire", "scopes": ["read"], "ttl_ms": 10_000}),
                "authenticated"
            ),
            Ok(ToolCall::Daemon(DaemonRequest::LeaseAcquire {
                principal: "authenticated".to_owned(),
                scopes: vec![RiskScope::Read],
                ttl_ms: 10_000
            }))
        );
    }
}
