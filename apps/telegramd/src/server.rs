//! Bounded JSONL lease protocol поверх private profile socket.

use std::collections::VecDeque;
use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use telegram_core::authorization::{
    AuthorizationChallengeKind, AuthorizationMachine, AuthorizationStep,
};
use telegram_core::raw_api::{
    self, PolicyError, RawApiError, SchemaDescription, SchemaSearchResult,
};
use telegram_core::reducer::{AppliedUpdate, CachedUpdateKind, ChatList};
use telegram_core::registry::{AccountKind, SymbolKind};
use telegram_core::runtime::CoreRuntime;
use telegram_core::workflows::{
    self, ChatSearchQuery, ChatTarget, ChatWorkflowError, DownloadQuery, HistoryQuery,
    InputFileSource, MembersQuery, MembershipTarget, PageOptions, StickerFormat, WebAppMode,
    WebAppRequest,
};
use telegram_protocol::{
    CommandErrorCode, DaemonRequest, DaemonResponse, EventKind, EventRecord, LeaseErrorCode,
    LoginState,
};

use crate::lease::LeaseManager;

const MAX_REQUEST_BYTES: u64 = 16 * 1024;
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(5);
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
const EVENT_BUFFER_CAPACITY: usize = 1024;
const WORKFLOWS: &[&str] = &[
    "resolve_chat",
    "ensure_membership",
    "load_chat_list",
    "inspect_chat",
    "chat_history",
    "search_chat_messages",
    "supergroup_members",
    "chat_statistics",
    "resync_after_gap",
    "download_file",
    "upload_sticker_file",
    "start_bot",
    "open_web_app",
];

pub struct LeaseServer {
    leases: LeaseManager,
    ready: bool,
    events: EventBuffer,
}

impl LeaseServer {
    pub fn new(leases: LeaseManager) -> Self {
        Self {
            leases,
            ready: true,
            events: EventBuffer::new(EVENT_BUFFER_CAPACITY),
        }
    }

    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
    }

    pub fn start_events_at(&mut self, sequence: Option<u64>) {
        self.events.start_at(sequence.unwrap_or_default());
    }

    pub fn record_event(&mut self, update: AppliedUpdate) {
        self.events.record(update.sequence.get(), update.kind);
    }

    pub fn poll(
        &mut self,
        listener: &UnixListener,
        runtime: &mut CoreRuntime,
        now: Instant,
    ) -> Result<(), ServerError> {
        self.leases.expire(now);
        loop {
            match self.serve_once(listener, Some(&mut *runtime)) {
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
                active_leases: self.leases.active_count(),
            },
            DaemonRequest::LoginStatus => {
                runtime.map_or_else(runtime_unavailable, |runtime| DaemonResponse::LoginStatus {
                    state: login_state(runtime),
                })
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
            DaemonRequest::WorkflowList => DaemonResponse::WorkflowList {
                workflows: WORKFLOWS.iter().map(|name| (*name).to_owned()).collect(),
            },
            DaemonRequest::WorkflowRun {
                lease_id,
                principal,
                workflow,
                input,
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
                let result = run_workflow(runtime, &policy, &workflow, input, deadline);
                self.events.reconcile(
                    runtime
                        .state()
                        .last_sequence()
                        .map(telegram_core::reducer::UpdateSequence::get),
                );
                match result {
                    Ok(result) => DaemonResponse::WorkflowResult { workflow, result },
                    Err(error) => DaemonResponse::CommandError {
                        code: workflow_error(error),
                    },
                }
            }
            DaemonRequest::EventsWatch {
                lease_id,
                principal,
                after,
            } => {
                if let Err(code) =
                    self.leases
                        .raw_policy(&lease_id, &principal, AccountKind::RegularUser, now)
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

fn login_state(runtime: &CoreRuntime) -> LoginState {
    let Some(state) = runtime.state().authorization() else {
        return LoginState::Unknown;
    };
    let Ok(step) = AuthorizationMachine::default().observe_state(&state.value) else {
        return LoginState::Unknown;
    };
    match step {
        AuthorizationStep::ParametersRequired { .. } => LoginState::Parameters,
        AuthorizationStep::Ready => LoginState::Ready,
        AuthorizationStep::LoggingOut => LoginState::LoggingOut,
        AuthorizationStep::Closing => LoginState::Closing,
        AuthorizationStep::Closed => LoginState::Closed,
        AuthorizationStep::Challenge(challenge) => match challenge.kind {
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

fn run_workflow(
    runtime: &mut CoreRuntime,
    policy: &telegram_core::raw_api::RawPolicy,
    name: &str,
    input: Value,
    deadline: Instant,
) -> Result<Value, WorkflowDispatchError> {
    match name {
        "resolve_chat" => {
            let input: TargetInput = parse(input)?;
            serialize(workflows::resolve(
                runtime,
                policy,
                input.target(),
                deadline,
            )?)
        }
        "ensure_membership" => {
            let input: MembershipInput = parse(input)?;
            serialize(workflows::ensure_membership(
                runtime,
                policy,
                input.target(),
                deadline,
            )?)
        }
        "load_chat_list" => {
            let input: ChatListInput = parse(input)?;
            serialize(workflows::load_chat_list(
                runtime,
                policy,
                input.list.into(),
                input.limit,
                deadline,
            )?)
        }
        "inspect_chat" => {
            let input: InspectInput = parse(input)?;
            serialize(workflows::inspect_chat(
                runtime,
                policy,
                input.target.target(),
                input.open,
                deadline,
            )?)
        }
        "chat_history" => {
            let input: HistoryInput = parse(input)?;
            serialize(workflows::chat_history(
                runtime,
                policy,
                HistoryQuery {
                    chat_id: input.chat_id,
                    only_local: input.only_local,
                    page: input.page.into(),
                },
                deadline,
            )?)
        }
        "search_chat_messages" => {
            let input: SearchInput = parse(input)?;
            serialize(workflows::search_chat_messages(
                runtime,
                policy,
                ChatSearchQuery {
                    chat_id: input.chat_id,
                    query: &input.query,
                    page: input.page.into(),
                },
                deadline,
            )?)
        }
        "supergroup_members" => {
            let input: MembersInput = parse(input)?;
            serialize(workflows::supergroup_members(
                runtime,
                policy,
                MembersQuery {
                    supergroup_id: input.supergroup_id,
                    count: input.count,
                    page_limit: input.page_limit,
                },
                deadline,
            )?)
        }
        "chat_statistics" => {
            let input: StatisticsInput = parse(input)?;
            serialize(workflows::chat_statistics(
                runtime,
                policy,
                input.chat_id,
                input.is_dark,
                deadline,
            )?)
        }
        "resync_after_gap" => {
            let _: EmptyInput = parse(input)?;
            serialize(workflows::resync_after_gap(runtime, policy, deadline)?)
        }
        "download_file" => {
            let input: DownloadInput = parse(input)?;
            serialize(workflows::download_file(
                runtime,
                policy,
                DownloadQuery {
                    file_id: input.file_id,
                    priority: input.priority,
                    offset: input.offset,
                    limit: input.limit,
                },
                deadline,
            )?)
        }
        "upload_sticker_file" => {
            let input: UploadInput = parse(input)?;
            serialize(workflows::upload_sticker_file(
                runtime,
                policy,
                input.user_id,
                input.format.into(),
                input.source.as_core(),
                deadline,
            )?)
        }
        "start_bot" => {
            let input: StartBotInput = parse(input)?;
            serialize(workflows::start_bot(
                runtime,
                policy,
                input.bot_user_id,
                input.chat_id,
                &input.parameter,
                deadline,
            )?)
        }
        "open_web_app" => {
            let input: WebAppInput = parse(input)?;
            let mut lease = workflows::open_web_app(
                runtime,
                policy,
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
            lease.close()?;
            Ok(json!({
                "receipt": receipt,
                "require_same_origin": require_same_origin,
            }))
        }
        _ => Err(WorkflowDispatchError::Unknown),
    }
}

fn parse<T: DeserializeOwned>(input: Value) -> Result<T, WorkflowDispatchError> {
    serde_json::from_value(input).map_err(|_| WorkflowDispatchError::InvalidInput)
}

fn serialize(value: impl serde::Serialize) -> Result<Value, WorkflowDispatchError> {
    Ok(serde_json::to_value(value).expect("workflow result is serializable"))
}

enum WorkflowDispatchError {
    Unknown,
    InvalidInput,
    Core(ChatWorkflowError),
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
    page: PageInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchInput {
    chat_id: i64,
    query: String,
    page: PageInput,
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
struct UploadInput {
    user_id: i64,
    format: StickerFormatInput,
    source: FileSourceInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum StickerFormatInput {
    Webp,
    Tgs,
    Webm,
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

impl FileSourceInput {
    fn as_core(&self) -> InputFileSource<'_> {
        match self {
            Self::Id { id } => InputFileSource::Id(*id),
            Self::Remote { id } => InputFileSource::Remote(id),
            Self::Local { path } => InputFileSource::Local(path),
            Self::Generated {
                original_path,
                conversion,
                expected_size,
            } => InputFileSource::Generated {
                original_path,
                conversion,
                expected_size: *expected_size,
            },
        }
    }
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
struct WebAppInput {
    chat_id: i64,
    bot_user_id: i64,
    button_url: String,
    application_name: String,
    mode: WebAppModeInput,
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
            | ChatWorkflowError::InvalidFileTransfer,
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
        let DaemonResponse::WorkflowList { workflows } =
            exchange(&mut server, &socket, DaemonRequest::WorkflowList)
        else {
            panic!("expected workflow list")
        };
        assert!(workflows.iter().any(|name| name == "chat_history"));
        assert!(workflows.iter().any(|name| name == "open_web_app"));
        assert!(
            parse::<TargetInput>(json!({
                "kind": "id",
                "chat_id": 7,
                "unexpected": true,
            }))
            .is_err()
        );
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
