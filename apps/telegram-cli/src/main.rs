//! Тонкий local client единственного daemon protocol.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use telegram_protocol::{
    ClientErrorCode, DaemonRequest, DaemonResponse, LeaseErrorCode, LeaseId, LoginInput,
    LoginState, MachineEnvelope, ProtectedString, RiskScope,
};
use zeroize::{Zeroize, Zeroizing};

const DEFAULT_TTL_MS: u64 = 60_000;
const IO_TIMEOUT: Duration = Duration::from_secs(35);
const EXIT_INPUT: u8 = 2;
const EXIT_UNAVAILABLE: u8 = 3;
const EXIT_REJECTED: u8 = 4;
const EXIT_PROTOCOL: u8 = 5;
const EXIT_CANCELLED: u8 = 6;
const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(200);
static RECEIVED_SIGNAL: AtomicI32 = AtomicI32::new(0);

fn main() -> ExitCode {
    execute(env::args().skip(1).collect())
}

fn execute(arguments: Vec<String>) -> ExitCode {
    let default_output = match env::var("TELEGRAM_OUTPUT") {
        Ok(value) => match OutputFormat::parse(&value) {
            Ok(format) => format,
            Err(error) => return finish_error(OutputFormat::Human, error),
        },
        Err(env::VarError::NotPresent) => OutputFormat::Human,
        Err(env::VarError::NotUnicode(_)) => {
            return finish_error(
                OutputFormat::Human,
                CliError::new(ClientErrorCode::InvalidOutputFormat),
            );
        }
    };
    let (format, arguments) = match split_output(arguments, default_output) {
        Ok(invocation) => invocation,
        Err(error) => return finish_error(default_output, error),
    };
    match run(arguments, format) {
        Ok(exit) => exit,
        Err(error) => finish_error(format, error),
    }
}

fn run(arguments: Vec<String>, format: OutputFormat) -> Result<ExitCode, CliError> {
    let profile = env::var("TELEGRAM_PROFILE").unwrap_or_else(|_| "default".to_owned());
    let principal = env::var("TELEGRAM_PRINCIPAL").unwrap_or_else(|_| "telegram-cli".to_owned());
    if arguments == ["login", "tty"] {
        return login_tty(&profile, format);
    }
    let request = command(&arguments, principal)?;
    if format != OutputFormat::Json {
        if let DaemonRequest::EventsWatch {
            lease_id,
            principal,
            after,
        } = request
        {
            install_signal_handlers()?;
            return stream_events(
                &profile,
                lease_id,
                principal,
                after,
                |response| write_response(format, response),
                || RECEIVED_SIGNAL.load(Ordering::Relaxed) != 0,
            );
        }
    }
    let response = exchange(&profile, &request)?;
    let exit = response_exit(&response);
    write_response(format, &response).map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?;
    Ok(exit)
}

fn command(arguments: &[String], principal: String) -> Result<DaemonRequest, CliError> {
    match arguments {
        [session, status] if session == "session" && status == "status" => {
            Ok(DaemonRequest::SessionStatus)
        }
        [status] if status == "status" => Ok(DaemonRequest::SessionStatus),
        [login] if login == "login" => Ok(DaemonRequest::LoginStatus),
        [schema, version] if schema == "schema" && version == "version" => {
            Ok(DaemonRequest::SchemaVersion)
        }
        [schema, capabilities] if schema == "schema" && capabilities == "capabilities" => {
            Ok(DaemonRequest::SchemaCapabilities)
        }
        [schema, search, query @ ..]
            if schema == "schema" && search == "search" && !query.is_empty() =>
        {
            Ok(DaemonRequest::SchemaSearch {
                query: query.join(" "),
            })
        }
        [schema, describe, name] if schema == "schema" && describe == "describe" => {
            Ok(DaemonRequest::SchemaDescribe { name: name.clone() })
        }
        [td, call, lease_id, request] if td == "td" && call == "call" => {
            Ok(DaemonRequest::TdCall {
                lease_id: LeaseId::new(lease_id.clone()),
                principal,
                request: parse_json(request)?,
            })
        }
        [workflow, list] if workflow == "workflow" && list == "list" => {
            Ok(DaemonRequest::WorkflowList)
        }
        [workflow, run, lease_id, name, input] if workflow == "workflow" && run == "run" => {
            Ok(DaemonRequest::WorkflowRun {
                lease_id: LeaseId::new(lease_id.clone()),
                principal,
                workflow: name.clone(),
                input: parse_json(input)?,
            })
        }
        [events, watch, lease_id] if events == "events" && watch == "watch" => {
            Ok(DaemonRequest::EventsWatch {
                lease_id: LeaseId::new(lease_id.clone()),
                principal,
                after: None,
            })
        }
        [events, watch, lease_id, after] if events == "events" && watch == "watch" => {
            Ok(DaemonRequest::EventsWatch {
                lease_id: LeaseId::new(lease_id.clone()),
                principal,
                after: Some(
                    after
                        .parse()
                        .map_err(|_| CliError::new(ClientErrorCode::InvalidArguments))?,
                ),
            })
        }
        [session, hold] if session == "session" && hold == "hold" => {
            Ok(acquire(principal, "read", DEFAULT_TTL_MS)?)
        }
        [session, hold, scopes] if session == "session" && hold == "hold" => {
            Ok(acquire(principal, scopes, DEFAULT_TTL_MS)?)
        }
        [session, hold, scopes, ttl] if session == "session" && hold == "hold" => {
            let ttl = ttl
                .parse()
                .map_err(|_| CliError::new(ClientErrorCode::InvalidArguments))?;
            acquire(principal, scopes, ttl)
        }
        [session, release, lease_id] if session == "session" && release == "release" => {
            Ok(DaemonRequest::LeaseRelease {
                lease_id: LeaseId::new(lease_id.clone()),
                principal,
            })
        }
        _ => Err(CliError::new(ClientErrorCode::InvalidArguments)),
    }
}

fn acquire(principal: String, scopes: &str, ttl_ms: u64) -> Result<DaemonRequest, CliError> {
    let scopes = scopes
        .split(',')
        .map(RiskScope::from_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CliError::new(ClientErrorCode::InvalidArguments))?;
    Ok(DaemonRequest::LeaseAcquire {
        principal,
        scopes,
        ttl_ms,
    })
}

fn exchange(profile: &str, request: &DaemonRequest) -> Result<DaemonResponse, CliError> {
    let path = socket_path(profile)?;
    validate_socket(&path)?;
    let mut stream =
        UnixStream::connect(path).map_err(|_| CliError::new(ClientErrorCode::TransportFailed))?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|_| stream.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|_| CliError::new(ClientErrorCode::TransportFailed))?;
    serde_json::to_writer(&mut stream, request)
        .map_err(|_| CliError::new(ClientErrorCode::TransportFailed))?;
    stream
        .write_all(b"\n")
        .and_then(|_| stream.flush())
        .map_err(|_| CliError::new(ClientErrorCode::TransportFailed))?;

    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .map_err(|_| CliError::new(ClientErrorCode::TransportFailed))?;
    let parsed = serde_json::from_str(&response);
    response.zeroize();
    parsed.map_err(|_| CliError::new(ClientErrorCode::InvalidResponse))
}

fn login_tty(profile: &str, format: OutputFormat) -> Result<ExitCode, CliError> {
    install_signal_handlers()?;
    let mut waiting_for = None;
    loop {
        if RECEIVED_SIGNAL.load(Ordering::Relaxed) != 0 {
            return Err(CliError::new(ClientErrorCode::Cancelled));
        }
        let response = exchange(profile, &DaemonRequest::LoginStatus)?;
        match response {
            response @ DaemonResponse::LoginStatus {
                state: LoginState::Ready,
                ..
            }
            | response @ DaemonResponse::LoginStatus {
                state:
                    LoginState::LoggingOut
                    | LoginState::Closing
                    | LoginState::Closed
                    | LoginState::Unknown,
                ..
            } => {
                let exit = response_exit(&response);
                write_response(format, &response)
                    .map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?;
                return Ok(exit);
            }
            DaemonResponse::LoginStatus {
                state,
                challenge_id: Some(challenge_id),
            } => {
                if waiting_for == Some(challenge_id) {
                    thread::sleep(WATCH_POLL_INTERVAL);
                    continue;
                }
                let Some(input) = tty_login_input(state)? else {
                    waiting_for = Some(challenge_id);
                    thread::sleep(WATCH_POLL_INTERVAL);
                    continue;
                };
                let submitted = exchange(
                    profile,
                    &DaemonRequest::LoginSubmit {
                        challenge_id,
                        input,
                    },
                )?;
                match submitted {
                    DaemonResponse::LoginSubmitted {
                        challenge_id: submitted_id,
                    } if submitted_id == challenge_id => {
                        waiting_for = Some(challenge_id);
                    }
                    response @ (DaemonResponse::CommandError { .. }
                    | DaemonResponse::Error { .. }) => {
                        let exit = response_exit(&response);
                        write_response(format, &response)
                            .map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?;
                        return Ok(exit);
                    }
                    _ => return Err(CliError::new(ClientErrorCode::InvalidResponse)),
                }
            }
            DaemonResponse::LoginStatus {
                challenge_id: None, ..
            } => thread::sleep(WATCH_POLL_INTERVAL),
            response @ (DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }) => {
                let exit = response_exit(&response);
                write_response(format, &response)
                    .map_err(|_| CliError::new(ClientErrorCode::OutputFailed))?;
                return Ok(exit);
            }
            _ => return Err(CliError::new(ClientErrorCode::InvalidResponse)),
        }
    }
}

fn tty_login_input(state: LoginState) -> Result<Option<LoginInput>, CliError> {
    let input = match state {
        LoginState::PhoneNumber | LoginState::PremiumPurchase => LoginInput::PhoneNumber {
            value: read_tty_secret("Телефон: ")?,
        },
        LoginState::Code => LoginInput::AuthenticationCode {
            value: read_tty_secret("Код Telegram: ")?,
        },
        LoginState::Password => LoginInput::Password {
            value: read_tty_secret("Пароль 2FA: ")?,
        },
        LoginState::EmailAddress => LoginInput::EmailAddress {
            value: read_tty_secret("Email: ")?,
        },
        LoginState::EmailCode => LoginInput::EmailCode {
            value: read_tty_secret("Код из email: ")?,
        },
        LoginState::Registration => LoginInput::Registration {
            first_name: read_tty_secret("Имя: ")?,
            last_name: read_tty_secret("Фамилия (можно пустую): ")?,
        },
        LoginState::QrCode => {
            write_tty_notice("Подтвердите текущий QR login на другом устройстве.\n")?;
            return Ok(None);
        }
        LoginState::Parameters => return Ok(None),
        LoginState::Ready
        | LoginState::LoggingOut
        | LoginState::Closing
        | LoginState::Closed
        | LoginState::Unknown => return Ok(None),
    };
    Ok(Some(input))
}

const MAX_TTY_SECRET_BYTES: usize = 4096;

fn read_tty_secret(prompt: &str) -> Result<ProtectedString, CliError> {
    let mut tty = open_tty()?;
    let _echo = EchoGuard::disable(&tty)?;
    tty.write_all(prompt.as_bytes())
        .and_then(|_| tty.flush())
        .map_err(|_| CliError::new(ClientErrorCode::SecureTtyFailed))?;

    let mut bytes = Zeroizing::new(Vec::new());
    loop {
        if RECEIVED_SIGNAL.load(Ordering::Relaxed) != 0 {
            let _ = tty.write_all(b"\n");
            return Err(CliError::new(ClientErrorCode::Cancelled));
        }
        let mut poll = libc::pollfd {
            fd: tty.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: `poll` points to one valid `pollfd` for the duration of the call.
        let ready = unsafe { libc::poll(&mut poll, 1, 100) };
        if ready < 0 {
            if RECEIVED_SIGNAL.load(Ordering::Relaxed) != 0 {
                continue;
            }
            return Err(CliError::new(ClientErrorCode::SecureTtyFailed));
        }
        if ready == 0 {
            continue;
        }
        if poll.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return Err(CliError::new(ClientErrorCode::SecureTtyFailed));
        }
        let mut chunk = [0_u8; 256];
        let read = match tty.read(&mut chunk) {
            Ok(0) => {
                return Err(CliError::new(ClientErrorCode::SecureTtyFailed));
            }
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
            Err(_) => {
                return Err(CliError::new(ClientErrorCode::SecureTtyFailed));
            }
        };
        let newline = chunk[..read].iter().position(|byte| *byte == b'\n');
        let end = newline.unwrap_or(read);
        bytes.extend_from_slice(&chunk[..end]);
        chunk.zeroize();
        if bytes.len() > MAX_TTY_SECRET_BYTES {
            return Err(CliError::new(ClientErrorCode::SecureTtyFailed));
        }
        if newline.is_some() {
            break;
        }
    }
    tty.write_all(b"\n")
        .and_then(|_| tty.flush())
        .map_err(|_| CliError::new(ClientErrorCode::SecureTtyFailed))?;
    let value = match std::str::from_utf8(bytes.as_slice()) {
        Ok(value) => value.trim_end_matches('\r').to_owned(),
        Err(_) => return Err(CliError::new(ClientErrorCode::SecureTtyFailed)),
    };
    Ok(ProtectedString::new(value))
}

fn write_tty_notice(message: &str) -> Result<(), CliError> {
    open_tty()?
        .write_all(message.as_bytes())
        .map_err(|_| CliError::new(ClientErrorCode::SecureTtyFailed))
}

fn open_tty() -> Result<std::fs::File, CliError> {
    let tty = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open("/dev/tty")
        .map_err(|_| CliError::new(ClientErrorCode::SecureTtyUnavailable))?;
    let metadata = tty
        .metadata()
        .map_err(|_| CliError::new(ClientErrorCode::SecureTtyFailed))?;
    if !metadata.file_type().is_char_device() {
        return Err(CliError::new(ClientErrorCode::SecureTtyUnavailable));
    }
    Ok(tty)
}

struct EchoGuard {
    fd: libc::c_int,
    original: libc::termios,
}

impl EchoGuard {
    fn disable(tty: &std::fs::File) -> Result<Self, CliError> {
        let fd = tty.as_raw_fd();
        // SAFETY: `termios` is initialized by `tcgetattr` for this valid TTY descriptor.
        let mut current = unsafe { std::mem::zeroed::<libc::termios>() };
        // SAFETY: both pointers are valid for the duration of these calls.
        if unsafe { libc::tcgetattr(fd, &mut current) } != 0 {
            return Err(CliError::new(ClientErrorCode::SecureTtyFailed));
        }
        let original = current;
        current.c_lflag &= !(libc::ECHO | libc::ECHONL);
        // SAFETY: `current` came from the same descriptor and contains a valid termios value.
        if unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, &current) } != 0 {
            return Err(CliError::new(ClientErrorCode::SecureTtyFailed));
        }
        Ok(Self { fd, original })
    }
}

impl Drop for EchoGuard {
    fn drop(&mut self) {
        // SAFETY: best-effort restoration uses the original termios for the same descriptor.
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

extern "C" fn receive_signal(signal: libc::c_int) {
    RECEIVED_SIGNAL.store(signal, Ordering::Relaxed);
}

fn install_signal_handlers() -> Result<(), CliError> {
    RECEIVED_SIGNAL.store(0, Ordering::Relaxed);
    for signal in [libc::SIGINT, libc::SIGTERM] {
        // SAFETY: `receive_signal` has the required C ABI and performs only an atomic store.
        let previous =
            unsafe { libc::signal(signal, receive_signal as *const () as libc::sighandler_t) };
        if previous == libc::SIG_ERR {
            return Err(CliError::new(ClientErrorCode::TransportFailed));
        }
    }
    Ok(())
}

fn stream_events(
    profile: &str,
    lease_id: LeaseId,
    principal: String,
    mut cursor: Option<u64>,
    mut emit: impl FnMut(&DaemonResponse) -> io::Result<()>,
    cancelled: impl Fn() -> bool,
) -> Result<ExitCode, CliError> {
    let mut heartbeat_at = match renew_watch(profile, &lease_id, &principal) {
        Ok(delay) => Instant::now()
            .checked_add(delay)
            .unwrap_or_else(Instant::now),
        Err(WatchError::Client(error)) => {
            let _ = release_watch(profile, &lease_id, &principal);
            return Err(error);
        }
        Err(WatchError::Rejected(response)) => {
            let exit = response_exit(&response);
            let emitted = emit(&response);
            release_watch(profile, &lease_id, &principal)?;
            if emitted.is_err() {
                return Ok(ExitCode::from(EXIT_CANCELLED));
            }
            return Ok(exit);
        }
    };
    let mut first = true;

    loop {
        if cancelled() {
            release_watch(profile, &lease_id, &principal)?;
            return Err(CliError::new(ClientErrorCode::Cancelled));
        }

        let now = Instant::now();
        if now >= heartbeat_at {
            match renew_watch(profile, &lease_id, &principal) {
                Ok(delay) => {
                    heartbeat_at = now.checked_add(delay).unwrap_or(now);
                }
                Err(WatchError::Client(error)) => {
                    let _ = release_watch(profile, &lease_id, &principal);
                    return Err(error);
                }
                Err(WatchError::Rejected(response)) => {
                    let exit = response_exit(&response);
                    let emitted = emit(&response);
                    release_watch(profile, &lease_id, &principal)?;
                    if emitted.is_err() {
                        return Ok(ExitCode::from(EXIT_CANCELLED));
                    }
                    return Ok(exit);
                }
            }
        }

        let response = match exchange(
            profile,
            &DaemonRequest::EventsWatch {
                lease_id: lease_id.clone(),
                principal: principal.clone(),
                after: cursor,
            },
        ) {
            Ok(response) => response,
            Err(error) => {
                let _ = release_watch(profile, &lease_id, &principal);
                return Err(error);
            }
        };
        match &response {
            DaemonResponse::Events {
                events,
                next_cursor,
                gap,
            } => {
                cursor = Some(*next_cursor);
                if (first || !events.is_empty() || *gap) && emit(&response).is_err() {
                    release_watch(profile, &lease_id, &principal)?;
                    return Ok(ExitCode::from(EXIT_CANCELLED));
                }
            }
            DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. } => {
                let exit = response_exit(&response);
                let emitted = emit(&response);
                release_watch(profile, &lease_id, &principal)?;
                if emitted.is_err() {
                    return Ok(ExitCode::from(EXIT_CANCELLED));
                }
                return Ok(exit);
            }
            _ => {
                release_watch(profile, &lease_id, &principal)?;
                return Err(CliError::new(ClientErrorCode::InvalidResponse));
            }
        }
        first = false;

        let until_heartbeat = heartbeat_at.saturating_duration_since(Instant::now());
        thread::sleep(WATCH_POLL_INTERVAL.min(until_heartbeat));
    }
}

enum WatchError {
    Client(CliError),
    Rejected(DaemonResponse),
}

fn renew_watch(profile: &str, lease_id: &LeaseId, principal: &str) -> Result<Duration, WatchError> {
    let response = exchange(
        profile,
        &DaemonRequest::LeaseHeartbeat {
            lease_id: lease_id.clone(),
            principal: principal.to_owned(),
        },
    )
    .map_err(WatchError::Client)?;
    match response {
        DaemonResponse::LeaseRenewed { lease } => {
            Ok(Duration::from_millis((lease.ttl_ms / 3).max(1)))
        }
        response @ (DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }) => {
            Err(WatchError::Rejected(response))
        }
        _ => Err(WatchError::Client(CliError::new(
            ClientErrorCode::InvalidResponse,
        ))),
    }
}

fn release_watch(profile: &str, lease_id: &LeaseId, principal: &str) -> Result<(), CliError> {
    match exchange(
        profile,
        &DaemonRequest::LeaseRelease {
            lease_id: lease_id.clone(),
            principal: principal.to_owned(),
        },
    )? {
        DaemonResponse::LeaseReleased { .. }
        | DaemonResponse::Error {
            code: LeaseErrorCode::LeaseNotFound | LeaseErrorCode::LeaseExpired,
        } => Ok(()),
        _ => Err(CliError::new(ClientErrorCode::InvalidResponse)),
    }
}

fn socket_path(profile: &str) -> Result<PathBuf, CliError> {
    if profile.is_empty()
        || profile.len() > 48
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(CliError::new(ClientErrorCode::InvalidProfile));
    }
    // SAFETY: geteuid has no preconditions and does not access memory.
    let uid = unsafe { libc::geteuid() };
    Ok(PathBuf::from(format!(
        "/tmp/telegramd-{uid}/{profile}.sock"
    )))
}

fn validate_socket(path: &std::path::Path) -> Result<(), CliError> {
    let parent = path
        .parent()
        .ok_or_else(|| CliError::new(ClientErrorCode::InvalidProfile))?;
    let directory = fs::symlink_metadata(parent)
        .map_err(|_| CliError::new(ClientErrorCode::SocketUnavailable))?;
    let socket = fs::symlink_metadata(path)
        .map_err(|_| CliError::new(ClientErrorCode::SocketUnavailable))?;
    // SAFETY: geteuid has no preconditions and does not access memory.
    let uid = unsafe { libc::geteuid() };
    if !directory.is_dir()
        || directory.uid() != uid
        || directory.mode() & 0o777 != 0o700
        || !socket.file_type().is_socket()
        || socket.uid() != uid
        || socket.nlink() != 1
        || socket.mode() & 0o777 != 0o600
    {
        return Err(CliError::new(ClientErrorCode::UnsafeSocket));
    }
    Ok(())
}

fn parse_json(value: &str) -> Result<serde_json::Value, CliError> {
    serde_json::from_str(value).map_err(|_| CliError::new(ClientErrorCode::InvalidJson))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Human,
    Json,
    Jsonl,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            "jsonl" => Ok(Self::Jsonl),
            _ => Err(CliError::new(ClientErrorCode::InvalidOutputFormat)),
        }
    }
}

fn split_output(
    mut arguments: Vec<String>,
    default: OutputFormat,
) -> Result<(OutputFormat, Vec<String>), CliError> {
    if arguments.first().is_some_and(|value| value == "--output") {
        if arguments.len() < 2 {
            return Err(CliError::new(ClientErrorCode::InvalidOutputFormat));
        }
        let format = OutputFormat::parse(&arguments[1])?;
        arguments.drain(..2);
        Ok((format, arguments))
    } else {
        Ok((default, arguments))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CliError {
    code: ClientErrorCode,
}

impl CliError {
    const fn new(code: ClientErrorCode) -> Self {
        Self { code }
    }

    const fn exit_code(self) -> u8 {
        match self.code {
            ClientErrorCode::InvalidArguments
            | ClientErrorCode::InvalidJson
            | ClientErrorCode::InvalidOutputFormat
            | ClientErrorCode::InvalidProfile => EXIT_INPUT,
            ClientErrorCode::SocketUnavailable
            | ClientErrorCode::UnsafeSocket
            | ClientErrorCode::TransportFailed => EXIT_UNAVAILABLE,
            ClientErrorCode::InvalidResponse | ClientErrorCode::OutputFailed => EXIT_PROTOCOL,
            ClientErrorCode::Cancelled => EXIT_CANCELLED,
            ClientErrorCode::SecureTtyUnavailable | ClientErrorCode::SecureTtyFailed => {
                EXIT_UNAVAILABLE
            }
        }
    }

    const fn message(self) -> &'static str {
        match self.code {
            ClientErrorCode::InvalidArguments => {
                "usage: telegram-cli session ... | login [tty] | schema ... | td call <lease_id> <json> | workflow list|run ... | events watch ..."
            }
            ClientErrorCode::InvalidJson => "неверный JSON input",
            ClientErrorCode::InvalidOutputFormat => "output должен быть human, json или jsonl",
            ClientErrorCode::InvalidProfile => "неверное имя profile",
            ClientErrorCode::SocketUnavailable => "daemon socket недоступен",
            ClientErrorCode::UnsafeSocket => "daemon socket не прошёл проверку безопасности",
            ClientErrorCode::TransportFailed => "обмен с daemon не выполнен",
            ClientErrorCode::InvalidResponse => "daemon вернул неверный protocol response",
            ClientErrorCode::OutputFailed => "не удалось записать output",
            ClientErrorCode::Cancelled => "операция отменена",
            ClientErrorCode::SecureTtyUnavailable => "защищённый /dev/tty недоступен",
            ClientErrorCode::SecureTtyFailed => "защищённый TTY input не выполнен",
        }
    }
}

fn response_exit(response: &DaemonResponse) -> ExitCode {
    if matches!(
        response,
        DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }
    ) {
        ExitCode::from(EXIT_REJECTED)
    } else {
        ExitCode::SUCCESS
    }
}

fn finish_error(format: OutputFormat, error: CliError) -> ExitCode {
    if write_client_error(format, error).is_err() {
        ExitCode::from(EXIT_PROTOCOL)
    } else {
        ExitCode::from(error.exit_code())
    }
}

fn write_response(format: OutputFormat, response: &DaemonResponse) -> io::Result<()> {
    match format {
        OutputFormat::Human => {
            if matches!(
                response,
                DaemonResponse::CommandError { .. } | DaemonResponse::Error { .. }
            ) {
                human_response(&mut io::stderr().lock(), response)
            } else {
                human_response(&mut io::stdout().lock(), response)
            }
        }
        OutputFormat::Json | OutputFormat::Jsonl => write_machine(
            &mut io::stdout().lock(),
            &MachineEnvelope::from_response(response.clone()),
        ),
    }
}

fn write_client_error(format: OutputFormat, error: CliError) -> io::Result<()> {
    match format {
        OutputFormat::Human => writeln!(io::stderr().lock(), "telegram-cli: {}", error.message()),
        OutputFormat::Json | OutputFormat::Jsonl => write_machine(
            &mut io::stdout().lock(),
            &MachineEnvelope::client_error(error.code),
        ),
    }
}

fn write_machine(writer: &mut impl Write, envelope: &MachineEnvelope) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, envelope).map_err(io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn human_response(writer: &mut impl Write, response: &DaemonResponse) -> io::Result<()> {
    match response {
        DaemonResponse::SessionStatus { active_leases } => {
            writeln!(writer, "Активных leases: {active_leases}")
        }
        DaemonResponse::LoginStatus {
            state,
            challenge_id,
        } => writeln!(
            writer,
            "Авторизация: {state:?}{}",
            challenge_id.map_or(String::new(), |id| format!("; challenge={id}")),
        ),
        DaemonResponse::LoginSubmitted { challenge_id } => {
            writeln!(writer, "Challenge {challenge_id} принят")
        }
        DaemonResponse::SchemaVersion { version } => pretty(writer, version),
        DaemonResponse::SchemaCapabilities { capabilities } => pretty(writer, capabilities),
        DaemonResponse::SchemaSearchResults { results } => {
            let Some(results) = results.as_array() else {
                return pretty(writer, results);
            };
            if results.is_empty() {
                return writeln!(writer, "Ничего не найдено.");
            }
            for result in results {
                let name = result.get("name").and_then(serde_json::Value::as_str);
                let kind = result.get("kind").and_then(serde_json::Value::as_str);
                writeln!(
                    writer,
                    "{}{}",
                    name.unwrap_or("unknown"),
                    kind.map_or(String::new(), |kind| format!(" ({kind})")),
                )?;
            }
            Ok(())
        }
        DaemonResponse::SchemaDescription { description } => pretty(writer, description),
        DaemonResponse::TdResult { result } => pretty(writer, result),
        DaemonResponse::WorkflowList { workflows } => {
            for workflow in workflows {
                writeln!(writer, "{workflow}")?;
            }
            Ok(())
        }
        DaemonResponse::WorkflowResult {
            workflow,
            result,
            complete,
        } => {
            writeln!(writer, "Workflow {workflow}: complete={complete}")?;
            pretty(writer, result)
        }
        DaemonResponse::Events {
            events,
            next_cursor,
            gap,
        } => {
            for event in events {
                writeln!(writer, "{} {:?}", event.sequence, event.kind)?;
            }
            writeln!(writer, "cursor={next_cursor} gap={gap}")
        }
        DaemonResponse::LeaseGranted { lease } => writeln!(
            writer,
            "Lease {}: ttl={}ms scopes={}",
            lease.lease_id.as_str(),
            lease.expires_in_ms,
            lease
                .scopes
                .iter()
                .map(|scope| scope.as_str())
                .collect::<Vec<_>>()
                .join(","),
        ),
        DaemonResponse::LeaseRenewed { lease } => writeln!(
            writer,
            "Lease {} продлён: ttl={}ms",
            lease.lease_id.as_str(),
            lease.expires_in_ms,
        ),
        DaemonResponse::LeaseReleased { lease_id } => {
            writeln!(writer, "Lease {} освобождён", lease_id.as_str())
        }
        DaemonResponse::CommandError { code } => writeln!(writer, "Daemon error: {code:?}"),
        DaemonResponse::Error { code } => writeln!(writer, "Lease error: {code:?}"),
    }
}

fn pretty(writer: &mut impl Write, value: &serde_json::Value) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use std::fs::{DirBuilder, Permissions};
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use telegram_protocol::{LeaseView, MachineStatus};

    use super::*;

    #[test]
    fn commands_build_closed_protocol_requests() {
        assert_eq!(
            command(&["status".to_owned()], "cli".to_owned()).unwrap(),
            DaemonRequest::SessionStatus
        );
        assert_eq!(
            command(&["login".to_owned()], "cli".to_owned()).unwrap(),
            DaemonRequest::LoginStatus
        );
        assert!(command(
            &[
                "login".to_owned(),
                "tty".to_owned(),
                "never-a-secret-argument".to_owned(),
            ],
            "cli".to_owned(),
        )
        .is_err());
        assert_eq!(
            command(
                &[
                    "session".to_owned(),
                    "hold".to_owned(),
                    "read,send".to_owned(),
                    "500".to_owned(),
                ],
                "cli".to_owned(),
            )
            .unwrap(),
            DaemonRequest::LeaseAcquire {
                principal: "cli".to_owned(),
                scopes: vec![RiskScope::Read, RiskScope::Send],
                ttl_ms: 500,
            }
        );
        assert_eq!(
            command(
                &[
                    "workflow".to_owned(),
                    "run".to_owned(),
                    "lease".to_owned(),
                    "chat_history".to_owned(),
                    r#"{"chat_id":7,"only_local":false,"page":{"count":1,"min_date":null,"page_limit":100}}"#.to_owned(),
                ],
                "cli".to_owned(),
            )
            .unwrap(),
            DaemonRequest::WorkflowRun {
                lease_id: LeaseId::new("lease".to_owned()),
                principal: "cli".to_owned(),
                workflow: "chat_history".to_owned(),
                input: serde_json::json!({
                    "chat_id": 7,
                    "only_local": false,
                    "page": {"count": 1, "min_date": null, "page_limit": 100},
                }),
            }
        );
        assert!(command(
            &[
                "session".to_owned(),
                "hold".to_owned(),
                "unknown".to_owned(),
            ],
            "cli".to_owned(),
        )
        .is_err());
        assert_eq!(
            command(
                &[
                    "schema".to_owned(),
                    "search".to_owned(),
                    "chat".to_owned(),
                    "statistics".to_owned(),
                ],
                "cli".to_owned(),
            )
            .unwrap(),
            DaemonRequest::SchemaSearch {
                query: "chat statistics".to_owned(),
            }
        );
        assert_eq!(
            command(
                &[
                    "td".to_owned(),
                    "call".to_owned(),
                    "lease".to_owned(),
                    r#"{"@type":"getMe"}"#.to_owned(),
                ],
                "cli".to_owned(),
            )
            .unwrap(),
            DaemonRequest::TdCall {
                lease_id: LeaseId::new("lease".to_owned()),
                principal: "cli".to_owned(),
                request: serde_json::json!({"@type": "getMe"}),
            }
        );
        assert_eq!(
            command(
                &[
                    "events".to_owned(),
                    "watch".to_owned(),
                    "lease".to_owned(),
                    "42".to_owned(),
                ],
                "cli".to_owned(),
            )
            .unwrap(),
            DaemonRequest::EventsWatch {
                lease_id: LeaseId::new("lease".to_owned()),
                principal: "cli".to_owned(),
                after: Some(42),
            }
        );
    }

    #[test]
    fn machine_envelope_is_versioned_and_keeps_partial_and_error_structured() {
        let ok = MachineEnvelope::from_response(DaemonResponse::SessionStatus { active_leases: 2 });
        assert_eq!(
            serde_json::to_value(&ok).unwrap(),
            serde_json::json!({
                "version": 1,
                "status": "ok",
                "data": {"type": "session_status", "active_leases": 2},
            })
        );

        let partial = MachineEnvelope::from_response(DaemonResponse::WorkflowResult {
            workflow: "chat_history".to_owned(),
            result: serde_json::json!({"complete": false}),
            complete: false,
        });
        assert_eq!(partial.status(), MachineStatus::Partial);
        assert_eq!(
            serde_json::to_value(MachineEnvelope::client_error(
                ClientErrorCode::InvalidArguments,
            ))
            .unwrap(),
            serde_json::json!({
                "version": 1,
                "status": "error",
                "error": {"domain": "client", "code": "invalid_arguments"},
            })
        );
    }

    #[test]
    fn output_selection_and_human_digest_are_bounded() {
        let (format, command) = split_output(
            vec![
                "--output".to_owned(),
                "jsonl".to_owned(),
                "status".to_owned(),
            ],
            OutputFormat::Human,
        )
        .unwrap();
        assert_eq!(format, OutputFormat::Jsonl);
        assert_eq!(command, vec!["status"]);
        assert_eq!(
            CliError::new(ClientErrorCode::UnsafeSocket).exit_code(),
            EXIT_UNAVAILABLE
        );

        let mut output = Vec::new();
        human_response(
            &mut output,
            &DaemonResponse::LoginStatus {
                state: telegram_protocol::LoginState::Ready,
                challenge_id: None,
            },
        )
        .unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "Авторизация: Ready\n");
    }

    #[test]
    fn client_uses_private_jsonl_profile_socket() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let profile = format!("cli-{}-{nonce:x}", std::process::id());
        let path = socket_path(&profile).unwrap();
        let directory = path.parent().unwrap();
        match DirBuilder::new().mode(0o700).create(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("can't create test socket directory: {error}"),
        }
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(&mut stream).read_line(&mut request).unwrap();
            assert_eq!(
                serde_json::from_str::<DaemonRequest>(&request).unwrap(),
                DaemonRequest::SessionStatus
            );
            serde_json::to_writer(
                &mut stream,
                &DaemonResponse::SessionStatus { active_leases: 2 },
            )
            .unwrap();
            stream.write_all(b"\n").unwrap();
        });

        assert_eq!(
            exchange(&profile, &DaemonRequest::SessionStatus).unwrap(),
            DaemonResponse::SessionStatus { active_leases: 2 }
        );
        server.join().unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn watch_releases_lease_on_cancellation_or_pipe_close() {
        for cancelled in [true, false] {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let profile = format!("watch-{}-{nonce:x}", std::process::id());
            let path = socket_path(&profile).unwrap();
            let directory = path.parent().unwrap();
            match DirBuilder::new().mode(0o700).create(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("can't create test socket directory: {error}"),
            }
            let listener = UnixListener::bind(&path).unwrap();
            fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();
            let lease_id = LeaseId::new(format!("lease-{nonce:x}"));
            let server_lease = lease_id.clone();
            let server = thread::spawn(move || {
                let respond = |expected: DaemonRequest, response: DaemonResponse| {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut request = String::new();
                    BufReader::new(&mut stream).read_line(&mut request).unwrap();
                    assert_eq!(
                        serde_json::from_str::<DaemonRequest>(&request).unwrap(),
                        expected
                    );
                    serde_json::to_writer(&mut stream, &response).unwrap();
                    stream.write_all(b"\n").unwrap();
                };
                respond(
                    DaemonRequest::LeaseHeartbeat {
                        lease_id: server_lease.clone(),
                        principal: "cli".to_owned(),
                    },
                    DaemonResponse::LeaseRenewed {
                        lease: LeaseView {
                            lease_id: server_lease.clone(),
                            principal: "cli".to_owned(),
                            scopes: vec![RiskScope::Read],
                            ttl_ms: 60_000,
                            expires_in_ms: 60_000,
                        },
                    },
                );
                if !cancelled {
                    respond(
                        DaemonRequest::EventsWatch {
                            lease_id: server_lease.clone(),
                            principal: "cli".to_owned(),
                            after: None,
                        },
                        DaemonResponse::Events {
                            events: Vec::new(),
                            next_cursor: 7,
                            gap: false,
                        },
                    );
                }
                respond(
                    DaemonRequest::LeaseRelease {
                        lease_id: server_lease.clone(),
                        principal: "cli".to_owned(),
                    },
                    DaemonResponse::LeaseReleased {
                        lease_id: server_lease,
                    },
                );
            });

            let result = stream_events(
                &profile,
                lease_id,
                "cli".to_owned(),
                None,
                |_| Err(io::Error::from(io::ErrorKind::BrokenPipe)),
                || cancelled,
            );
            if cancelled {
                assert_eq!(result.unwrap_err().code, ClientErrorCode::Cancelled);
            } else {
                assert_eq!(result.unwrap(), ExitCode::from(EXIT_CANCELLED));
            }
            server.join().unwrap();
            fs::remove_file(path).unwrap();
        }
    }
}
