//! Стабильные wire-контракты клиентов и daemon.

#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;
use std::str::FromStr;
use zeroize::Zeroize;

pub const MACHINE_PROTOCOL_VERSION: u16 = 4;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LoginChallengeId(String);

impl LoginChallengeId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LoginChallengeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for LoginChallengeId {
    type Err = ParseLoginChallengeIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((epoch, generation)) = value
            .strip_prefix("auth-")
            .and_then(|value| value.split_once('-'))
        else {
            return Err(ParseLoginChallengeIdError);
        };
        if epoch.len() != 32
            || generation.len() != 16
            || !epoch.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !generation.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ParseLoginChallengeIdError);
        }
        Ok(Self(value.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLoginChallengeIdError;

impl fmt::Display for ParseLoginChallengeIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid login challenge identifier")
    }
}

impl std::error::Error for ParseLoginChallengeIdError {}

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
    LoginPrompt {
        challenge_id: LoginChallengeId,
    },
    LoginSubmit {
        challenge_id: LoginChallengeId,
        input: LoginInput,
    },
    LoginCodeResend {
        challenge_id: LoginChallengeId,
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
    WebAppArtifactTake {
        handle: String,
        principal: String,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalMetrics {
    pub requests: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub uncertain: u64,
    pub denied: u64,
    pub request_latency_ms_total: u64,
    pub request_latency_ms_max: u64,
    pub queue_depth: usize,
    pub queue_depth_max: usize,
    pub queue_rejections: u64,
    pub retries: u64,
    pub flood_waits: u64,
    pub flood_delay_ms_total: u64,
    pub update_lag_events: u64,
    pub update_lag_ms_max: u64,
    pub fresh_results: u64,
    pub cached_results: u64,
    pub stale_results: u64,
    pub partial_results: u64,
    pub active_leases: usize,
    pub active_leases_max: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DaemonResponse {
    SessionStatus {
        metrics: Box<OperationalMetrics>,
    },
    LoginStatus {
        state: LoginState,
        challenge_id: Option<LoginChallengeId>,
        next_action: LoginNextAction,
    },
    LoginPrompt {
        challenge_id: LoginChallengeId,
        prompt: OwnerLoginPrompt,
    },
    LoginSubmitted {
        challenge_id: LoginChallengeId,
    },
    LoginCodeResent {
        challenge_id: LoginChallengeId,
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
        retries: u64,
        reconciliation_required: bool,
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
    WebAppArtifact {
        launch_id: i64,
        url: ProtectedString,
        require_same_origin: bool,
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

impl LoginState {
    pub const fn next_action(self) -> LoginNextAction {
        match self {
            Self::QrCode => LoginNextAction::ConfirmOtherDevice,
            Self::PhoneNumber
            | Self::PremiumPurchase
            | Self::Code
            | Self::Password
            | Self::EmailAddress
            | Self::EmailCode
            | Self::Registration => LoginNextAction::SubmitViaProtectedChannel,
            Self::Ready => LoginNextAction::Ready,
            Self::Closed => LoginNextAction::RestartDaemon,
            Self::Parameters | Self::LoggingOut | Self::Closing | Self::Unknown => {
                LoginNextAction::Wait
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginNextAction {
    Wait,
    SubmitViaProtectedChannel,
    ConfirmOtherDevice,
    Ready,
    RestartDaemon,
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LoginInput {
    PhoneNumber {
        value: ProtectedString,
    },
    QrCode,
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
    AppleIdToken {
        value: ProtectedString,
    },
    GoogleIdToken {
        value: ProtectedString,
    },
    Registration {
        first_name: ProtectedString,
        last_name: ProtectedString,
        terms_accepted: bool,
        disable_notification: bool,
    },
}

impl fmt::Debug for LoginInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            Self::PhoneNumber { .. } => "phone_number",
            Self::QrCode => "qr_code",
            Self::AuthenticationCode { .. } => "authentication_code",
            Self::Password { .. } => "password",
            Self::EmailAddress { .. } => "email_address",
            Self::EmailCode { .. } => "email_code",
            Self::AppleIdToken { .. } => "apple_id_token",
            Self::GoogleIdToken { .. } => "google_id_token",
            Self::Registration { .. } => "registration",
        };
        formatter
            .debug_struct("LoginInput")
            .field("kind", &kind)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerLoginPrompt {
    PhoneNumber,
    PremiumPurchase,
    AuthenticationCode,
    Password {
        hint: ProtectedString,
        has_recovery_email_address: bool,
        recovery_email_address_pattern: ProtectedString,
    },
    EmailAddress {
        allow_apple_id: bool,
        allow_google_id: bool,
    },
    EmailCode {
        allow_apple_id: bool,
        allow_google_id: bool,
    },
    QrCode {
        link: ProtectedString,
    },
    Registration {
        terms: ProtectedString,
        minimum_user_age: i32,
        show_popup: bool,
    },
}

#[derive(Clone, PartialEq, Eq)]
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
    WebAppArtifactUnavailable,
    LoginChallengeInvalid,
    LoginSubmissionPending,
    LoginSubmissionRejected,
    LoginCodeResendUnavailable,
    LoginCodeResendRejected,
    OperationAlreadySucceeded,
    ReconciliationRequired,
    ReliabilityUnavailable,
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
            | response @ DaemonResponse::TdResult {
                reconciliation_required: true,
                ..
            }
            | response @ DaemonResponse::Events { gap: true, .. } => {
                MachineOutcome::Partial { data: response }
            }
            response @ DaemonResponse::LoginStatus { state, .. } if state != LoginState::Ready => {
                MachineOutcome::Partial { data: response }
            }
            response @ (DaemonResponse::LoginPrompt { .. }
            | DaemonResponse::LoginSubmitted { .. }
            | DaemonResponse::LoginCodeResent { .. }) => MachineOutcome::Partial { data: response },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserEvidence {
    pub passed: bool,
    pub dom_assertions: u32,
    pub bridge_assertions: u32,
    pub network_assertions: u32,
    pub js_errors: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebAppBrowserReport {
    pub launch_id: i64,
    pub telegram_prepared: bool,
    pub browser: BrowserEvidence,
    pub artifact_consumed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn challenge() -> LoginChallengeId {
        LoginChallengeId::new("auth-0000000000000000000000000000002a-0000000000000007".to_owned())
    }

    #[test]
    fn protected_login_input_is_redacted_and_zeroizable() {
        let canary = "PROTECTED_LOGIN_CANARY";
        let request = DaemonRequest::LoginSubmit {
            challenge_id: challenge(),
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
    fn code_resend_request_contains_only_challenge_metadata() {
        let request = DaemonRequest::LoginCodeResend {
            challenge_id: challenge(),
        };
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "type": "login_code_resend",
                "challenge_id": "auth-0000000000000000000000000000002a-0000000000000007"
            })
        );
    }

    #[test]
    fn login_status_exposes_only_broker_metadata() {
        let envelope = MachineEnvelope::from_response(DaemonResponse::LoginStatus {
            state: LoginState::Code,
            challenge_id: Some(challenge()),
            next_action: LoginState::Code.next_action(),
        });
        assert_eq!(
            serde_json::to_value(envelope).unwrap(),
            serde_json::json!({
                "version": 4,
                "status": "partial",
                "data": {
                    "type": "login_status",
                    "state": "code",
                    "challenge_id": "auth-0000000000000000000000000000002a-0000000000000007",
                    "next_action": "submit_via_protected_channel",
                },
            })
        );
    }

    #[test]
    fn login_progress_responses_are_machine_partial() {
        assert_eq!(
            MachineEnvelope::from_response(DaemonResponse::LoginSubmitted {
                challenge_id: challenge(),
            })
            .status(),
            MachineStatus::Partial
        );
        assert_eq!(
            MachineEnvelope::from_response(DaemonResponse::LoginCodeResent {
                challenge_id: challenge(),
            })
            .status(),
            MachineStatus::Partial
        );
    }

    #[test]
    fn owner_prompt_and_registration_choices_are_explicit_and_debug_redacted() {
        let canary = "OWNER_ONLY_QR_CANARY";
        let response = DaemonResponse::LoginPrompt {
            challenge_id: challenge(),
            prompt: OwnerLoginPrompt::QrCode {
                link: ProtectedString::new(canary.to_owned()),
            },
        };
        assert!(!format!("{response:?}").contains(canary));

        let input = LoginInput::Registration {
            first_name: ProtectedString::new("Ada".to_owned()),
            last_name: ProtectedString::new(String::new()),
            terms_accepted: true,
            disable_notification: true,
        };
        let value = serde_json::to_value(input).unwrap();
        assert_eq!(value["terms_accepted"], true);
        assert_eq!(value["disable_notification"], true);
    }

    #[test]
    fn challenge_identifier_parser_rejects_unscoped_numbers() {
        assert!("7".parse::<LoginChallengeId>().is_err());
        assert_eq!(challenge().to_string(), challenge().as_str());
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

    #[test]
    fn web_app_artifact_url_is_redacted_from_debug() {
        let canary = "WEB_APP_INIT_DATA_CANARY";
        let response = DaemonResponse::WebAppArtifact {
            launch_id: 11,
            url: ProtectedString::new(canary.to_owned()),
            require_same_origin: true,
        };
        assert!(!format!("{response:?}").contains(canary));
    }
}
