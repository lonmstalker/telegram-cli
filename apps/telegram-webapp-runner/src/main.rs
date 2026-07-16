//! Local one-shot browser adapter boundary for Mini App launch artifacts.

use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use telegram_protocol::{
    BrowserEvidence, DaemonRequest, DaemonResponse, ProtectedString, WebAppBrowserReport,
};
use zeroize::Zeroizing;

const IO_TIMEOUT: Duration = Duration::from_secs(5);
const ADAPTER_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RESPONSE_BYTES: u64 = 16 * 1024;
const MAX_ADAPTER_OUTPUT_BYTES: u64 = 16 * 1024;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(report) => {
            let written = (|| {
                let mut stdout = io::stdout().lock();
                serde_json::to_writer(&mut stdout, &report).map_err(io::Error::other)?;
                stdout.write_all(b"\n")
            })();
            if written.is_err() {
                return ExitCode::from(5);
            }
            if report.browser.passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(4)
            }
        }
        Err(error) => {
            eprintln!("telegram-webapp-runner: {}", error.message());
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(arguments: Vec<String>) -> Result<WebAppBrowserReport, RunnerError> {
    let [handle, separator, adapter @ ..] = arguments.as_slice() else {
        return Err(RunnerError::Usage);
    };
    if separator != "--" || adapter.is_empty() || !valid_handle(handle) {
        return Err(RunnerError::Usage);
    }
    let profile = env::var("TELEGRAM_PROFILE").unwrap_or_else(|_| "default".to_owned());
    let principal = env::var("TELEGRAM_PRINCIPAL").unwrap_or_else(|_| "telegram-cli".to_owned());
    let artifact = take_artifact(&profile, &principal, handle)?;
    run_adapter(artifact, adapter)
}

fn take_artifact(
    profile: &str,
    principal: &str,
    handle: &str,
) -> Result<BrowserLaunch, RunnerError> {
    let path = socket_path(profile)?;
    validate_socket(&path)?;
    let mut stream = UnixStream::connect(path).map_err(|_| RunnerError::SocketUnavailable)?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|_| stream.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|_| RunnerError::Transport)?;
    let request = DaemonRequest::WebAppArtifactTake {
        handle: handle.to_owned(),
        principal: principal.to_owned(),
    };
    let mut wire = Zeroizing::new(Vec::new());
    serde_json::to_writer(&mut *wire, &request).map_err(|_| RunnerError::Transport)?;
    wire.push(b'\n');
    stream
        .write_all(&wire)
        .and_then(|_| stream.flush())
        .map_err(|_| RunnerError::Transport)?;

    let mut response = Zeroizing::new(Vec::new());
    BufReader::new(stream)
        .take(MAX_RESPONSE_BYTES + 1)
        .read_until(b'\n', &mut response)
        .map_err(|_| RunnerError::Transport)?;
    if response.is_empty()
        || response.len() as u64 > MAX_RESPONSE_BYTES
        || !response.ends_with(b"\n")
    {
        return Err(RunnerError::InvalidResponse);
    }
    let response: DaemonResponse =
        serde_json::from_slice(&response).map_err(|_| RunnerError::InvalidResponse)?;
    match response {
        DaemonResponse::WebAppArtifact {
            launch_id,
            url,
            require_same_origin,
        } => Ok(BrowserLaunch {
            launch_id,
            url,
            require_same_origin,
        }),
        _ => Err(RunnerError::ArtifactUnavailable),
    }
}

fn run_adapter(
    artifact: BrowserLaunch,
    adapter: &[String],
) -> Result<WebAppBrowserReport, RunnerError> {
    let mut child = Command::new(&adapter[0])
        .args(&adapter[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| RunnerError::AdapterUnavailable)?;
    let mut payload = Zeroizing::new(Vec::new());
    serde_json::to_writer(&mut *payload, &artifact).map_err(|_| RunnerError::AdapterFailed)?;
    payload.push(b'\n');
    child
        .stdin
        .take()
        .ok_or(RunnerError::AdapterFailed)?
        .write_all(&payload)
        .map_err(|_| RunnerError::AdapterFailed)?;

    let stdout = child.stdout.take().ok_or(RunnerError::AdapterFailed)?;
    let reader = thread::spawn(move || {
        let mut output = Zeroizing::new(Vec::new());
        stdout
            .take(MAX_ADAPTER_OUTPUT_BYTES + 1)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let deadline = Instant::now() + ADAPTER_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| RunnerError::AdapterFailed)? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(RunnerError::AdapterTimedOut);
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = reader
        .join()
        .map_err(|_| RunnerError::AdapterFailed)?
        .map_err(|_| RunnerError::AdapterFailed)?;
    if !status.success() || output.len() as u64 > MAX_ADAPTER_OUTPUT_BYTES {
        return Err(RunnerError::AdapterFailed);
    }
    let browser: BrowserEvidence =
        serde_json::from_slice(&output).map_err(|_| RunnerError::AdapterFailed)?;
    if browser.dom_assertions == 0
        && browser.bridge_assertions == 0
        && browser.network_assertions == 0
    {
        return Err(RunnerError::AdapterFailed);
    }
    Ok(WebAppBrowserReport {
        launch_id: artifact.launch_id,
        telegram_prepared: true,
        browser,
        artifact_consumed: true,
    })
}

#[derive(Serialize)]
struct BrowserLaunch {
    launch_id: i64,
    url: ProtectedString,
    require_same_origin: bool,
}

fn valid_handle(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn socket_path(profile: &str) -> Result<PathBuf, RunnerError> {
    if profile.is_empty()
        || profile.len() > 48
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(RunnerError::InvalidProfile);
    }
    // SAFETY: geteuid has no preconditions and does not access memory.
    let uid = unsafe { libc::geteuid() };
    Ok(PathBuf::from(format!(
        "/tmp/telegramd-{uid}/{profile}.sock"
    )))
}

fn validate_socket(path: &PathBuf) -> Result<(), RunnerError> {
    let parent = path.parent().ok_or(RunnerError::UnsafeSocket)?;
    let directory = fs::symlink_metadata(parent).map_err(|_| RunnerError::SocketUnavailable)?;
    let socket = fs::symlink_metadata(path).map_err(|_| RunnerError::SocketUnavailable)?;
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
        return Err(RunnerError::UnsafeSocket);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunnerError {
    Usage,
    InvalidProfile,
    SocketUnavailable,
    UnsafeSocket,
    Transport,
    InvalidResponse,
    ArtifactUnavailable,
    AdapterUnavailable,
    AdapterTimedOut,
    AdapterFailed,
}

impl RunnerError {
    const fn exit_code(self) -> u8 {
        match self {
            Self::Usage | Self::InvalidProfile => 2,
            Self::SocketUnavailable
            | Self::UnsafeSocket
            | Self::Transport
            | Self::ArtifactUnavailable
            | Self::AdapterUnavailable
            | Self::AdapterTimedOut => 3,
            Self::InvalidResponse | Self::AdapterFailed => 5,
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::Usage => "usage: telegram-webapp-runner <artifact_handle> -- <adapter> [args...]",
            Self::InvalidProfile => "неверное имя profile",
            Self::SocketUnavailable => "daemon socket недоступен",
            Self::UnsafeSocket => "daemon socket не прошёл проверку безопасности",
            Self::Transport => "обмен с daemon не выполнен",
            Self::InvalidResponse => "daemon вернул неверный protocol response",
            Self::ArtifactUnavailable => "launch artifact недоступен или уже использован",
            Self::AdapterUnavailable => "browser adapter не запущен",
            Self::AdapterTimedOut => "browser adapter превысил deadline",
            Self::AdapterFailed => "browser adapter вернул неверное evidence",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_receives_secret_only_on_stdin_and_returns_closed_evidence() {
        let canary = "TG_WEBAPP_INIT_DATA_CANARY";
        let artifact = BrowserLaunch {
            launch_id: 11,
            url: ProtectedString::new(canary.to_owned()),
            require_same_origin: true,
        };
        let adapter = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "IFS= read -r launch; test -n \"$launch\" && printf '%s' '{\"passed\":false,\"dom_assertions\":2,\"bridge_assertions\":1,\"network_assertions\":1,\"js_errors\":1}'".to_owned(),
        ];
        assert!(adapter.iter().all(|argument| !argument.contains(canary)));
        let report = run_adapter(artifact, &adapter).unwrap();
        assert!(!report.browser.passed);
        assert_eq!(report.browser.dom_assertions, 2);
        assert!(report.telegram_prepared && report.artifact_consumed);
        assert!(!serde_json::to_string(&report).unwrap().contains(canary));
    }
}
