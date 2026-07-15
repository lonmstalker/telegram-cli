//! Стабильные wire-контракты клиентов и daemon.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::str::FromStr;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DaemonRequest {
    SessionStatus,
    LoginStatus,
    SchemaVersion,
    SchemaCapabilities,
    SchemaSearch {
        query: String,
    },
    SchemaDescribe {
        name: String,
    },
    TdCall {
        lease_id: LeaseId,
        principal: String,
        request: Value,
    },
    WorkflowList,
    WorkflowRun {
        lease_id: LeaseId,
        principal: String,
        workflow: String,
        input: Value,
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
    TdResult {
        result: Value,
    },
    WorkflowList {
        workflows: Vec<String>,
    },
    WorkflowResult {
        workflow: String,
        result: Value,
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
