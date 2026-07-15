//! Bounded JSONL lease protocol поверх private profile socket.

use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use telegram_core::raw_api::{
    self, PolicyError, RawApiError, SchemaDescription, SchemaSearchResult,
};
use telegram_core::registry::{AccountKind, SymbolKind};
use telegram_core::runtime::CoreRuntime;
use telegram_protocol::{CommandErrorCode, DaemonRequest, DaemonResponse, LeaseErrorCode};

use crate::lease::LeaseManager;

const MAX_REQUEST_BYTES: u64 = 16 * 1024;
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(5);
const CALL_TIMEOUT: Duration = Duration::from_secs(30);

pub struct LeaseServer {
    leases: LeaseManager,
}

impl LeaseServer {
    pub fn new(leases: LeaseManager) -> Self {
        Self { leases }
    }

    pub fn poll(
        &mut self,
        listener: &UnixListener,
        runtime: &CoreRuntime,
        now: Instant,
    ) -> Result<(), ServerError> {
        self.leases.expire(now);
        loop {
            match self.serve_once(listener, Some(runtime)) {
                Ok(()) => {}
                Err(ServerError::Accept(io::ErrorKind::WouldBlock)) => return Ok(()),
                Err(error @ ServerError::Accept(_)) => return Err(error),
                Err(ServerError::ClientIo(_) | ServerError::SerializeResponse) => {}
            }
        }
    }

    pub fn active_leases(&self) -> usize {
        self.leases.active_count()
    }

    fn serve_once(
        &mut self,
        listener: &UnixListener,
        runtime: Option<&CoreRuntime>,
    ) -> Result<(), ServerError> {
        let (stream, _) = listener
            .accept()
            .map_err(|error| ServerError::Accept(error.kind()))?;
        stream
            .set_nonblocking(false)
            .map_err(|error| ServerError::ClientIo(error.kind()))?;
        self.serve_connection(stream, runtime)
    }

    fn serve_connection(
        &mut self,
        mut stream: UnixStream,
        runtime: Option<&CoreRuntime>,
    ) -> Result<(), ServerError> {
        stream
            .set_read_timeout(Some(CLIENT_IO_TIMEOUT))
            .map_err(|error| ServerError::ClientIo(error.kind()))?;
        stream
            .set_write_timeout(Some(CLIENT_IO_TIMEOUT))
            .map_err(|error| ServerError::ClientIo(error.kind()))?;
        let mut bytes = Vec::new();
        {
            let reader = BufReader::new(&mut stream);
            let mut limited = reader.take(MAX_REQUEST_BYTES + 1);
            limited
                .read_until(b'\n', &mut bytes)
                .map_err(|error| ServerError::ClientIo(error.kind()))?;
        }
        let response = if bytes.is_empty()
            || bytes.len() as u64 > MAX_REQUEST_BYTES
            || !bytes.ends_with(b"\n")
        {
            DaemonResponse::Error {
                code: LeaseErrorCode::InvalidRequest,
            }
        } else {
            bytes.pop();
            if bytes.ends_with(b"\r") {
                bytes.pop();
            }
            match serde_json::from_slice(&bytes) {
                Ok(request) => self.handle(request, runtime, Instant::now()),
                Err(_) => DaemonResponse::Error {
                    code: LeaseErrorCode::InvalidRequest,
                },
            }
        };
        serde_json::to_writer(&mut stream, &response)
            .map_err(|_| ServerError::SerializeResponse)?;
        stream
            .write_all(b"\n")
            .and_then(|_| stream.flush())
            .map_err(|error| ServerError::ClientIo(error.kind()))
    }

    fn handle(
        &mut self,
        request: DaemonRequest,
        runtime: Option<&CoreRuntime>,
        now: Instant,
    ) -> DaemonResponse {
        match request {
            DaemonRequest::SessionStatus => DaemonResponse::SessionStatus {
                active_leases: self.leases.active_count(),
            },
            DaemonRequest::SchemaVersion => runtime.map_or_else(runtime_unavailable, |runtime| {
                DaemonResponse::SchemaVersion {
                    version: serde_json::to_value(raw_api::version(runtime))
                        .expect("version descriptor is serializable"),
                }
            }),
            DaemonRequest::SchemaCapabilities => DaemonResponse::SchemaCapabilities {
                capabilities: serde_json::to_value(raw_api::capabilities())
                    .expect("capability descriptors are serializable"),
            },
            DaemonRequest::SchemaSearch { query } => DaemonResponse::SchemaSearchResults {
                results: Value::Array(
                    raw_api::schema_search(&query)
                        .into_iter()
                        .map(search_result)
                        .collect(),
                ),
            },
            DaemonRequest::SchemaDescribe { name } => raw_api::schema_describe(&name).map_or_else(
                || DaemonResponse::CommandError {
                    code: CommandErrorCode::SchemaNotFound,
                },
                |description| DaemonResponse::SchemaDescription {
                    description: describe(description),
                },
            ),
            DaemonRequest::TdCall {
                lease_id,
                principal,
                request,
            } => {
                let policy = match self.leases.raw_policy(
                    &lease_id,
                    &principal,
                    AccountKind::RegularUser,
                    now,
                ) {
                    Ok(policy) => policy,
                    Err(code) => return DaemonResponse::Error { code },
                };
                let Some(runtime) = runtime else {
                    return runtime_unavailable();
                };
                let deadline = now.checked_add(CALL_TIMEOUT).unwrap_or(now);
                match raw_api::td_call(runtime, &policy, request, deadline) {
                    Ok(result) => DaemonResponse::TdResult {
                        result: result.into_value(),
                    },
                    Err(error) => DaemonResponse::CommandError {
                        code: raw_error(error),
                    },
                }
            }
            DaemonRequest::LeaseAcquire {
                principal,
                scopes,
                ttl_ms,
            } => match self.leases.acquire(principal, scopes, ttl_ms, now) {
                Ok(lease) => DaemonResponse::LeaseGranted { lease },
                Err(code) => DaemonResponse::Error { code },
            },
            DaemonRequest::LeaseHeartbeat {
                lease_id,
                principal,
            } => match self.leases.heartbeat(&lease_id, &principal, now) {
                Ok(lease) => DaemonResponse::LeaseRenewed { lease },
                Err(code) => DaemonResponse::Error { code },
            },
            DaemonRequest::LeaseRelease {
                lease_id,
                principal,
            } => match self.leases.release(&lease_id, &principal, now) {
                Ok(()) => DaemonResponse::LeaseReleased { lease_id },
                Err(code) => DaemonResponse::Error { code },
            },
        }
    }
}

fn runtime_unavailable() -> DaemonResponse {
    DaemonResponse::CommandError {
        code: CommandErrorCode::RuntimeUnavailable,
    }
}

fn search_result(result: SchemaSearchResult) -> Value {
    match result {
        SchemaSearchResult::Symbol(symbol) => json!({
            "kind": symbol_kind(symbol.kind),
            "name": symbol.name,
            "result": symbol.result.name,
        }),
        SchemaSearchResult::Type(name) => json!({"kind": "type", "name": name}),
    }
}

fn describe(description: SchemaDescription) -> Value {
    match description {
        SchemaDescription::Symbol(symbol) => {
            serde_json::to_value(symbol).expect("symbol descriptor is serializable")
        }
        SchemaDescription::Type { name, constructors } => json!({
            "kind": "type",
            "name": name,
            "constructors": constructors,
        }),
    }
}

fn symbol_kind(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Builtin => "builtin",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Method => "method",
    }
}

fn raw_error(error: RawApiError) -> CommandErrorCode {
    match error {
        RawApiError::Validation(_) => CommandErrorCode::InvalidTdjson,
        RawApiError::Policy(PolicyError::DefaultDeny) => CommandErrorCode::MethodDefaultDenied,
        RawApiError::Policy(PolicyError::AccountScopeDenied) => {
            CommandErrorCode::AccountScopeDenied
        }
        RawApiError::Policy(PolicyError::RiskDenied { .. }) => CommandErrorCode::RiskScopeDenied,
        RawApiError::Policy(PolicyError::ApprovalRequired { .. }) => {
            CommandErrorCode::ApprovalRequired
        }
        RawApiError::Policy(PolicyError::ApprovalDenied { .. }) => CommandErrorCode::ApprovalDenied,
        RawApiError::Transport(_) => CommandErrorCode::TdlibTransport,
        RawApiError::UnexpectedResult { .. } => CommandErrorCode::UnexpectedTdlibResult,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerError {
    Accept(io::ErrorKind),
    ClientIo(io::ErrorKind),
    SerializeResponse,
}

impl fmt::Display for ServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accept(kind) => write!(formatter, "profile socket accept failed: {kind:?}"),
            Self::ClientIo(kind) => write!(formatter, "profile client IO failed: {kind:?}"),
            Self::SerializeResponse => formatter.write_str("lease response serialization failed"),
        }
    }
}

impl std::error::Error for ServerError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use telegram_protocol::{LeaseView, RiskScope};

    use crate::ownership::ProfileDatabaseLock;
    use crate::socket::DaemonSocket;

    use super::*;

    #[test]
    fn jsonl_protocol_acquires_heartbeats_and_releases_lease() {
        let (root, profile) = temporary_scope();
        fs::create_dir_all(&root).unwrap();
        let ownership = ProfileDatabaseLock::acquire(profile, &root).unwrap();
        let socket = DaemonSocket::bind(&ownership).unwrap();
        socket.listener().set_nonblocking(true).unwrap();
        let mut server = LeaseServer::new(LeaseManager::default());

        let granted = exchange(
            &mut server,
            &socket,
            DaemonRequest::LeaseAcquire {
                principal: "agent".to_owned(),
                scopes: vec![RiskScope::Read],
                ttl_ms: 1_000,
            },
        );
        let DaemonResponse::LeaseGranted {
            lease: LeaseView { lease_id, .. },
        } = granted
        else {
            panic!("expected granted lease")
        };
        assert!(matches!(
            exchange(
                &mut server,
                &socket,
                DaemonRequest::LeaseHeartbeat {
                    lease_id: lease_id.clone(),
                    principal: "agent".to_owned(),
                }
            ),
            DaemonResponse::LeaseRenewed { .. }
        ));
        assert_eq!(
            exchange(
                &mut server,
                &socket,
                DaemonRequest::LeaseRelease {
                    lease_id: lease_id.clone(),
                    principal: "agent".to_owned(),
                }
            ),
            DaemonResponse::LeaseReleased { lease_id }
        );
        assert_eq!(
            exchange(&mut server, &socket, DaemonRequest::SessionStatus),
            DaemonResponse::SessionStatus { active_leases: 0 }
        );
        let DaemonResponse::SchemaSearchResults { results } = exchange(
            &mut server,
            &socket,
            DaemonRequest::SchemaSearch {
                query: "chat statistics".to_owned(),
            },
        ) else {
            panic!("expected schema search results")
        };
        assert!(results.as_array().unwrap().iter().any(|result| {
            result.get("name").and_then(Value::as_str) == Some("getChatStatistics")
        }));
        assert_eq!(
            exchange(&mut server, &socket, DaemonRequest::SchemaVersion),
            DaemonResponse::CommandError {
                code: CommandErrorCode::RuntimeUnavailable,
            }
        );
        assert_eq!(server.leases.active_count(), 0);

        drop(socket);
        drop(ownership);
        fs::remove_dir_all(root).unwrap();
    }

    fn exchange(
        server: &mut LeaseServer,
        socket: &DaemonSocket,
        request: DaemonRequest,
    ) -> DaemonResponse {
        let mut client = UnixStream::connect(socket.path()).unwrap();
        serde_json::to_writer(&mut client, &request).unwrap();
        client.write_all(b"\n").unwrap();
        server.serve_once(socket.listener(), None).unwrap();
        let mut response = String::new();
        BufReader::new(client).read_line(&mut response).unwrap();
        serde_json::from_str(&response).unwrap()
    }

    fn temporary_scope() -> (std::path::PathBuf, String) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let profile = format!("lease-{}-{nonce:x}", std::process::id());
        (
            std::env::temp_dir().join(format!("telegramd-{profile}")),
            profile,
        )
    }
}
