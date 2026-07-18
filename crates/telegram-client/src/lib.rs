//! Общий security-checked Unix socket client для локальных daemon adapters.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use telegram_protocol::{ClientErrorCode, DaemonRequest, DaemonResponse};
use zeroize::{Zeroize, Zeroizing};

const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(35);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseFraming {
    Line,
    UntilEof,
    BoundedLine { max_bytes: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExchangeOptions {
    io_timeout: Duration,
    response_framing: ResponseFraming,
    connect_error: ClientErrorCode,
}

impl ExchangeOptions {
    pub const fn new(io_timeout: Duration, response_framing: ResponseFraming) -> Self {
        Self {
            io_timeout,
            response_framing,
            connect_error: ClientErrorCode::TransportFailed,
        }
    }

    pub const fn with_connect_error(mut self, connect_error: ClientErrorCode) -> Self {
        self.connect_error = connect_error;
        self
    }
}

pub fn exchange(profile: &str, request: &DaemonRequest) -> Result<DaemonResponse, ClientErrorCode> {
    exchange_with_options(
        profile,
        request,
        ExchangeOptions::new(DEFAULT_IO_TIMEOUT, ResponseFraming::Line),
    )
}

pub fn exchange_with_options(
    profile: &str,
    request: &DaemonRequest,
    options: ExchangeOptions,
) -> Result<DaemonResponse, ClientErrorCode> {
    let path = socket_path(profile)?;
    validate_socket(&path)?;
    let mut stream = UnixStream::connect(path).map_err(|_| options.connect_error)?;
    stream
        .set_read_timeout(Some(options.io_timeout))
        .and_then(|_| stream.set_write_timeout(Some(options.io_timeout)))
        .map_err(|_| ClientErrorCode::TransportFailed)?;
    serde_json::to_writer(&mut stream, request).map_err(|_| ClientErrorCode::TransportFailed)?;
    stream
        .write_all(b"\n")
        .and_then(|_| stream.flush())
        .map_err(|_| ClientErrorCode::TransportFailed)?;

    read_response(stream, options.response_framing)
}

pub fn socket_path(profile: &str) -> Result<PathBuf, ClientErrorCode> {
    if !valid_name(profile) {
        return Err(ClientErrorCode::InvalidProfile);
    }
    Ok(PathBuf::from(format!(
        "/tmp/telegramd-{}/{profile}.sock",
        effective_uid()
    )))
}

pub fn validate_socket(path: &Path) -> Result<(), ClientErrorCode> {
    let parent = path.parent().ok_or(ClientErrorCode::InvalidProfile)?;
    let directory = fs::symlink_metadata(parent).map_err(|_| ClientErrorCode::SocketUnavailable)?;
    let socket = fs::symlink_metadata(path).map_err(|_| ClientErrorCode::SocketUnavailable)?;
    let uid = effective_uid();
    if !directory.is_dir()
        || directory.uid() != uid
        || directory.mode() & 0o777 != 0o700
        || !socket.file_type().is_socket()
        || socket.uid() != uid
        || socket.nlink() != 1
        || socket.mode() & 0o777 != 0o600
    {
        return Err(ClientErrorCode::UnsafeSocket);
    }
    Ok(())
}

pub fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub fn effective_uid() -> u32 {
    // SAFETY: geteuid has no preconditions and does not access memory.
    unsafe { libc::geteuid() }
}

fn read_response(
    stream: UnixStream,
    framing: ResponseFraming,
) -> Result<DaemonResponse, ClientErrorCode> {
    match framing {
        ResponseFraming::Line => {
            let mut response = String::new();
            BufReader::new(stream)
                .read_line(&mut response)
                .map_err(|_| ClientErrorCode::TransportFailed)?;
            let parsed = serde_json::from_str(&response);
            response.zeroize();
            parsed.map_err(|_| ClientErrorCode::InvalidResponse)
        }
        ResponseFraming::UntilEof => serde_json::from_reader(BufReader::new(stream))
            .map_err(|_| ClientErrorCode::InvalidResponse),
        ResponseFraming::BoundedLine { max_bytes } => {
            let mut response = Zeroizing::new(Vec::new());
            BufReader::new(stream)
                .take(max_bytes.saturating_add(1))
                .read_until(b'\n', &mut response)
                .map_err(|_| ClientErrorCode::TransportFailed)?;
            if response.is_empty()
                || response.len() as u64 > max_bytes
                || !response.ends_with(b"\n")
            {
                return Err(ClientErrorCode::InvalidResponse);
            }
            serde_json::from_slice(&response).map_err(|_| ClientErrorCode::InvalidResponse)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, DirBuilder, Permissions};
    use std::io::{self, BufRead, BufReader, Write};
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use telegram_protocol::OperationalMetrics;

    use super::*;

    #[test]
    fn validation_requires_private_directory_and_socket_metadata() {
        let directory = TestDirectory(unique_directory("validation"));
        DirBuilder::new().mode(0o700).create(&directory.0).unwrap();
        let path = directory.0.join("daemon.sock");
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();
        assert_eq!(validate_socket(&path), Ok(()));

        fs::set_permissions(&path, Permissions::from_mode(0o640)).unwrap();
        assert_eq!(validate_socket(&path), Err(ClientErrorCode::UnsafeSocket));
        fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&directory.0, Permissions::from_mode(0o750)).unwrap();
        assert_eq!(validate_socket(&path), Err(ClientErrorCode::UnsafeSocket));

        drop(listener);
    }

    #[test]
    fn exchange_uses_private_jsonl_profile_socket() {
        let (profile, path, listener) = bind_profile_socket("client");
        let expected = DaemonResponse::SessionStatus {
            metrics: Box::new(OperationalMetrics {
                active_leases: 2,
                ..Default::default()
            }),
        };
        let server_response = expected.clone();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(&mut stream).read_line(&mut request).unwrap();
            assert_eq!(
                serde_json::from_str::<DaemonRequest>(&request).unwrap(),
                DaemonRequest::SessionStatus
            );
            serde_json::to_writer(&mut stream, &server_response).unwrap();
            stream.write_all(b"\n").unwrap();
        });

        assert_eq!(
            exchange(&profile, &DaemonRequest::SessionStatus).unwrap(),
            expected
        );
        server.join().unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn response_framing_preserves_eof_and_bounded_line_contracts() {
        let response = serde_json::to_vec(&DaemonResponse::SessionStatus {
            metrics: Box::default(),
        })
        .unwrap();
        let (profile, path, listener) = bind_profile_socket("eof");
        let server = respond_once(listener, response.clone());
        assert!(
            exchange_with_options(
                &profile,
                &DaemonRequest::SessionStatus,
                ExchangeOptions::new(Duration::from_secs(1), ResponseFraming::UntilEof),
            )
            .is_ok()
        );
        server.join().unwrap();
        fs::remove_file(path).unwrap();

        let (profile, path, listener) = bind_profile_socket("bounded");
        let server = respond_once(listener, response);
        assert_eq!(
            exchange_with_options(
                &profile,
                &DaemonRequest::SessionStatus,
                ExchangeOptions::new(
                    Duration::from_secs(1),
                    ResponseFraming::BoundedLine {
                        max_bytes: 16 * 1024
                    },
                ),
            ),
            Err(ClientErrorCode::InvalidResponse)
        );
        server.join().unwrap();
        fs::remove_file(path).unwrap();
    }

    fn bind_profile_socket(label: &str) -> (String, PathBuf, UnixListener) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let profile = format!("{label}-{}-{nonce:x}", std::process::id());
        let path = socket_path(&profile).unwrap();
        let directory = path.parent().unwrap();
        match DirBuilder::new().mode(0o700).create(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("can't create test socket directory: {error}"),
        }
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o600)).unwrap();
        (profile, path, listener)
    }

    fn respond_once(listener: UnixListener, response: Vec<u8>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(&mut stream).read_line(&mut request).unwrap();
            stream.write_all(&response).unwrap();
        })
    }

    fn unique_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        PathBuf::from("/tmp").join(format!(
            "telegram-client-{label}-{}-{nonce:x}",
            std::process::id()
        ))
    }

    struct TestDirectory(PathBuf);

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0.join("daemon.sock"));
            let _ = fs::remove_dir(&self.0);
        }
    }
}
