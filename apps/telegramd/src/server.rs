//! Bounded JSONL lease protocol поверх private profile socket.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use telegram_core::approval::{ApprovalReceipt, ApprovalVerifier, PlanPreview};
use telegram_core::authorization::{
    AuthorizationChallengeKind, AuthorizationError, AuthorizationInput, AuthorizationMachine,
    AuthorizationRequest, AuthorizationStep, ChallengeId, SensitiveString,
};
use telegram_core::idempotency::{
    BeginDecision, IdempotencyJournal, OperationFingerprint, OperationState,
};
use telegram_core::raw_api::{
    self, PolicyError, RawApiError, SchemaDescription, SchemaSearchResult,
};
use telegram_core::reducer::{AppliedUpdate, CachedUpdateKind, ChatList};
use telegram_core::registry::{
    self, AccountKind, CapabilityDisposition, RetryClass, RiskClass, SymbolKind, ValidatedRequest,
};
use telegram_core::runtime::CoreRuntime;
use telegram_core::workflows::{
    self, ChatSearchQuery, ChatTarget, ChatWorkflowError, CustomEmojiSetAction, DownloadQuery,
    ForumTopicsQuery, HistoryQuery, InputFileSource, MembersQuery, MembershipTarget,
    NotificationScope, NotificationSettingsPatch, PageOptions, StickerFormat, StoryAction,
    StoryPrivacy, UserTarget, WebAppMode, WebAppRequest,
};
use telegram_protocol::{
    CommandErrorCode, DaemonRequest, DaemonResponse, EventKind, EventRecord, LeaseErrorCode,
    LoginInput, LoginState, PlanApproval, ProtectedString,
};
use zeroize::Zeroizing;

use crate::lease::LeaseManager;
use crate::scheduler::{
    AccountScheduler, FloodScope, OperationClass, OperationContext, OperationPermit,
};
use crate::telemetry::{AuditEvent, AuditLog, OperationOutcome, Telemetry};

const MAX_REQUEST_BYTES: u64 = 16 * 1024;
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(5);
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
const EVENT_BUFFER_CAPACITY: usize = 1024;
const WEB_APP_ARTIFACT_TTL: Duration = Duration::from_secs(60);
const WORKFLOWS: &[(&str, &str)] = &[
    (
        "user_profile",
        r#"{"target":{"kind":"self"},"include_full_info":true}"#,
    ),
    (
        "update_profile_name",
        r#"{"first_name":"Name","last_name":""}"#,
    ),
    ("plan_chat_title", r#"{"chat_id":0,"title":"Title"}"#),
    ("apply_chat_title", r#"{"chat_id":0,"title":"Title"}"#),
    ("resolve_chat", r#"{"kind":"id","chat_id":0}"#),
    ("ensure_membership", r#"{"kind":"chat_id","chat_id":0}"#),
    ("load_chat_list", r#"{"list":{"kind":"main"},"limit":100}"#),
    (
        "inspect_chat",
        r#"{"target":{"kind":"id","chat_id":0},"open":false}"#,
    ),
    (
        "forum_topics",
        r#"{"chat_id":0,"query":"","count":100,"page_limit":100}"#,
    ),
    (
        "set_forum_topic_closed",
        r#"{"chat_id":0,"topic_id":0,"is_closed":true}"#,
    ),
    (
        "chat_history",
        r#"{"chat_id":0,"only_local":false,"mark_read":false,"page":{"count":100,"min_date":null,"page_limit":100}}"#,
    ),
    (
        "search_chat_messages",
        r#"{"chat_id":0,"query":"","mark_read":false,"page":{"count":100,"min_date":null,"page_limit":100}}"#,
    ),
    ("send_text_message", r#"{"chat_id":0,"text":"hello"}"#),
    (
        "supergroup_members",
        r#"{"supergroup_id":0,"count":100,"page_limit":100}"#,
    ),
    ("chat_statistics", r#"{"chat_id":0,"is_dark":false}"#),
    ("resource_statistics", r#"{"only_current_network":true}"#),
    ("proxy_status", "{}"),
    ("set_proxy_enabled", r#"{"action":"enable","proxy_id":1}"#),
    ("resync_after_gap", "{}"),
    (
        "download_file",
        r#"{"file_id":0,"priority":1,"offset":0,"limit":0}"#,
    ),
    (
        "cancel_download",
        r#"{"file_id":0,"only_if_pending":false}"#,
    ),
    (
        "upload_sticker_file",
        r#"{"user_id":0,"format":"webp","source":{"kind":"id","id":0}}"#,
    ),
    (
        "plan_custom_emoji_set",
        r#"{"action":"create","user_id":1,"title":"Disposable","name":"codex_disposable","format":"webp","sticker_file_id":1,"emojis":"🧪","needs_repainting":false}"#,
    ),
    (
        "apply_custom_emoji_set",
        r#"{"action":"create","user_id":1,"title":"Disposable","name":"codex_disposable","format":"webp","sticker_file_id":1,"emojis":"🧪","needs_repainting":false}"#,
    ),
    (
        "plan_story_mutation",
        r#"{"action":"post_photo","chat_id":1,"photo_file_id":1,"caption":"","privacy":{"kind":"selected_users","user_ids":[1]},"active_period":86400,"is_posted_to_chat_page":false,"protect_content":true}"#,
    ),
    (
        "apply_story_mutation",
        r#"{"action":"post_photo","chat_id":1,"photo_file_id":1,"caption":"","privacy":{"kind":"selected_users","user_ids":[1]},"active_period":86400,"is_posted_to_chat_page":false,"protect_content":true}"#,
    ),
    ("inspect_group_call", r#"{"group_call_id":1}"#),
    ("leave_group_call", r#"{"group_call_id":1}"#),
    ("notification_settings", r#"{"scope":"private_chats"}"#),
    (
        "set_notification_settings",
        r#"{"scope":"private_chats","patch":{"mute_for":60}}"#,
    ),
    ("active_sessions", "{}"),
    ("plan_terminate_session", r#"{"session_id":1}"#),
    ("apply_terminate_session", r#"{"session_id":1}"#),
    (
        "business_connection",
        r#"{"connection_id":"connection-id"}"#,
    ),
    (
        "send_business_text",
        r#"{"connection_id":"connection-id","chat_id":1,"text":"hello"}"#,
    ),
    ("star_balance", "{}"),
    (
        "plan_star_invoice_payment",
        r#"{"invoice_name":"invoice-name"}"#,
    ),
    (
        "apply_star_invoice_payment",
        r#"{"invoice_name":"invoice-name"}"#,
    ),
    (
        "start_bot",
        r#"{"bot_user_id":0,"chat_id":0,"parameter":""}"#,
    ),
    (
        "start_bot_and_wait_reply",
        r#"{"bot_user_id":0,"chat_id":0,"parameter":""}"#,
    ),
    (
        "click_bot_callback",
        r#"{"chat_id":0,"message_id":0,"row":0,"column":0}"#,
    ),
    (
        "open_web_app",
        r#"{"chat_id":0,"bot_user_id":0,"button_url":"https://example.invalid","application_name":"main","mode":"compact"}"#,
    ),
    (
        "prepare_web_app_handoff",
        r#"{"chat_id":0,"bot_user_id":0,"button_url":"https://example.invalid","application_name":"main","mode":"compact"}"#,
    ),
    ("close_web_app_handoff", r#"{"launch_id":0}"#),
];
const JOURNALED_WORKFLOWS: &[&str] = &[
    "ensure_membership",
    "send_text_message",
    "upload_sticker_file",
    "apply_custom_emoji_set",
    "apply_story_mutation",
    "apply_terminate_session",
    "send_business_text",
    "apply_star_invoice_payment",
    "start_bot",
    "start_bot_and_wait_reply",
    "click_bot_callback",
    "open_web_app",
    "prepare_web_app_handoff",
];

struct WebAppArtifact {
    principal: String,
    launch_id: i64,
    url: ProtectedString,
    require_same_origin: bool,
    expires_at: Instant,
}

struct WebAppArtifactStore {
    epoch: u128,
    next_id: u64,
    artifacts: HashMap<String, WebAppArtifact>,
}

impl Default for WebAppArtifactStore {
    fn default() -> Self {
        let epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            ^ ((std::process::id() as u128) << 64);
        Self {
            epoch,
            next_id: 1,
            artifacts: HashMap::new(),
        }
    }
}

impl WebAppArtifactStore {
    fn insert(
        &mut self,
        principal: String,
        launch_id: i64,
        url: ProtectedString,
        require_same_origin: bool,
        now: Instant,
    ) -> Option<(String, u64)> {
        self.expire(now);
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1)?;
        let expires_at = now.checked_add(WEB_APP_ARTIFACT_TTL)?;
        let handle = format!("webapp-{:032x}-{id:016x}", self.epoch);
        self.artifacts.insert(
            handle.clone(),
            WebAppArtifact {
                principal,
                launch_id,
                url,
                require_same_origin,
                expires_at,
            },
        );
        Some((handle, WEB_APP_ARTIFACT_TTL.as_millis() as u64))
    }

    fn take(&mut self, handle: &str, principal: &str, now: Instant) -> Option<WebAppArtifact> {
        self.expire(now);
        (self.artifacts.get(handle)?.principal == principal)
            .then(|| self.artifacts.remove(handle))?
    }

    fn expire(&mut self, now: Instant) {
        self.artifacts
            .retain(|_, artifact| artifact.expires_at > now);
    }
}

pub struct LeaseServer {
    leases: LeaseManager,
    scheduler: AccountScheduler,
    telemetry: Telemetry,
    idempotency: IdempotencyJournal,
    audit: AuditLog,
    ready: bool,
    events: EventBuffer,
    authorization: AuthorizationBroker,
    artifact_root: Option<PathBuf>,
    approval_verifier: Option<ApprovalVerifier>,
    web_app_artifacts: WebAppArtifactStore,
    account_kind: Option<AccountKind>,
}

impl LeaseServer {
    pub fn new(
        leases: LeaseManager,
        scheduler: AccountScheduler,
        telemetry: Telemetry,
        idempotency: IdempotencyJournal,
        audit: AuditLog,
    ) -> Self {
        Self {
            leases,
            scheduler,
            telemetry,
            idempotency,
            audit,
            ready: true,
            events: EventBuffer::new(EVENT_BUFFER_CAPACITY),
            authorization: AuthorizationBroker::default(),
            artifact_root: None,
            approval_verifier: None,
            web_app_artifacts: WebAppArtifactStore::default(),
            account_kind: None,
        }
    }

    pub fn with_artifact_root(mut self, root: PathBuf) -> Self {
        self.artifact_root = Some(root);
        self
    }

    pub fn with_approval_verifier(mut self, verifier: Option<ApprovalVerifier>) -> Self {
        self.approval_verifier = verifier;
        self
    }

    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }

    pub fn set_account_kind(&mut self, account_kind: AccountKind) {
        self.account_kind = Some(account_kind);
    }

    pub fn start_events_at(&mut self, sequence: Option<u64>) {
        self.events.start_at(sequence.unwrap_or_default());
    }

    pub fn record_event(&mut self, update: AppliedUpdate) {
        self.events.record(update.sequence.get(), update.kind);
    }

    pub fn observe_authorization(&mut self, runtime: &CoreRuntime) -> Result<(), ServerError> {
        let state = runtime
            .state()
            .authorization()
            .ok_or(ServerError::MissingAuthorizationState)?;
        self.authorization
            .observe(&state.value)
            .map_err(ServerError::Authorization)?;
        self.ready = matches!(self.authorization.step, Some(AuthorizationStep::Ready));
        Ok(())
    }

    pub fn poll(
        &mut self,
        listener: &UnixListener,
        runtime: &mut CoreRuntime,
        now: Instant,
    ) -> Result<(), ServerError> {
        self.leases.expire(now);
        self.web_app_artifacts.expire(now);
        loop {
            match self.serve_once(listener, Some(&mut *runtime)) {
                Ok(()) => {}
                Err(ServerError::Accept(io::ErrorKind::WouldBlock)) => return Ok(()),
                Err(error @ ServerError::Accept(_)) => return Err(error),
                Err(ServerError::ClientIo(_) | ServerError::SerializeResponse) => {}
                Err(
                    error
                    @ (ServerError::MissingAuthorizationState | ServerError::Authorization(_)),
                ) => {
                    return Err(error);
                }
            }
        }
    }

    pub fn active_leases(&self) -> usize {
        self.leases.active_count()
    }

    fn admit(
        &self,
        risk: RiskClass,
        chat_id: Option<i64>,
    ) -> Result<(OperationPermit, Duration), CommandErrorCode> {
        let queued_at = Instant::now();
        let permit = self
            .scheduler
            .enqueue(OperationContext {
                operation: if risk == RiskClass::Read {
                    OperationClass::Read
                } else {
                    OperationClass::Mutation
                },
                method_class: risk,
                chat_id,
            })
            .and_then(|queued| queued.wait())
            .map_err(|_| CommandErrorCode::ReliabilityUnavailable)?;
        Ok((permit, queued_at.elapsed()))
    }

    fn begin_idempotent(
        &mut self,
        fingerprint: Option<OperationFingerprint>,
    ) -> Result<Option<OperationFingerprint>, CommandErrorCode> {
        let Some(fingerprint) = fingerprint else {
            return Ok(None);
        };
        match self.idempotency.begin(fingerprint) {
            Ok(BeginDecision::Dispatch) => Ok(Some(fingerprint)),
            Ok(BeginDecision::AlreadySucceeded) => Err(CommandErrorCode::OperationAlreadySucceeded),
            Ok(BeginDecision::ReconcileRequired) => Err(CommandErrorCode::ReconciliationRequired),
            Err(_) => Err(CommandErrorCode::ReliabilityUnavailable),
        }
    }

    fn finish_idempotent(
        &mut self,
        fingerprint: Option<OperationFingerprint>,
        state: OperationState,
    ) -> Result<(), CommandErrorCode> {
        let Some(fingerprint) = fingerprint else {
            return Ok(());
        };
        let result = match state {
            OperationState::Succeeded => self.idempotency.succeeded(fingerprint),
            OperationState::Failed => self.idempotency.failed(fingerprint),
            OperationState::Uncertain => self.idempotency.uncertain(fingerprint),
            OperationState::Pending => return Err(CommandErrorCode::ReliabilityUnavailable),
        };
        result.map_err(|_| CommandErrorCode::ReliabilityUnavailable)
    }

    fn record_raw(
        &mut self,
        request: &ValidatedRequest,
        outcome: OperationOutcome,
        started: Instant,
        queued: Duration,
        retries: u64,
    ) -> Result<(), CommandErrorCode> {
        let latency = started.elapsed();
        self.telemetry.record_request(latency, outcome);
        for _ in 0..retries {
            self.telemetry.record_retry();
        }
        let event = AuditEvent::operation(
            request,
            outcome,
            latency,
            queued,
            retries,
            false,
            SystemTime::now(),
        )
        .map_err(|_| CommandErrorCode::ReliabilityUnavailable)?;
        self.audit
            .append(&event)
            .map_err(|_| CommandErrorCode::ReliabilityUnavailable)
    }

    fn record_workflow(&self, outcome: OperationOutcome, started: Instant) {
        let latency = started.elapsed();
        self.telemetry.record_request(latency, outcome);
    }

    fn serve_once(
        &mut self,
        listener: &UnixListener,
        runtime: Option<&mut CoreRuntime>,
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
        runtime: Option<&mut CoreRuntime>,
    ) -> Result<(), ServerError> {
        stream
            .set_read_timeout(Some(CLIENT_IO_TIMEOUT))
            .map_err(|error| ServerError::ClientIo(error.kind()))?;
        stream
            .set_write_timeout(Some(CLIENT_IO_TIMEOUT))
            .map_err(|error| ServerError::ClientIo(error.kind()))?;
        let mut bytes = Zeroizing::new(Vec::new());
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
            let request = serde_json::from_slice(&bytes);
            match request {
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
        runtime: Option<&mut CoreRuntime>,
        now: Instant,
    ) -> DaemonResponse {
        if !self.ready
            && matches!(
                &request,
                DaemonRequest::TdCall { .. } | DaemonRequest::WorkflowRun { .. }
            )
        {
            return runtime_unavailable();
        }
        match request {
            DaemonRequest::SessionStatus => DaemonResponse::SessionStatus {
                metrics: Box::new(self.telemetry.snapshot()),
            },
            DaemonRequest::LoginStatus => {
                let (state, challenge_id) = self.authorization.status();
                DaemonResponse::LoginStatus {
                    state,
                    challenge_id,
                }
            }
            DaemonRequest::LoginSubmit {
                challenge_id,
                input,
            } => {
                let (challenge, request) = match self.authorization.submit(challenge_id, input) {
                    Ok(request) => request,
                    Err(AuthorizationError::SubmissionPending) => {
                        return DaemonResponse::CommandError {
                            code: CommandErrorCode::LoginSubmissionPending,
                        };
                    }
                    Err(_) => {
                        return DaemonResponse::CommandError {
                            code: CommandErrorCode::LoginChallengeInvalid,
                        };
                    }
                };
                let Some(runtime) = runtime else {
                    let _ = self.authorization.machine.submission_failed(challenge);
                    return runtime_unavailable();
                };
                let deadline = now.checked_add(CALL_TIMEOUT).unwrap_or(now);
                let response = runtime
                    .transport()
                    .call_until(request.into_value(), deadline);
                match response {
                    Ok(response) if response.get("@type").and_then(Value::as_str) == Some("ok") => {
                        DaemonResponse::LoginSubmitted { challenge_id }
                    }
                    Ok(_) => {
                        let _ = self.authorization.machine.submission_failed(challenge);
                        DaemonResponse::CommandError {
                            code: CommandErrorCode::LoginSubmissionRejected,
                        }
                    }
                    Err(_) => {
                        let _ = self.authorization.machine.submission_failed(challenge);
                        DaemonResponse::CommandError {
                            code: CommandErrorCode::TdlibTransport,
                        }
                    }
                }
            }
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
            DaemonRequest::TdPreview { request } => match plan_preview(request) {
                Ok(preview) => DaemonResponse::TdPlanPreview { preview },
                Err(code) => DaemonResponse::CommandError { code },
            },
            DaemonRequest::TdCall {
                lease_id,
                principal,
                request,
                approval,
            } => {
                let started = Instant::now();
                let Some(account_kind) = self.account_kind else {
                    return runtime_unavailable();
                };
                let validated = match ValidatedRequest::from_value(request.clone()) {
                    Ok(request) => request,
                    Err(_) => {
                        return DaemonResponse::CommandError {
                            code: CommandErrorCode::InvalidTdjson,
                        };
                    }
                };
                let Some(capability) = registry::capability(validated.descriptor().name) else {
                    return DaemonResponse::CommandError {
                        code: CommandErrorCode::MethodDefaultDenied,
                    };
                };
                let CapabilityDisposition::Reviewed { risk, retry, .. } = capability.disposition
                else {
                    return DaemonResponse::CommandError {
                        code: CommandErrorCode::MethodDefaultDenied,
                    };
                };
                let policy = match self
                    .leases
                    .raw_policy(&lease_id, &principal, account_kind, now)
                {
                    Ok(policy) => policy,
                    Err(code) => return DaemonResponse::Error { code },
                };
                let Some(runtime) = runtime else {
                    return runtime_unavailable();
                };
                let policy = match approved_policy(
                    policy,
                    approval,
                    &request,
                    self.approval_verifier.as_ref(),
                ) {
                    Ok(policy) => policy,
                    Err(code) => return DaemonResponse::CommandError { code },
                };
                let (_permit, queued) = match self.admit(
                    risk,
                    validated.as_value().get("chat_id").and_then(Value::as_i64),
                ) {
                    Ok(admission) => admission,
                    Err(code) => return DaemonResponse::CommandError { code },
                };
                let fingerprint = (retry != RetryClass::SafeRead)
                    .then(|| OperationFingerprint::for_request(&validated));
                let fingerprint = match self.begin_idempotent(fingerprint) {
                    Ok(fingerprint) => fingerprint,
                    Err(code) => {
                        let outcome = if code == CommandErrorCode::ReconciliationRequired {
                            OperationOutcome::Uncertain
                        } else {
                            OperationOutcome::Denied
                        };
                        let _ = self.record_raw(&validated, outcome, started, queued, 0);
                        return DaemonResponse::CommandError { code };
                    }
                };
                let deadline = now.checked_add(CALL_TIMEOUT).unwrap_or(now);
                let scheduler = self.scheduler.clone();
                let result =
                    raw_api::td_call_observed(runtime, &policy, request, deadline, |delay| {
                        scheduler
                            .record_flood_wait(FloodScope::MethodClass(risk), delay)
                            .ok()
                            .and_then(|decision| decision.automatic_delay)
                    });
                let (state, outcome) = match &result {
                    Ok((result, _))
                        if result.as_value()["@type"] != "error" && fingerprint.is_none() =>
                    {
                        (OperationState::Succeeded, OperationOutcome::Succeeded)
                    }
                    Ok((result, _)) if result.as_value()["@type"] != "error" => {
                        (OperationState::Uncertain, OperationOutcome::Uncertain)
                    }
                    Ok(_) => (OperationState::Failed, OperationOutcome::Failed),
                    Err(RawApiError::Transport(_) | RawApiError::UnexpectedResult { .. }) => {
                        (OperationState::Uncertain, OperationOutcome::Uncertain)
                    }
                    Err(_) => (OperationState::Failed, OperationOutcome::Failed),
                };
                if let Err(code) = self.finish_idempotent(fingerprint, state) {
                    let retries = result.as_ref().map_or(0, |(_, retries)| *retries);
                    let _ = self.record_raw(
                        &validated,
                        OperationOutcome::Uncertain,
                        started,
                        queued,
                        retries,
                    );
                    return DaemonResponse::CommandError { code };
                }
                let retries = result.as_ref().map_or(0, |(_, retries)| *retries);
                if let Err(code) = self.record_raw(&validated, outcome, started, queued, retries) {
                    return DaemonResponse::CommandError { code };
                }
                match result {
                    Ok((result, retries)) => DaemonResponse::TdResult {
                        result: result.into_value(),
                        retries,
                        reconciliation_required: state == OperationState::Uncertain,
                    },
                    Err(error) => DaemonResponse::CommandError {
                        code: raw_error(error),
                    },
                }
            }
            DaemonRequest::WorkflowList => DaemonResponse::WorkflowList {
                workflows: WORKFLOWS
                    .iter()
                    .map(|(name, _)| (*name).to_owned())
                    .collect(),
            },
            DaemonRequest::WorkflowDescribe { workflow } => match workflow_input_example(&workflow)
            {
                Some(input_example) => DaemonResponse::WorkflowDescription {
                    workflow,
                    input_example,
                },
                None => DaemonResponse::CommandError {
                    code: CommandErrorCode::WorkflowNotFound,
                },
            },
            DaemonRequest::WorkflowRun {
                lease_id,
                principal,
                workflow,
                input,
                approval,
            } => {
                let started = Instant::now();
                let Some((workflow_name, _)) = WORKFLOWS
                    .iter()
                    .find(|(candidate, _)| *candidate == workflow)
                else {
                    return DaemonResponse::CommandError {
                        code: CommandErrorCode::WorkflowNotFound,
                    };
                };
                let Some(account_kind) = self.account_kind else {
                    return runtime_unavailable();
                };
                let policy = match self
                    .leases
                    .raw_policy(&lease_id, &principal, account_kind, now)
                {
                    Ok(policy) => policy,
                    Err(code) => return DaemonResponse::Error { code },
                };
                let Some(runtime) = runtime else {
                    return runtime_unavailable();
                };
                let fingerprint = JOURNALED_WORKFLOWS
                    .contains(workflow_name)
                    .then(|| OperationFingerprint::for_workflow(workflow_name, &input));
                let fingerprint = match self.begin_idempotent(fingerprint) {
                    Ok(fingerprint) => fingerprint,
                    Err(code) => {
                        let outcome = if code == CommandErrorCode::ReconciliationRequired {
                            OperationOutcome::Uncertain
                        } else {
                            OperationOutcome::Denied
                        };
                        self.record_workflow(outcome, started);
                        return DaemonResponse::CommandError { code };
                    }
                };
                let deadline = now.checked_add(CALL_TIMEOUT).unwrap_or(now);
                let result = run_workflow(
                    runtime,
                    &workflow,
                    input,
                    WorkflowContext {
                        policy,
                        principal,
                        approval,
                        approval_verifier: self.approval_verifier.as_ref(),
                        artifact_root: self.artifact_root.as_deref(),
                        web_app_artifacts: &mut self.web_app_artifacts,
                        deadline,
                    },
                );
                let (state, outcome) = match &result {
                    Ok(output) if output.complete => {
                        (OperationState::Succeeded, OperationOutcome::Succeeded)
                    }
                    Ok(_) => (OperationState::Uncertain, OperationOutcome::Uncertain),
                    Err(_) => (OperationState::Failed, OperationOutcome::Failed),
                };
                self.events.reconcile(
                    runtime
                        .state()
                        .last_sequence()
                        .map(telegram_core::reducer::UpdateSequence::get),
                );
                if let Err(code) = self.finish_idempotent(fingerprint, state) {
                    self.record_workflow(OperationOutcome::Uncertain, started);
                    return DaemonResponse::CommandError { code };
                }
                self.record_workflow(outcome, started);
                match result {
                    Ok(output) => DaemonResponse::WorkflowResult {
                        workflow,
                        result: output.result,
                        complete: output.complete,
                    },
                    Err(error) => DaemonResponse::CommandError {
                        code: workflow_error(error),
                    },
                }
            }
            DaemonRequest::WebAppArtifactTake { handle, principal } => {
                match self.web_app_artifacts.take(&handle, &principal, now) {
                    Some(artifact) => DaemonResponse::WebAppArtifact {
                        launch_id: artifact.launch_id,
                        url: artifact.url,
                        require_same_origin: artifact.require_same_origin,
                    },
                    None => DaemonResponse::CommandError {
                        code: CommandErrorCode::WebAppArtifactUnavailable,
                    },
                }
            }
            DaemonRequest::EventsWatch {
                lease_id,
                principal,
                after,
            } => {
                let Some(account_kind) = self.account_kind else {
                    return runtime_unavailable();
                };
                if let Err(code) = self
                    .leases
                    .raw_policy(&lease_id, &principal, account_kind, now)
                {
                    return DaemonResponse::Error { code };
                }
                let (events, next_cursor, gap) = self.events.since(after);
                DaemonResponse::Events {
                    events,
                    next_cursor,
                    gap,
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

fn workflow_input_example(name: &str) -> Option<Value> {
    let (_, input_example) = WORKFLOWS.iter().find(|(candidate, _)| *candidate == name)?;
    Some(serde_json::from_str(input_example).expect("workflow input example is valid JSON"))
}

struct EventBuffer {
    capacity: usize,
    baseline: u64,
    latest: u64,
    events: VecDeque<EventRecord>,
}

impl EventBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            baseline: 0,
            latest: 0,
            events: VecDeque::new(),
        }
    }

    fn start_at(&mut self, sequence: u64) {
        self.baseline = sequence;
        self.latest = sequence;
        self.events.clear();
    }

    fn record(&mut self, sequence: u64, kind: CachedUpdateKind) {
        if sequence <= self.latest {
            return;
        }
        if sequence != self.latest.saturating_add(1) {
            self.push(EventRecord {
                sequence,
                kind: EventKind::Gap,
            });
            return;
        }
        self.push(EventRecord {
            sequence,
            kind: event_kind(kind),
        });
    }

    fn reconcile(&mut self, sequence: Option<u64>) {
        let Some(sequence) = sequence else {
            return;
        };
        if sequence > self.latest {
            self.push(EventRecord {
                sequence,
                kind: EventKind::Gap,
            });
        }
    }

    fn push(&mut self, event: EventRecord) {
        self.latest = event.sequence;
        self.events.push_back(event);
        while self.events.len() > self.capacity {
            if let Some(discarded) = self.events.pop_front() {
                self.baseline = discarded.sequence;
            }
        }
    }

    fn since(&self, after: Option<u64>) -> (Vec<EventRecord>, u64, bool) {
        let Some(after) = after else {
            return (Vec::new(), self.latest, false);
        };
        let gap = after < self.baseline || after > self.latest;
        let after = after.max(self.baseline);
        let events = self
            .events
            .iter()
            .copied()
            .filter(|event| event.sequence > after)
            .collect();
        (events, self.latest, gap)
    }
}

#[derive(Default)]
struct AuthorizationBroker {
    machine: AuthorizationMachine,
    step: Option<AuthorizationStep>,
}

impl AuthorizationBroker {
    fn observe(&mut self, state: &Value) -> Result<(), AuthorizationError> {
        self.step = Some(self.machine.observe_state(state)?);
        Ok(())
    }

    fn status(&self) -> (LoginState, Option<u64>) {
        let Some(step) = &self.step else {
            return (LoginState::Unknown, None);
        };
        let challenge_id = match step {
            AuthorizationStep::ParametersRequired { generation } => Some(generation.get()),
            AuthorizationStep::Challenge(challenge) => Some(challenge.id.get()),
            AuthorizationStep::Ready
            | AuthorizationStep::LoggingOut
            | AuthorizationStep::Closing
            | AuthorizationStep::Closed => None,
        };
        (login_state(step), challenge_id)
    }

    fn submit(
        &mut self,
        challenge_id: u64,
        input: LoginInput,
    ) -> Result<(ChallengeId, AuthorizationRequest), AuthorizationError> {
        let challenge = match &self.step {
            Some(AuthorizationStep::Challenge(challenge)) if challenge.id.get() == challenge_id => {
                challenge.id
            }
            _ => return Err(AuthorizationError::StaleChallenge),
        };
        let request = self.machine.submit(challenge, authorization_input(input))?;
        Ok((challenge, request))
    }
}

fn login_state(step: &AuthorizationStep) -> LoginState {
    match step {
        AuthorizationStep::ParametersRequired { .. } => LoginState::Parameters,
        AuthorizationStep::Ready => LoginState::Ready,
        AuthorizationStep::LoggingOut => LoginState::LoggingOut,
        AuthorizationStep::Closing => LoginState::Closing,
        AuthorizationStep::Closed => LoginState::Closed,
        AuthorizationStep::Challenge(challenge) => match &challenge.kind {
            AuthorizationChallengeKind::PhoneNumber => LoginState::PhoneNumber,
            AuthorizationChallengeKind::PremiumPurchase { .. } => LoginState::PremiumPurchase,
            AuthorizationChallengeKind::EmailAddress { .. } => LoginState::EmailAddress,
            AuthorizationChallengeKind::EmailCode { .. } => LoginState::EmailCode,
            AuthorizationChallengeKind::AuthenticationCode(_) => LoginState::Code,
            AuthorizationChallengeKind::OtherDeviceConfirmation { .. } => LoginState::QrCode,
            AuthorizationChallengeKind::Registration(_) => LoginState::Registration,
            AuthorizationChallengeKind::Password { .. } => LoginState::Password,
        },
    }
}

fn authorization_input(input: LoginInput) -> AuthorizationInput {
    match input {
        LoginInput::PhoneNumber { value } => {
            AuthorizationInput::PhoneNumber(SensitiveString::new(value.into_inner()))
        }
        LoginInput::AuthenticationCode { value } => {
            AuthorizationInput::AuthenticationCode(SensitiveString::new(value.into_inner()))
        }
        LoginInput::Password { value } => {
            AuthorizationInput::Password(SensitiveString::new(value.into_inner()))
        }
        LoginInput::EmailAddress { value } => {
            AuthorizationInput::EmailAddress(SensitiveString::new(value.into_inner()))
        }
        LoginInput::EmailCode { value } => {
            AuthorizationInput::EmailCode(SensitiveString::new(value.into_inner()))
        }
        LoginInput::Registration {
            first_name,
            last_name,
        } => AuthorizationInput::Registration {
            first_name: SensitiveString::new(first_name.into_inner()),
            last_name: SensitiveString::new(last_name.into_inner()),
            disable_notification: false,
        },
    }
}

fn event_kind(kind: CachedUpdateKind) -> EventKind {
    match kind {
        CachedUpdateKind::Authorization => EventKind::Authorization,
        CachedUpdateKind::User => EventKind::User,
        CachedUpdateKind::UserFullInfo => EventKind::UserFullInfo,
        CachedUpdateKind::Chat => EventKind::Chat,
        CachedUpdateKind::BasicGroup => EventKind::BasicGroup,
        CachedUpdateKind::BasicGroupFullInfo => EventKind::BasicGroupFullInfo,
        CachedUpdateKind::Supergroup => EventKind::Supergroup,
        CachedUpdateKind::SupergroupFullInfo => EventKind::SupergroupFullInfo,
        CachedUpdateKind::File => EventKind::File,
        CachedUpdateKind::Connection => EventKind::Connection,
        CachedUpdateKind::MessageSend => EventKind::MessageSend,
        CachedUpdateKind::WebAppMessage => EventKind::WebAppMessage,
        CachedUpdateKind::Unknown => EventKind::Unknown,
    }
}

struct WorkflowContext<'server> {
    policy: telegram_core::raw_api::RawPolicy,
    principal: String,
    approval: Option<PlanApproval>,
    approval_verifier: Option<&'server ApprovalVerifier>,
    artifact_root: Option<&'server Path>,
    web_app_artifacts: &'server mut WebAppArtifactStore,
    deadline: Instant,
}

fn run_workflow(
    runtime: &mut CoreRuntime,
    name: &str,
    input: Value,
    context: WorkflowContext<'_>,
) -> Result<WorkflowOutput, WorkflowDispatchError> {
    let WorkflowContext {
        policy,
        principal,
        approval,
        approval_verifier,
        artifact_root,
        web_app_artifacts,
        deadline,
    } = context;
    if approval.is_some()
        && name != "apply_chat_title"
        && name != "apply_custom_emoji_set"
        && name != "apply_story_mutation"
        && name != "apply_terminate_session"
        && name != "apply_star_invoice_payment"
    {
        return Err(WorkflowDispatchError::InvalidInput);
    }
    match name {
        "user_profile" => {
            let input: UserProfileInput = parse(input)?;
            output(
                workflows::user_profile(
                    runtime,
                    &policy,
                    input.target.as_core(),
                    input.include_full_info,
                    deadline,
                )?,
                true,
            )
        }
        "update_profile_name" => {
            let input: ProfileNameInput = parse(input)?;
            let result = workflows::update_profile_name(
                runtime,
                &policy,
                &input.first_name,
                &input.last_name,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "plan_chat_title" => {
            let input: ChatTitleInput = parse(input)?;
            output(
                workflows::plan_chat_title(runtime, input.chat_id, &input.title)?,
                true,
            )
        }
        "apply_chat_title" => {
            let input: ChatTitleInput = parse(input)?;
            let plan = workflows::plan_chat_title(runtime, input.chat_id, &input.title)?;
            let policy = if plan.changed {
                approved_policy(
                    policy,
                    approval,
                    &json!({
                        "@type": "setChatTitle",
                        "chat_id": input.chat_id,
                        "title": input.title,
                    }),
                    approval_verifier,
                )
                .map_err(WorkflowDispatchError::Approval)?
            } else {
                policy
            };
            let result = workflows::apply_chat_title(runtime, &policy, &plan, deadline)?;
            let complete = result.complete;
            output(result, complete)
        }
        "resolve_chat" => {
            let input: TargetInput = parse(input)?;
            let result = workflows::resolve(runtime, &policy, input.target(), deadline)?;
            let complete = matches!(result.state, workflows::ResolutionState::Chat { .. });
            output(result, complete)
        }
        "ensure_membership" => {
            let input: MembershipInput = parse(input)?;
            let result = workflows::ensure_membership(runtime, &policy, input.target(), deadline)?;
            let complete = result.state.complete();
            output(result, complete)
        }
        "load_chat_list" => {
            let input: ChatListInput = parse(input)?;
            output(
                workflows::load_chat_list(
                    runtime,
                    &policy,
                    input.list.into(),
                    input.limit,
                    deadline,
                )?,
                true,
            )
        }
        "inspect_chat" => {
            let input: InspectInput = parse(input)?;
            let result = workflows::inspect_chat(
                runtime,
                &policy,
                input.target.target(),
                input.open,
                deadline,
            )?;
            let complete = result.complete();
            output(result, complete)
        }
        "forum_topics" => {
            let input: ForumTopicsInput = parse(input)?;
            let result = workflows::forum_topics(
                runtime,
                &policy,
                ForumTopicsQuery {
                    chat_id: input.chat_id,
                    query: &input.query,
                    count: input.count,
                    page_limit: input.page_limit,
                },
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "set_forum_topic_closed" => {
            let input: ForumTopicMutationInput = parse(input)?;
            let result = workflows::set_forum_topic_closed(
                runtime,
                &policy,
                input.chat_id,
                input.topic_id,
                input.is_closed,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "chat_history" => {
            let input: HistoryInput = parse(input)?;
            let result = workflows::chat_history(
                runtime,
                &policy,
                HistoryQuery {
                    chat_id: input.chat_id,
                    only_local: input.only_local,
                    mark_read: input.mark_read,
                    page: input.page.into(),
                },
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "search_chat_messages" => {
            let input: SearchInput = parse(input)?;
            let result = workflows::search_chat_messages(
                runtime,
                &policy,
                ChatSearchQuery {
                    chat_id: input.chat_id,
                    query: &input.query,
                    mark_read: input.mark_read,
                    page: input.page.into(),
                },
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "send_text_message" => {
            let input: TextMessageInput = parse(input)?;
            let result = workflows::send_text_message(
                runtime,
                &policy,
                input.chat_id,
                &input.text,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "supergroup_members" => {
            let input: MembersInput = parse(input)?;
            let result = workflows::supergroup_members(
                runtime,
                &policy,
                MembersQuery {
                    supergroup_id: input.supergroup_id,
                    count: input.count,
                    page_limit: input.page_limit,
                },
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "chat_statistics" => {
            let input: StatisticsInput = parse(input)?;
            let result = workflows::chat_statistics(
                runtime,
                &policy,
                input.chat_id,
                input.is_dark,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "resource_statistics" => {
            let input: ResourceStatisticsInput = parse(input)?;
            output(
                workflows::resource_statistics(
                    runtime,
                    &policy,
                    input.only_current_network,
                    deadline,
                )?,
                true,
            )
        }
        "proxy_status" => {
            let _: EmptyInput = parse(input)?;
            output(workflows::proxy_status(runtime, &policy, deadline)?, true)
        }
        "set_proxy_enabled" => {
            let input: ProxyInput = parse(input)?;
            let result =
                workflows::set_proxy_enabled(runtime, &policy, input.proxy_id(), deadline)?;
            let complete = result.complete;
            output(result, complete)
        }
        "resync_after_gap" => {
            let _: EmptyInput = parse(input)?;
            let result = workflows::resync_after_gap(runtime, &policy, deadline)?;
            let complete = result.complete;
            output(result, complete)
        }
        "download_file" => {
            let input: DownloadInput = parse(input)?;
            let result = workflows::download_file(
                runtime,
                &policy,
                DownloadQuery {
                    file_id: input.file_id,
                    priority: input.priority,
                    offset: input.offset,
                    limit: input.limit,
                },
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "cancel_download" => {
            let input: CancelDownloadInput = parse(input)?;
            let result = workflows::cancel_download(
                runtime,
                &policy,
                input.file_id,
                input.only_if_pending,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "upload_sticker_file" => {
            let input: UploadInput = parse(input)?;
            let result = upload_sticker_file(runtime, &policy, input, artifact_root, deadline)?;
            let complete = result.complete;
            output(result, complete)
        }
        "plan_custom_emoji_set" => {
            let input: CustomEmojiSetInput = parse(input)?;
            output(workflows::plan_custom_emoji_set(input.as_core())?, true)
        }
        "apply_custom_emoji_set" => {
            let input: CustomEmojiSetInput = parse(input)?;
            let action = input.as_core();
            let request = workflows::custom_emoji_set_request(action)?;
            let policy = approved_policy(policy, approval, &request, approval_verifier)
                .map_err(WorkflowDispatchError::Approval)?;
            let result = workflows::apply_custom_emoji_set(runtime, &policy, action, deadline)?;
            let complete = result.complete;
            output(result, complete)
        }
        "plan_story_mutation" => {
            let input: StoryMutationInput = parse(input)?;
            output(workflows::plan_story_mutation(input.as_core())?, true)
        }
        "apply_story_mutation" => {
            let input: StoryMutationInput = parse(input)?;
            let action = input.as_core();
            let request = workflows::story_mutation_request(action)?;
            let policy = approved_policy(policy, approval, &request, approval_verifier)
                .map_err(WorkflowDispatchError::Approval)?;
            let result = workflows::apply_story_mutation(runtime, &policy, action, deadline)?;
            let complete = result.complete;
            output(result, complete)
        }
        "inspect_group_call" => {
            let input: GroupCallInput = parse(input)?;
            output(
                workflows::inspect_group_call(runtime, &policy, input.group_call_id, deadline)?,
                true,
            )
        }
        "leave_group_call" => {
            let input: GroupCallInput = parse(input)?;
            let result =
                workflows::leave_group_call(runtime, &policy, input.group_call_id, deadline)?;
            let complete = result.complete;
            output(result, complete)
        }
        "notification_settings" => {
            let input: NotificationScopeInput = parse(input)?;
            output(
                workflows::notification_settings(runtime, &policy, input.scope.into(), deadline)?,
                true,
            )
        }
        "set_notification_settings" => {
            let input: NotificationPatchInput = parse(input)?;
            let result = workflows::set_notification_settings(
                runtime,
                &policy,
                input.scope.into(),
                input.patch.into(),
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "active_sessions" => {
            let _: EmptyInput = parse(input)?;
            output(
                workflows::active_sessions(runtime, &policy, deadline)?,
                true,
            )
        }
        "plan_terminate_session" => {
            let input: SessionInput = parse(input)?;
            output(workflows::plan_terminate_session(input.session_id)?, true)
        }
        "apply_terminate_session" => {
            let input: SessionInput = parse(input)?;
            let request = workflows::terminate_session_request(input.session_id)?;
            let policy = approved_policy(policy, approval, &request, approval_verifier)
                .map_err(WorkflowDispatchError::Approval)?;
            let result =
                workflows::apply_terminate_session(runtime, &policy, input.session_id, deadline)?;
            let complete = result.complete;
            output(result, complete)
        }
        "business_connection" => {
            let input: BusinessConnectionInput = parse(input)?;
            output(
                workflows::business_connection(runtime, &policy, &input.connection_id, deadline)?,
                true,
            )
        }
        "send_business_text" => {
            let input: BusinessMessageInput = parse(input)?;
            let result = workflows::send_business_text(
                runtime,
                &policy,
                &input.connection_id,
                input.chat_id,
                &input.text,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "star_balance" => {
            let _: EmptyInput = parse(input)?;
            output(workflows::star_balance(runtime, &policy, deadline)?, true)
        }
        "plan_star_invoice_payment" => {
            let input: StarInvoiceInput = parse(input)?;
            output(
                workflows::plan_star_invoice_payment(
                    runtime,
                    &policy,
                    &input.invoice_name,
                    deadline,
                )?,
                true,
            )
        }
        "apply_star_invoice_payment" => {
            let input: StarInvoiceInput = parse(input)?;
            let plan = workflows::plan_star_invoice_payment(
                runtime,
                &policy,
                &input.invoice_name,
                deadline,
            )?;
            let request =
                workflows::star_invoice_payment_request(&input.invoice_name, plan.payment_form_id)?;
            let policy = approved_policy(policy, approval, &request, approval_verifier)
                .map_err(WorkflowDispatchError::Approval)?;
            let result = workflows::apply_star_invoice_payment(
                runtime,
                &policy,
                &plan,
                &input.invoice_name,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "start_bot" => {
            let input: StartBotInput = parse(input)?;
            let result = workflows::start_bot(
                runtime,
                &policy,
                input.bot_user_id,
                input.chat_id,
                &input.parameter,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "start_bot_and_wait_reply" => {
            let input: StartBotInput = parse(input)?;
            let result = workflows::start_bot_and_wait_reply(
                runtime,
                &policy,
                input.bot_user_id,
                input.chat_id,
                &input.parameter,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "click_bot_callback" => {
            let input: BotCallbackInput = parse(input)?;
            let result = workflows::click_bot_callback(
                runtime,
                &policy,
                input.chat_id,
                input.message_id,
                input.row,
                input.column,
                deadline,
            )?;
            let complete = result.complete;
            output(result, complete)
        }
        "open_web_app" => {
            let input: WebAppInput = parse(input)?;
            let mut lease = workflows::open_web_app(
                runtime,
                &policy,
                WebAppRequest {
                    chat_id: input.chat_id,
                    bot_user_id: input.bot_user_id,
                    button_url: &input.button_url,
                    application_name: &input.application_name,
                    mode: input.mode.into(),
                },
                deadline,
            )?;
            let require_same_origin = lease.require_same_origin();
            let receipt = lease.wait_message_sent()?;
            let complete = receipt.complete;
            lease.close()?;
            output(
                json!({
                    "receipt": receipt,
                    "require_same_origin": require_same_origin,
                }),
                complete,
            )
        }
        "prepare_web_app_handoff" => {
            let input: WebAppInput = parse(input)?;
            let lease = workflows::open_web_app(
                runtime,
                &policy,
                WebAppRequest {
                    chat_id: input.chat_id,
                    bot_user_id: input.bot_user_id,
                    button_url: &input.button_url,
                    application_name: &input.application_name,
                    mode: input.mode.into(),
                },
                deadline,
            )?;
            let launch_id = lease.launch_id();
            let require_same_origin = lease.require_same_origin();
            let (artifact_handle, artifact_ttl_ms) = web_app_artifacts
                .insert(
                    principal,
                    launch_id,
                    ProtectedString::new(lease.launch_url().expose_secret().to_owned()),
                    require_same_origin,
                    Instant::now(),
                )
                .ok_or(WorkflowDispatchError::ArtifactUnavailable)?;
            let _ = lease.handoff();
            output(
                json!({
                    "launch_id": launch_id,
                    "telegram_status": "prepared",
                    "browser_status": "pending",
                    "artifact_handle": artifact_handle,
                    "artifact_ttl_ms": artifact_ttl_ms,
                    "require_same_origin": require_same_origin,
                    "next_action": "run_browser_then_close",
                }),
                false,
            )
        }
        "close_web_app_handoff" => {
            let input: CloseWebAppInput = parse(input)?;
            workflows::close_web_app_launch(runtime, &policy, input.launch_id, deadline)?;
            output(
                json!({
                    "launch_id": input.launch_id,
                    "telegram_status": "closed",
                    "browser_proof": "separate",
                }),
                true,
            )
        }
        _ => Err(WorkflowDispatchError::Unknown),
    }
}

fn parse<T: DeserializeOwned>(input: Value) -> Result<T, WorkflowDispatchError> {
    serde_json::from_value(input).map_err(|_| WorkflowDispatchError::InvalidInput)
}

fn plan_preview(request: Value) -> Result<Value, CommandErrorCode> {
    let request =
        ValidatedRequest::from_value(request).map_err(|_| CommandErrorCode::InvalidTdjson)?;
    let preview =
        PlanPreview::for_request(&request).map_err(|_| CommandErrorCode::ApprovalDenied)?;
    Ok(json!({
        "method": preview.method,
        "risk": preview.risk,
        "retry": preview.retry,
        "plan_hash": preview.hash.to_hex(),
    }))
}

fn approved_policy(
    policy: telegram_core::raw_api::RawPolicy,
    approval: Option<PlanApproval>,
    request: &Value,
    verifier: Option<&ApprovalVerifier>,
) -> Result<telegram_core::raw_api::RawPolicy, CommandErrorCode> {
    let Some(approval) = approval else {
        return Ok(policy);
    };
    let verifier = verifier.ok_or(CommandErrorCode::ApprovalDenied)?;
    let request = ValidatedRequest::from_value(request.clone())
        .map_err(|_| CommandErrorCode::InvalidTdjson)?;
    let preview =
        PlanPreview::for_request(&request).map_err(|_| CommandErrorCode::ApprovalDenied)?;
    if approval.plan_hash != preview.hash.to_hex() {
        return Err(CommandErrorCode::ApprovalDenied);
    }
    let nonce_hex = Zeroizing::new(approval.nonce.into_inner());
    let signature_hex = Zeroizing::new(approval.signature.into_inner());
    let nonce = decode_hex::<16>(&nonce_hex).ok_or(CommandErrorCode::ApprovalDenied)?;
    let signature = decode_hex::<64>(&signature_hex).ok_or(CommandErrorCode::ApprovalDenied)?;
    let receipt = ApprovalReceipt::new(preview.hash, approval.expires_at_unix, nonce, signature);
    let approval = verifier
        .verify(preview, receipt, SystemTime::now())
        .map_err(|_| CommandErrorCode::ApprovalDenied)?;
    Ok(policy.with_approval(approval))
}

fn decode_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2 {
        return None;
    }
    let mut decoded = [0; N];
    for (target, pair) in decoded.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        *target = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
    }
    Some(decoded)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

struct WorkflowOutput {
    result: Value,
    complete: bool,
}

fn output(
    value: impl serde::Serialize,
    complete: bool,
) -> Result<WorkflowOutput, WorkflowDispatchError> {
    Ok(WorkflowOutput {
        result: serde_json::to_value(value).expect("workflow result is serializable"),
        complete,
    })
}

#[derive(Debug)]
enum WorkflowDispatchError {
    Unknown,
    InvalidInput,
    ArtifactUnavailable,
    Approval(CommandErrorCode),
    Core(ChatWorkflowError),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UserProfileInput {
    target: UserTargetInput,
    include_full_info: bool,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum UserTargetInput {
    #[serde(rename = "self")]
    SelfUser,
    Id {
        user_id: i64,
    },
    PublicUsername {
        username: String,
    },
}

impl UserTargetInput {
    fn as_core(&self) -> UserTarget<'_> {
        match self {
            Self::SelfUser => UserTarget::SelfUser,
            Self::Id { user_id } => UserTarget::Id(*user_id),
            Self::PublicUsername { username } => UserTarget::PublicUsername(username),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileNameInput {
    first_name: String,
    last_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatTitleInput {
    chat_id: i64,
    title: String,
}

impl From<ChatWorkflowError> for WorkflowDispatchError {
    fn from(error: ChatWorkflowError) -> Self {
        Self::Core(error)
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum TargetInput {
    Id { chat_id: i64 },
    PublicUsername { username: String },
    PublicLink { url: String },
    InviteLink { url: String },
}

impl TargetInput {
    fn target(&self) -> ChatTarget<'_> {
        match self {
            Self::Id { chat_id } => ChatTarget::Id(*chat_id),
            Self::PublicUsername { username } => ChatTarget::PublicUsername(username),
            Self::PublicLink { url } => ChatTarget::PublicLink(url),
            Self::InviteLink { url } => ChatTarget::InviteLink(url),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum MembershipInput {
    ChatId { chat_id: i64 },
    InviteLink { url: String },
}

impl MembershipInput {
    fn target(&self) -> MembershipTarget<'_> {
        match self {
            Self::ChatId { chat_id } => MembershipTarget::ChatId(*chat_id),
            Self::InviteLink { url } => MembershipTarget::InviteLink(url),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChatListInput {
    list: ChatListKind,
    limit: i32,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ChatListKind {
    Main,
    Archive,
    Folder { folder_id: i32 },
}

impl From<ChatListKind> for ChatList {
    fn from(value: ChatListKind) -> Self {
        match value {
            ChatListKind::Main => Self::Main,
            ChatListKind::Archive => Self::Archive,
            ChatListKind::Folder { folder_id } => Self::Folder(folder_id),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectInput {
    target: TargetInput,
    open: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForumTopicsInput {
    chat_id: i64,
    query: String,
    count: usize,
    page_limit: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForumTopicMutationInput {
    chat_id: i64,
    topic_id: i32,
    is_closed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageInput {
    count: usize,
    min_date: Option<i32>,
    page_limit: i32,
}

impl From<PageInput> for PageOptions {
    fn from(value: PageInput) -> Self {
        Self {
            count: value.count,
            min_date: value.min_date,
            page_limit: value.page_limit,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryInput {
    chat_id: i64,
    only_local: bool,
    mark_read: bool,
    page: PageInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchInput {
    chat_id: i64,
    query: String,
    mark_read: bool,
    page: PageInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextMessageInput {
    chat_id: i64,
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MembersInput {
    supergroup_id: i64,
    count: usize,
    page_limit: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatisticsInput {
    chat_id: i64,
    is_dark: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceStatisticsInput {
    only_current_network: bool,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum ProxyInput {
    Enable { proxy_id: i32 },
    Disable,
}

impl ProxyInput {
    fn proxy_id(&self) -> Option<i32> {
        match self {
            Self::Enable { proxy_id } => Some(*proxy_id),
            Self::Disable => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyInput {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DownloadInput {
    file_id: i32,
    priority: i32,
    offset: i64,
    limit: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CancelDownloadInput {
    file_id: i32,
    only_if_pending: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadInput {
    user_id: i64,
    format: StickerFormatInput,
    source: FileSourceInput,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StickerFormatInput {
    Webp,
    Tgs,
    Webm,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum CustomEmojiSetInput {
    Create {
        user_id: i64,
        title: String,
        name: String,
        format: StickerFormatInput,
        sticker_file_id: i32,
        emojis: String,
        needs_repainting: bool,
    },
    Add {
        user_id: i64,
        set_id: i64,
        name: String,
        format: StickerFormatInput,
        sticker_file_id: i32,
        emojis: String,
    },
    Delete {
        set_id: i64,
        name: String,
    },
}

impl CustomEmojiSetInput {
    fn as_core(&self) -> CustomEmojiSetAction<'_> {
        match self {
            Self::Create {
                user_id,
                title,
                name,
                format,
                sticker_file_id,
                emojis,
                needs_repainting,
            } => CustomEmojiSetAction::Create {
                user_id: *user_id,
                title,
                name,
                format: (*format).into(),
                sticker_file_id: *sticker_file_id,
                emojis,
                needs_repainting: *needs_repainting,
            },
            Self::Add {
                user_id,
                set_id,
                name,
                format,
                sticker_file_id,
                emojis,
            } => CustomEmojiSetAction::Add {
                user_id: *user_id,
                set_id: *set_id,
                name,
                format: (*format).into(),
                sticker_file_id: *sticker_file_id,
                emojis,
            },
            Self::Delete { set_id, name } => CustomEmojiSetAction::Delete {
                set_id: *set_id,
                name,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum StoryMutationInput {
    PostPhoto {
        chat_id: i64,
        photo_file_id: i32,
        caption: String,
        privacy: StoryPrivacyInput,
        active_period: i32,
        is_posted_to_chat_page: bool,
        protect_content: bool,
    },
    Delete {
        story_poster_chat_id: i64,
        story_id: i32,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StoryPrivacyInput {
    Everyone { except_user_ids: Vec<i64> },
    Contacts { except_user_ids: Vec<i64> },
    CloseFriends,
    SelectedUsers { user_ids: Vec<i64> },
}

impl StoryMutationInput {
    fn as_core(&self) -> StoryAction<'_> {
        match self {
            Self::PostPhoto {
                chat_id,
                photo_file_id,
                caption,
                privacy,
                active_period,
                is_posted_to_chat_page,
                protect_content,
            } => StoryAction::PostPhoto {
                chat_id: *chat_id,
                photo_file_id: *photo_file_id,
                caption,
                privacy: privacy.as_core(),
                active_period: *active_period,
                is_posted_to_chat_page: *is_posted_to_chat_page,
                protect_content: *protect_content,
            },
            Self::Delete {
                story_poster_chat_id,
                story_id,
            } => StoryAction::Delete {
                story_poster_chat_id: *story_poster_chat_id,
                story_id: *story_id,
            },
        }
    }
}

impl StoryPrivacyInput {
    fn as_core(&self) -> StoryPrivacy<'_> {
        match self {
            Self::Everyone { except_user_ids } => StoryPrivacy::Everyone(except_user_ids),
            Self::Contacts { except_user_ids } => StoryPrivacy::Contacts(except_user_ids),
            Self::CloseFriends => StoryPrivacy::CloseFriends,
            Self::SelectedUsers { user_ids } => StoryPrivacy::SelectedUsers(user_ids),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupCallInput {
    group_call_id: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NotificationScopeInput {
    scope: NotificationScopeValue,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NotificationScopeValue {
    #[serde(rename = "private_chats")]
    Private,
    #[serde(rename = "group_chats")]
    Group,
    #[serde(rename = "channel_chats")]
    Channel,
}

impl From<NotificationScopeValue> for NotificationScope {
    fn from(value: NotificationScopeValue) -> Self {
        match value {
            NotificationScopeValue::Private => Self::PrivateChats,
            NotificationScopeValue::Group => Self::GroupChats,
            NotificationScopeValue::Channel => Self::ChannelChats,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NotificationPatchInput {
    scope: NotificationScopeValue,
    patch: NotificationSettingsPatchInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NotificationSettingsPatchInput {
    mute_for: Option<i32>,
    sound_id: Option<i64>,
    show_preview: Option<bool>,
    use_default_mute_stories: Option<bool>,
    mute_stories: Option<bool>,
    story_sound_id: Option<i64>,
    show_story_poster: Option<bool>,
    disable_pinned_message_notifications: Option<bool>,
    disable_mention_notifications: Option<bool>,
}

impl From<NotificationSettingsPatchInput> for NotificationSettingsPatch {
    fn from(value: NotificationSettingsPatchInput) -> Self {
        Self {
            mute_for: value.mute_for,
            sound_id: value.sound_id,
            show_preview: value.show_preview,
            use_default_mute_stories: value.use_default_mute_stories,
            mute_stories: value.mute_stories,
            story_sound_id: value.story_sound_id,
            show_story_poster: value.show_story_poster,
            disable_pinned_message_notifications: value.disable_pinned_message_notifications,
            disable_mention_notifications: value.disable_mention_notifications,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionInput {
    session_id: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BusinessConnectionInput {
    connection_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BusinessMessageInput {
    connection_id: String,
    chat_id: i64,
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StarInvoiceInput {
    invoice_name: String,
}

impl From<StickerFormatInput> for StickerFormat {
    fn from(value: StickerFormatInput) -> Self {
        match value {
            StickerFormatInput::Webp => Self::Webp,
            StickerFormatInput::Tgs => Self::Tgs,
            StickerFormatInput::Webm => Self::Webm,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum FileSourceInput {
    Id {
        id: i32,
    },
    Remote {
        id: String,
    },
    Local {
        path: PathBuf,
    },
    Generated {
        original_path: PathBuf,
        conversion: String,
        expected_size: i64,
    },
}

fn upload_sticker_file(
    runtime: &mut CoreRuntime,
    policy: &telegram_core::raw_api::RawPolicy,
    input: UploadInput,
    artifact_root: Option<&Path>,
    deadline: Instant,
) -> Result<workflows::FileTransferReceipt, WorkflowDispatchError> {
    let UploadInput {
        user_id,
        format,
        source,
    } = input;
    let format = format.into();
    let receipt = match source {
        FileSourceInput::Id { id } => workflows::upload_sticker_file(
            runtime,
            policy,
            user_id,
            format,
            InputFileSource::Id(id),
            deadline,
        ),
        FileSourceInput::Remote { id } => workflows::upload_sticker_file(
            runtime,
            policy,
            user_id,
            format,
            InputFileSource::Remote(&id),
            deadline,
        ),
        FileSourceInput::Local { path } => {
            let path = scoped_artifact_path(artifact_root, &path)?;
            workflows::upload_sticker_file(
                runtime,
                policy,
                user_id,
                format,
                InputFileSource::Local(&path),
                deadline,
            )
        }
        FileSourceInput::Generated {
            original_path,
            conversion,
            expected_size,
        } => {
            let original_path = scoped_artifact_path(artifact_root, &original_path)?;
            workflows::upload_sticker_file(
                runtime,
                policy,
                user_id,
                format,
                InputFileSource::Generated {
                    original_path: &original_path,
                    conversion: &conversion,
                    expected_size,
                },
                deadline,
            )
        }
    }?;
    Ok(receipt)
}

fn scoped_artifact_path(
    root: Option<&Path>,
    path: &Path,
) -> Result<PathBuf, WorkflowDispatchError> {
    let root = root.ok_or(WorkflowDispatchError::InvalidInput)?;
    if !root.is_absolute() || !path.is_absolute() {
        return Err(WorkflowDispatchError::InvalidInput);
    }
    let root = fs::canonicalize(root).map_err(|_| WorkflowDispatchError::InvalidInput)?;
    let path = fs::canonicalize(path).map_err(|_| WorkflowDispatchError::InvalidInput)?;
    if !path.starts_with(&root)
        || !fs::metadata(&path)
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
    {
        return Err(WorkflowDispatchError::InvalidInput);
    }
    Ok(path)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartBotInput {
    bot_user_id: i64,
    chat_id: i64,
    parameter: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BotCallbackInput {
    chat_id: i64,
    message_id: i64,
    row: usize,
    column: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WebAppInput {
    chat_id: i64,
    bot_user_id: i64,
    button_url: String,
    application_name: String,
    mode: WebAppModeInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloseWebAppInput {
    launch_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WebAppModeInput {
    Compact,
    FullSize,
    FullScreen,
}

impl From<WebAppModeInput> for WebAppMode {
    fn from(value: WebAppModeInput) -> Self {
        match value {
            WebAppModeInput::Compact => Self::Compact,
            WebAppModeInput::FullSize => Self::FullSize,
            WebAppModeInput::FullScreen => Self::FullScreen,
        }
    }
}

fn workflow_error(error: WorkflowDispatchError) -> CommandErrorCode {
    match error {
        WorkflowDispatchError::Unknown => CommandErrorCode::WorkflowNotFound,
        WorkflowDispatchError::InvalidInput => CommandErrorCode::InvalidWorkflowInput,
        WorkflowDispatchError::ArtifactUnavailable => CommandErrorCode::WebAppArtifactUnavailable,
        WorkflowDispatchError::Approval(code) => code,
        WorkflowDispatchError::Core(ChatWorkflowError::Call(error)) => raw_error(error),
        WorkflowDispatchError::Core(ChatWorkflowError::PrerequisiteMissing { .. }) => {
            CommandErrorCode::WorkflowPrerequisiteMissing
        }
        WorkflowDispatchError::Core(ChatWorkflowError::CapabilityDenied { .. }) => {
            CommandErrorCode::WorkflowCapabilityDenied
        }
        WorkflowDispatchError::Core(ChatWorkflowError::ResyncRequired { .. }) => {
            CommandErrorCode::WorkflowResyncRequired
        }
        WorkflowDispatchError::Core(ChatWorkflowError::NoResyncRequired) => {
            CommandErrorCode::WorkflowNoResyncRequired
        }
        WorkflowDispatchError::Core(
            ChatWorkflowError::InvalidTarget
            | ChatWorkflowError::InvalidLimit
            | ChatWorkflowError::InvalidPageOptions
            | ChatWorkflowError::InvalidFileTransfer
            | ChatWorkflowError::InvalidStickerSetMutation
            | ChatWorkflowError::InvalidStoryMutation
            | ChatWorkflowError::InvalidGroupCall
            | ChatWorkflowError::InvalidNotificationSettings
            | ChatWorkflowError::InvalidSessionTarget
            | ChatWorkflowError::InvalidProfileInput
            | ChatWorkflowError::InvalidChatConfiguration
            | ChatWorkflowError::InvalidBotInteraction,
        ) => CommandErrorCode::InvalidWorkflowInput,
        WorkflowDispatchError::Core(_) => CommandErrorCode::WorkflowFailed,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    Accept(io::ErrorKind),
    ClientIo(io::ErrorKind),
    SerializeResponse,
    MissingAuthorizationState,
    Authorization(AuthorizationError),
}

impl fmt::Display for ServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accept(kind) => write!(formatter, "profile socket accept failed: {kind:?}"),
            Self::ClientIo(kind) => write!(formatter, "profile client IO failed: {kind:?}"),
            Self::SerializeResponse => formatter.write_str("lease response serialization failed"),
            Self::MissingAuthorizationState => {
                formatter.write_str("daemon authorization state is missing")
            }
            Self::Authorization(error) => write!(formatter, "authorization broker failed: {error}"),
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
        let telemetry = Telemetry::default();
        let scheduler = AccountScheduler::with_telemetry(
            crate::scheduler::serial_daemon_budgets(),
            telemetry.clone(),
        )
        .unwrap();
        let mut server = LeaseServer::new(
            LeaseManager::with_telemetry([RiskScope::Read], telemetry.clone()),
            scheduler,
            telemetry,
            IdempotencyJournal::open(root.join("idempotency.jsonl")).unwrap(),
            AuditLog::open(root.join("audit.jsonl")).unwrap(),
        );

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
            DaemonResponse::SessionStatus {
                metrics: Box::new(telegram_protocol::OperationalMetrics {
                    active_leases_max: 1,
                    ..Default::default()
                }),
            }
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
        assert!(matches!(
            exchange(
                &mut server,
                &socket,
                DaemonRequest::TdPreview {
                    request: json!({
                        "@type": "setChatTitle",
                        "chat_id": 7,
                        "title": "Title",
                    }),
                }
            ),
            DaemonResponse::TdPlanPreview { preview }
                if preview["method"] == "setChatTitle"
                    && preview["risk"] == "admin"
                    && preview["plan_hash"].as_str().is_some_and(|hash| hash.len() == 64)
        ));
        assert_eq!(
            exchange(&mut server, &socket, DaemonRequest::SchemaVersion),
            DaemonResponse::CommandError {
                code: CommandErrorCode::RuntimeUnavailable,
            }
        );
        let DaemonResponse::WorkflowList { workflows } =
            exchange(&mut server, &socket, DaemonRequest::WorkflowList)
        else {
            panic!("expected workflow list")
        };
        assert!(workflows.iter().any(|name| name == "chat_history"));
        assert!(workflows.iter().any(|name| name == "open_web_app"));
        assert_eq!(
            exchange(
                &mut server,
                &socket,
                DaemonRequest::WorkflowDescribe {
                    workflow: "chat_history".to_owned(),
                }
            ),
            DaemonResponse::WorkflowDescription {
                workflow: "chat_history".to_owned(),
                input_example: json!({
                    "chat_id": 0,
                    "only_local": false,
                    "mark_read": false,
                    "page": {"count": 100, "min_date": null, "page_limit": 100},
                }),
            }
        );
        assert!(parse::<TargetInput>(json!({
            "kind": "id",
            "chat_id": 7,
            "unexpected": true,
        }))
        .is_err());
        assert_eq!(server.leases.active_count(), 0);

        drop(socket);
        drop(ownership);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn event_cursor_reports_retention_and_unobserved_updates() {
        let mut events = EventBuffer::new(2);
        events.start_at(10);
        assert_eq!(events.since(None), (Vec::new(), 10, false));

        events.record(11, CachedUpdateKind::User);
        events.record(12, CachedUpdateKind::Chat);
        events.record(13, CachedUpdateKind::File);
        assert_eq!(
            events.since(Some(10)),
            (
                vec![
                    EventRecord {
                        sequence: 12,
                        kind: EventKind::Chat,
                    },
                    EventRecord {
                        sequence: 13,
                        kind: EventKind::File,
                    },
                ],
                13,
                true,
            )
        );

        events.reconcile(Some(15));
        assert_eq!(events.events.back().unwrap().kind, EventKind::Gap);
    }

    #[test]
    fn authorization_broker_exposes_id_but_redacts_protected_input() {
        let mut broker = AuthorizationBroker::default();
        broker
            .observe(&json!({"@type": "authorizationStateWaitPhoneNumber"}))
            .unwrap();
        let (state, challenge_id) = broker.status();
        assert_eq!(state, LoginState::PhoneNumber);
        let challenge_id = challenge_id.unwrap();
        let canary = "AUTH_INPUT_CANARY";
        let (challenge, request) = broker
            .submit(
                challenge_id,
                LoginInput::PhoneNumber {
                    value: telegram_protocol::ProtectedString::new(canary.to_owned()),
                },
            )
            .unwrap();
        assert_eq!(request.request_type(), "setAuthenticationPhoneNumber");
        assert!(!format!("{request:?}").contains(canary));
        broker.machine.submission_failed(challenge).unwrap();
    }

    #[test]
    fn artifact_paths_are_confined_to_the_daemon_root() {
        use std::os::unix::fs::symlink;

        let (root, _) = temporary_scope();
        let artifacts = root.join("artifacts");
        let outside = root.join("outside.bin");
        fs::create_dir_all(&artifacts).unwrap();
        fs::write(artifacts.join("inside.bin"), b"inside").unwrap();
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, artifacts.join("escape.bin")).unwrap();

        assert_eq!(
            scoped_artifact_path(Some(&artifacts), &artifacts.join("inside.bin")).unwrap(),
            fs::canonicalize(artifacts.join("inside.bin")).unwrap()
        );
        assert!(scoped_artifact_path(Some(&artifacts), &outside).is_err());
        assert!(scoped_artifact_path(Some(&artifacts), &artifacts.join("escape.bin")).is_err());
        assert!(scoped_artifact_path(None, &artifacts.join("inside.bin")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn web_app_artifact_is_owner_scoped_one_shot_and_expiring() {
        let now = Instant::now();
        let mut artifacts = WebAppArtifactStore::default();
        let (handle, ttl_ms) = artifacts
            .insert(
                "runner".to_owned(),
                11,
                ProtectedString::new("INIT_DATA_CANARY".to_owned()),
                true,
                now,
            )
            .unwrap();
        assert_eq!(ttl_ms, WEB_APP_ARTIFACT_TTL.as_millis() as u64);
        assert!(artifacts.take(&handle, "other", now).is_none());
        let artifact = artifacts.take(&handle, "runner", now).unwrap();
        assert_eq!(artifact.launch_id, 11);
        assert!(!format!("{:?}", artifact.url).contains("INIT_DATA_CANARY"));
        assert!(artifacts.take(&handle, "runner", now).is_none());

        let (expired, _) = artifacts
            .insert(
                "runner".to_owned(),
                12,
                ProtectedString::new("EXPIRED_CANARY".to_owned()),
                false,
                now,
            )
            .unwrap();
        assert!(artifacts
            .take(&expired, "runner", now + WEB_APP_ARTIFACT_TTL)
            .is_none());
    }

    #[test]
    fn every_discoverable_workflow_example_matches_its_input_contract() {
        for (name, _) in WORKFLOWS {
            let input = workflow_input_example(name).unwrap();
            let valid = match *name {
                "user_profile" => parse::<UserProfileInput>(input).is_ok(),
                "update_profile_name" => parse::<ProfileNameInput>(input).is_ok(),
                "plan_chat_title" | "apply_chat_title" => parse::<ChatTitleInput>(input).is_ok(),
                "resolve_chat" => parse::<TargetInput>(input).is_ok(),
                "ensure_membership" => parse::<MembershipInput>(input).is_ok(),
                "load_chat_list" => parse::<ChatListInput>(input).is_ok(),
                "inspect_chat" => parse::<InspectInput>(input).is_ok(),
                "forum_topics" => parse::<ForumTopicsInput>(input).is_ok(),
                "set_forum_topic_closed" => parse::<ForumTopicMutationInput>(input).is_ok(),
                "chat_history" => parse::<HistoryInput>(input).is_ok(),
                "search_chat_messages" => parse::<SearchInput>(input).is_ok(),
                "send_text_message" => parse::<TextMessageInput>(input).is_ok(),
                "supergroup_members" => parse::<MembersInput>(input).is_ok(),
                "chat_statistics" => parse::<StatisticsInput>(input).is_ok(),
                "resource_statistics" => parse::<ResourceStatisticsInput>(input).is_ok(),
                "proxy_status" => parse::<EmptyInput>(input).is_ok(),
                "set_proxy_enabled" => parse::<ProxyInput>(input).is_ok(),
                "resync_after_gap" => parse::<EmptyInput>(input).is_ok(),
                "download_file" => parse::<DownloadInput>(input).is_ok(),
                "cancel_download" => parse::<CancelDownloadInput>(input).is_ok(),
                "upload_sticker_file" => parse::<UploadInput>(input).is_ok(),
                "plan_custom_emoji_set" | "apply_custom_emoji_set" => {
                    parse::<CustomEmojiSetInput>(input).is_ok()
                }
                "plan_story_mutation" | "apply_story_mutation" => {
                    parse::<StoryMutationInput>(input).is_ok()
                }
                "inspect_group_call" | "leave_group_call" => parse::<GroupCallInput>(input).is_ok(),
                "notification_settings" => parse::<NotificationScopeInput>(input).is_ok(),
                "set_notification_settings" => parse::<NotificationPatchInput>(input).is_ok(),
                "active_sessions" => parse::<EmptyInput>(input).is_ok(),
                "plan_terminate_session" | "apply_terminate_session" => {
                    parse::<SessionInput>(input).is_ok()
                }
                "business_connection" => parse::<BusinessConnectionInput>(input).is_ok(),
                "send_business_text" => parse::<BusinessMessageInput>(input).is_ok(),
                "star_balance" => parse::<EmptyInput>(input).is_ok(),
                "plan_star_invoice_payment" | "apply_star_invoice_payment" => {
                    parse::<StarInvoiceInput>(input).is_ok()
                }
                "start_bot" | "start_bot_and_wait_reply" => parse::<StartBotInput>(input).is_ok(),
                "click_bot_callback" => parse::<BotCallbackInput>(input).is_ok(),
                "open_web_app" | "prepare_web_app_handoff" => parse::<WebAppInput>(input).is_ok(),
                "close_web_app_handoff" => parse::<CloseWebAppInput>(input).is_ok(),
                _ => false,
            };
            assert!(valid, "invalid input example for {name}");
        }
        assert!(parse::<ProxyInput>(json!({})).is_err());
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
