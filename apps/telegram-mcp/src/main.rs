//! Опциональный MCP-adapter к daemon protocol.

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorCode, Implementation, ListToolsResult,
    PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::time::{Duration, Instant};
use telegram_protocol::{DaemonRequest, DaemonResponse, MachineEnvelope, MachineStatus, RiskScope};

const AUTH_WAIT_MAX_MS: u64 = 60_000;
const IO_TIMEOUT: Duration = Duration::from_secs(35);
const SSH_POLICY_DIR: &str = "/etc/telegram-cli/mcp-ssh";
const USAGE: &str = "telegram-mcp: usage: telegram-mcp stdio | ssh-stdio <identity>";

#[derive(Debug, PartialEq)]
enum ToolCall {
    Daemon(DaemonRequest),
    AuthWait { challenge_id: u64, timeout_ms: u64 },
}

#[derive(Debug, PartialEq, Eq)]
enum AdapterError {
    UnknownTool,
    InvalidArguments,
}

fn tool(name: &'static str, description: &'static str, input_schema: Value) -> Tool {
    let Value::Object(input_schema) = input_schema else {
        unreachable!("fixed MCP tool schemas are objects")
    };
    Tool::new(name, description, input_schema)
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn tools() -> Vec<Tool> {
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
                json!({
                    "challenge_id": integer(),
                    "timeout_ms": { "type": "integer", "minimum": 0, "maximum": AUTH_WAIT_MAX_MS }
                }),
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

fn translate(name: &str, arguments: Value, principal: &str) -> Result<ToolCall, AdapterError> {
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
                (Some(challenge_id), Some(timeout_ms), true) if timeout_ms <= AUTH_WAIT_MAX_MS => {
                    Ok(ToolCall::AuthWait {
                        challenge_id,
                        timeout_ms,
                    })
                }
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

#[derive(Clone)]
struct TransportIdentity {
    profile: String,
    principal: String,
    scopes: Vec<RiskScope>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SshPolicy {
    profile: String,
    scopes: Vec<RiskScope>,
}

#[derive(Clone)]
struct TelegramMcp {
    identity: TransportIdentity,
}

impl TelegramMcp {
    fn permits(&self, call: &ToolCall) -> bool {
        match call {
            ToolCall::Daemon(DaemonRequest::LeaseAcquire { scopes, .. }) => scopes
                .iter()
                .all(|scope| self.identity.scopes.contains(scope)),
            _ => true,
        }
    }

    async fn exchange(&self, request: DaemonRequest) -> Result<DaemonResponse, ()> {
        let profile = self.identity.profile.clone();
        tokio::task::spawn_blocking(move || daemon_exchange(&profile, &request))
            .await
            .map_err(|_| ())?
    }

    async fn execute(&self, call: ToolCall) -> Result<DaemonResponse, ()> {
        match call {
            ToolCall::Daemon(request) => self.exchange(request).await,
            ToolCall::AuthWait {
                challenge_id,
                timeout_ms,
            } => {
                let deadline = Instant::now() + Duration::from_millis(timeout_ms);
                loop {
                    let response = self.exchange(DaemonRequest::LoginStatus).await?;
                    let unchanged = matches!(
                        response,
                        DaemonResponse::LoginStatus {
                            challenge_id: Some(current),
                            ..
                        } if current == challenge_id
                    );
                    if !unchanged || Instant::now() >= deadline {
                        return Ok(response);
                    }
                    tokio::time::sleep(
                        Duration::from_millis(200)
                            .min(deadline.saturating_duration_since(Instant::now())),
                    )
                    .await;
                }
            }
        }
    }
}

impl ServerHandler for TelegramMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
            .with_server_info(Implementation::new(
                "telegram-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Prefer workflow describe/run; use schema and raw call only as an on-demand fallback.",
            )
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tools().into_iter().find(|tool| tool.name == name)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let call = match translate(
            &request.name,
            Value::Object(request.arguments.unwrap_or_default()),
            &self.identity.principal,
        ) {
            Ok(call) => call,
            Err(AdapterError::InvalidArguments) => {
                return Ok(tool_error("invalid_arguments"));
            }
            Err(AdapterError::UnknownTool) => {
                return Err(ErrorData::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    "unknown tool",
                    None,
                ));
            }
        };
        if !self.permits(&call) {
            return Ok(tool_error("scope_denied"));
        }
        match self.execute(call).await {
            Ok(response) => Ok(tool_response(response)),
            Err(()) => Ok(tool_error("daemon_unavailable")),
        }
    }
}

fn tool_response(response: DaemonResponse) -> CallToolResult {
    let envelope = MachineEnvelope::from_response(response);
    let is_error = envelope.status() == MachineStatus::Error;
    match serde_json::to_value(envelope) {
        Ok(value) if is_error => CallToolResult::structured_error(value),
        Ok(value) => CallToolResult::structured(value),
        Err(_) => tool_error("response_serialization_failed"),
    }
}

fn tool_error(code: &str) -> CallToolResult {
    CallToolResult::structured_error(json!({ "error": code }))
}

fn daemon_exchange(profile: &str, request: &DaemonRequest) -> Result<DaemonResponse, ()> {
    let path = socket_path(profile)?;
    validate_socket(&path)?;
    let mut stream = UnixStream::connect(path).map_err(|_| ())?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|_| stream.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|_| ())?;
    serde_json::to_writer(&mut stream, request).map_err(|_| ())?;
    stream
        .write_all(b"\n")
        .and_then(|_| stream.flush())
        .map_err(|_| ())?;
    serde_json::from_reader(BufReader::new(stream)).map_err(|_| ())
}

fn socket_path(profile: &str) -> Result<PathBuf, ()> {
    if !valid_name(profile) {
        return Err(());
    }
    Ok(PathBuf::from(format!(
        "/tmp/telegramd-{}/{profile}.sock",
        effective_uid()
    )))
}

fn validate_socket(path: &Path) -> Result<(), ()> {
    let parent = path.parent().ok_or(())?;
    let directory = fs::symlink_metadata(parent).map_err(|_| ())?;
    let socket = fs::symlink_metadata(path).map_err(|_| ())?;
    let uid = effective_uid();
    if !directory.is_dir()
        || directory.uid() != uid
        || directory.mode() & 0o777 != 0o700
        || !socket.file_type().is_socket()
        || socket.uid() != uid
        || socket.nlink() != 1
        || socket.mode() & 0o777 != 0o600
    {
        return Err(());
    }
    Ok(())
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn effective_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and does not access memory.
    unsafe { libc::geteuid() }
}

fn parse_scopes(value: &str) -> Result<Vec<RiskScope>, ()> {
    let value = if value.is_empty() { "read" } else { value };
    let mut scopes = value
        .split(',')
        .map(RiskScope::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;
    scopes.sort_unstable();
    scopes.dedup();
    (!scopes.is_empty()).then_some(scopes).ok_or(())
}

fn local_identity() -> Result<TransportIdentity, ()> {
    let profile = env::var("TELEGRAM_PROFILE").unwrap_or_else(|_| "default".to_owned());
    if !valid_name(&profile) {
        return Err(());
    }
    let scopes = match env::var("TELEGRAM_MCP_SCOPES") {
        Ok(value) => parse_scopes(&value)?,
        Err(env::VarError::NotPresent) => vec![RiskScope::Read],
        Err(env::VarError::NotUnicode(_)) => return Err(()),
    };
    Ok(TransportIdentity {
        profile,
        principal: format!("local:{}", effective_uid()),
        scopes,
    })
}

fn ssh_identity(identity: &str) -> Result<TransportIdentity, ()> {
    if !valid_name(identity) || env::var_os("SSH_CONNECTION").is_none_or(|value| value.is_empty()) {
        return Err(());
    }
    let path = Path::new(SSH_POLICY_DIR).join(format!("{identity}.json"));
    let policy = load_ssh_policy(&path, 0)?;
    Ok(TransportIdentity {
        profile: policy.profile,
        principal: format!("ssh:{identity}"),
        scopes: policy.scopes,
    })
}

fn load_ssh_policy(path: &Path, owner_uid: u32) -> Result<SshPolicy, ()> {
    let parent = path.parent().ok_or(())?;
    let directory = fs::symlink_metadata(parent).map_err(|_| ())?;
    let file = fs::symlink_metadata(path).map_err(|_| ())?;
    if !directory.is_dir()
        || directory.uid() != owner_uid
        || directory.mode() & 0o777 != 0o755
        || !file.file_type().is_file()
        || file.uid() != owner_uid
        || file.nlink() != 1
        || file.mode() & 0o777 != 0o644
    {
        return Err(());
    }
    let mut policy: SshPolicy =
        serde_json::from_reader(File::open(path).map_err(|_| ())?).map_err(|_| ())?;
    if !valid_name(&policy.profile) || policy.scopes.is_empty() {
        return Err(());
    }
    policy.scopes.sort_unstable();
    policy.scopes.dedup();
    Ok(policy)
}

async fn serve(identity: TransportIdentity) -> Result<(), ()> {
    let service = TelegramMcp { identity }
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|_| ())?;
    service.waiting().await.map_err(|_| ())?;
    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let identity = match arguments.as_slice() {
        [mode] if mode == "stdio" => local_identity(),
        [mode, identity] if mode == "ssh-stdio" => ssh_identity(identity),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let Ok(identity) = identity else {
        eprintln!("telegram-mcp: transport identity rejected");
        return ExitCode::FAILURE;
    };
    match serve(identity).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => {
            eprintln!("telegram-mcp: transport failed");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use telegram_protocol::{LeaseId, RiskScope};

    #[test]
    fn exposes_small_on_demand_surface() {
        let tools = tools();
        assert_eq!(
            tools
                .iter()
                .map(|tool| tool.name.as_ref())
                .collect::<Vec<_>>(),
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

    #[test]
    fn transport_scope_caps_lease_requests() {
        let server = TelegramMcp {
            identity: TransportIdentity {
                profile: "default".to_owned(),
                principal: "ssh:reader".to_owned(),
                scopes: vec![RiskScope::Read],
            },
        };
        let read = translate(
            "session",
            json!({"action": "acquire", "scopes": ["read"], "ttl_ms": 1_000}),
            "ssh:reader",
        )
        .unwrap();
        let send = translate(
            "session",
            json!({"action": "acquire", "scopes": ["send"], "ttl_ms": 1_000}),
            "ssh:reader",
        )
        .unwrap();
        assert!(server.permits(&read));
        assert!(!server.permits(&send));
        assert_eq!(
            server.get_info().protocol_version,
            ProtocolVersion::V_2025_11_25
        );
    }

    #[test]
    fn ssh_policy_requires_safe_operator_owned_files() {
        let directory = env::temp_dir().join(format!(
            "telegram-mcp-policy-{}-{}",
            std::process::id(),
            effective_uid()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
        let policy_path = directory.join("reader.json");
        fs::write(
            &policy_path,
            br#"{"profile":"default","scopes":["read","read"]}"#,
        )
        .unwrap();
        fs::set_permissions(&policy_path, fs::Permissions::from_mode(0o644)).unwrap();

        let policy = load_ssh_policy(&policy_path, effective_uid()).unwrap();
        assert_eq!(policy.profile, "default");
        assert_eq!(policy.scopes, [RiskScope::Read]);

        fs::set_permissions(&policy_path, fs::Permissions::from_mode(0o664)).unwrap();
        assert!(load_ssh_policy(&policy_path, effective_uid()).is_err());
        fs::set_permissions(&policy_path, fs::Permissions::from_mode(0o644)).unwrap();
        let symlink_path = directory.join("linked.json");
        symlink(&policy_path, &symlink_path).unwrap();
        assert!(load_ssh_policy(&symlink_path, effective_uid()).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}
