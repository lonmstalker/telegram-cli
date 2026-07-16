//! Стабильные wire-контракты клиентов и daemon.

#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;
use std::str::FromStr;
use zeroize::Zeroize;

pub const MACHINE_PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskScope {
    Read,
    Presence,
    Send,
    ReversibleMutation,
    Admin,
    Destructive,
    Financial,
    AuthSecurity,
}

impl RiskScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Presence => "presence",
            Self::Send => "send",
            Self::ReversibleMutation => "reversible_mutation",
            Self::Admin => "admin",
            Self::Destructive => "destructive",
            Self::Financial => "financial",
            Self::AuthSecurity => "auth_security",
        }
    }
}

impl FromStr for RiskScope {
    type Err = ParseRiskScopeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "read" => Ok(Self::Read),
            "presence" => Ok(Self::Presence),
            "send" => Ok(Self::Send),
            "reversible_mutation" => Ok(Self::ReversibleMutation),
            "admin" => Ok(Self::Admin),
            "destructive" => Ok(Self::Destructive),
            "financial" => Ok(Self::Financial),
            "auth_security" => Ok(Self::AuthSecurity),
            _ => Err(ParseRiskScopeError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseRiskScopeError;

impl fmt::Display for ParseRiskScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown risk scope")
    }
}

impl std::error::Error for ParseRiskScopeError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LeaseId(String);

impl LeaseId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DaemonRequest {
    SessionStatus,
    LoginStatus,
    LoginSubmit {
        challenge_id: u64,
        input: LoginInput,
    },
    SchemaVersion,
    SchemaCapabilities,
    SchemaSearch {
        query: String,
    },
    SchemaDescribe {
        name: String,
    },
    TdPreview {
        request: Value,
    },
    TdCall {
        lease_id: LeaseId,
        principal: String,
        request: Value,
        approval: Option<PlanApproval>,
    },
    WorkflowList,
    WorkflowDescribe {
        workflow: String,
    },
    WorkflowRun {
        lease_id: LeaseId,
        principal: String,
        workflow: String,
        input: Value,
        approval: Option<PlanApproval>,
    },
    EventsWatch {
        lease_id: LeaseId,
        principal: String,
        after: Option<u64>,
    },
    LeaseAcquire {
        principal: String,
        scopes: Vec<RiskScope>,
        ttl_ms: u64,
    },
    LeaseHeartbeat {
        lease_id: LeaseId,
        principal: String,
    },
    LeaseRelease {
        lease_id: LeaseId,
        principal: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseView {
    pub lease_id: LeaseId,
    pub principal: String,
    pub scopes: Vec<RiskScope>,
    pub ttl_ms: u64,
    pub expires_in_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DaemonResponse {
    SessionStatus {
        active_leases: usize,
    },
    LoginStatus {
        state: LoginState,
        challenge_id: Option<u64>,
    },
    LoginSubmitted {
        challenge_id: u64,
    },
    SchemaVersion {
        version: Value,
    },
    SchemaCapabilities {
        capabilities: Value,
    },
    SchemaSearchResults {
        results: Value,
    },
    SchemaDescription {
        description: Value,
    },
    TdPlanPreview {
        preview: Value,
    },
    TdResult {
        result: Value,
    },
    WorkflowList {
        workflows: Vec<String>,
    },
    WorkflowDescription {
        workflow: String,
        input_example: Value,
    },
    WorkflowResult {
        workflow: String,
        result: Value,
        complete: bool,
    },
    Events {
        events: Vec<EventRecord>,
        next_cursor: u64,
        gap: bool,
    },
    CommandError {
        code: CommandErrorCode,
    },
    LeaseGranted {
        lease: LeaseView,
    },
    LeaseRenewed {
        lease: LeaseView,
    },
    LeaseReleased {
        lease_id: LeaseId,
    },
    Error {
        code: LeaseErrorCode,
    },
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanApproval {
    pub plan_hash: String,
    pub expires_at_unix: u64,
    pub nonce: ProtectedString,
    pub signature: ProtectedString,
}

impl fmt::Debug for PlanApproval {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlanApproval")
            .field("plan_hash", &self.plan_hash)
            .field("expires_at_unix", &self.expires_at_unix)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginState {
    Parameters,
    QrCode,
    PhoneNumber,
    PremiumPurchase,
    Code,
    Password,
    EmailAddress,
    EmailCode,
    Registration,
    Ready,
    LoggingOut,
    Closing,
    Closed,
    Unknown,
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LoginInput {
    PhoneNumber {
        value: ProtectedString,
    },
    AuthenticationCode {
        value: ProtectedString,
    },
    Password {
        value: ProtectedString,
    },
    EmailAddress {
        value: ProtectedString,
    },
    EmailCode {
        value: ProtectedString,
    },
    Registration {
        first_name: ProtectedString,
        last_name: ProtectedString,
    },
}

impl fmt::Debug for LoginInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            Self::PhoneNumber { .. } => "phone_number",
            Self::AuthenticationCode { .. } => "authentication_code",
            Self::Password { .. } => "password",
            Self::EmailAddress { .. } => "email_address",
            Self::EmailCode { .. } => "email_code",
            Self::Registration { .. } => "registration",
        };
        formatter
            .debug_struct("LoginInput")
            .field("kind", &kind)
            .finish_non_exhaustive()
    }
}

#[derive(PartialEq, Eq)]
pub struct ProtectedString(String);

impl ProtectedString {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn into_inner(mut self) -> String {
        std::mem::take(&mut self.0)
    }
}

impl fmt::Debug for ProtectedString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

impl Serialize for ProtectedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProtectedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self)
    }
}

impl Drop for ProtectedString {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    pub sequence: u64,
    pub kind: EventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Authorization,
    User,
    UserFullInfo,
    Chat,
    BasicGroup,
    BasicGroupFullInfo,
    Supergroup,
    SupergroupFullInfo,
    File,
    Connection,
    MessageSend,
    WebAppMessage,
    Unknown,
    Gap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandErrorCode {
    SchemaNotFound,
    RuntimeUnavailable,
    InvalidTdjson,
    MethodDefaultDenied,
    AccountScopeDenied,
    RiskScopeDenied,
    ApprovalRequired,
    ApprovalDenied,
    TdlibTransport,
    UnexpectedTdlibResult,
    WorkflowNotFound,
    InvalidWorkflowInput,
    WorkflowPrerequisiteMissing,
    WorkflowCapabilityDenied,
    WorkflowResyncRequired,
    WorkflowNoResyncRequired,
    WorkflowFailed,
    LoginChallengeInvalid,
    LoginSubmissionPending,
    LoginSubmissionRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseErrorCode {
    InvalidRequest,
    InvalidPrincipal,
    InvalidScope,
    ScopeDenied,
    InvalidTtl,
    LeaseNotFound,
    LeaseExpired,
    PrincipalMismatch,
    IdentifierExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientErrorCode {
    InvalidArguments,
    InvalidJson,
    InvalidOutputFormat,
    InvalidProfile,
    SocketUnavailable,
    UnsafeSocket,
    TransportFailed,
    InvalidResponse,
    OutputFailed,
    Cancelled,
    SecureTtyUnavailable,
    SecureTtyFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineEnvelope {
    version: u16,
    #[serde(flatten)]
    outcome: MachineOutcome,
}

impl MachineEnvelope {
    pub fn from_response(response: DaemonResponse) -> Self {
        let outcome = match response {
            DaemonResponse::CommandError { code } => MachineOutcome::Error {
                error: MachineError::Command { code },
            },
            DaemonResponse::Error { code } => MachineOutcome::Error {
                error: MachineError::Lease { code },
            },
            response @ DaemonResponse::WorkflowResult {
                complete: false, ..
            }
            | response @ DaemonResponse::Events { gap: true, .. } => {
                MachineOutcome::Partial { data: response }
            }
            response @ DaemonResponse::LoginStatus { state, .. } if state != LoginState::Ready => {
                MachineOutcome::Partial { data: response }
            }
            response => MachineOutcome::Ok { data: response },
        };
        Self {
            version: MACHINE_PROTOCOL_VERSION,
            outcome,
        }
    }

    pub fn client_error(code: ClientErrorCode) -> Self {
        Self {
            version: MACHINE_PROTOCOL_VERSION,
            outcome: MachineOutcome::Error {
                error: MachineError::Client { code },
            },
        }
    }

    pub fn status(&self) -> MachineStatus {
        match &self.outcome {
            MachineOutcome::Ok { .. } => MachineStatus::Ok,
            MachineOutcome::Partial { .. } => MachineStatus::Partial,
            MachineOutcome::Error { .. } => MachineStatus::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineStatus {
    Ok,
    Partial,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum MachineOutcome {
    Ok { data: DaemonResponse },
    Partial { data: DaemonResponse },
    Error { error: MachineError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "domain", rename_all = "snake_case")]
pub enum MachineError {
    Command { code: CommandErrorCode },
    Lease { code: LeaseErrorCode },
    Client { code: ClientErrorCode },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_login_input_is_redacted_and_zeroizable() {
        let canary = "PROTECTED_LOGIN_CANARY";
        let request = DaemonRequest::LoginSubmit {
            challenge_id: 7,
            input: LoginInput::Password {
                value: ProtectedString::new(canary.to_owned()),
            },
        };
        assert!(!format!("{request:?}").contains(canary));

        let mut wire = serde_json::to_string(&request).unwrap();
        assert!(wire.contains(canary));
        wire.zeroize();
        assert!(wire.is_empty());
    }

    #[test]
    fn approval_capability_is_wire_visible_but_debug_redacted() {
        let canary = "ONE_SHOT_APPROVAL_CANARY";
        let approval = PlanApproval {
            plan_hash: "ab".repeat(32),
            expires_at_unix: 7,
            nonce: ProtectedString::new(canary.to_owned()),
            signature: ProtectedString::new(canary.to_owned()),
        };
        assert!(!format!("{approval:?}").contains(canary));

        let mut wire = serde_json::to_string(&approval).unwrap();
        assert!(wire.contains(canary));
        wire.zeroize();
        assert!(wire.is_empty());
    }
}
