//! Тонкий local client единственного daemon protocol.

use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Duration;

use telegram_protocol::{DaemonRequest, DaemonResponse, LeaseId, RiskScope};

const DEFAULT_TTL_MS: u64 = 60_000;
const IO_TIMEOUT: Duration = Duration::from_secs(5);

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("telegram-cli: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<String>) -> Result<(), Box<dyn Error>> {
    let profile = env::var("TELEGRAM_PROFILE").unwrap_or_else(|_| "default".to_owned());
    let principal = env::var("TELEGRAM_PRINCIPAL").unwrap_or_else(|_| "telegram-cli".to_owned());
    let request = command(&arguments, principal)?;
    let response = exchange(&profile, &request)?;
    serde_json::to_writer(io::stdout().lock(), &response)?;
    println!();
    if matches!(response, DaemonResponse::Error { .. }) {
        Err("daemon rejected request".into())
    } else {
        Ok(())
    }
}

fn command(arguments: &[String], principal: String) -> Result<DaemonRequest, Box<dyn Error>> {
    match arguments {
        [session, status] if session == "session" && status == "status" => {
            Ok(DaemonRequest::SessionStatus)
        }
        [status] if status == "status" => Ok(DaemonRequest::SessionStatus),
        [session, hold] if session == "session" && hold == "hold" => {
            Ok(acquire(principal, "read", DEFAULT_TTL_MS)?)
        }
        [session, hold, scopes] if session == "session" && hold == "hold" => {
            Ok(acquire(principal, scopes, DEFAULT_TTL_MS)?)
        }
        [session, hold, scopes, ttl] if session == "session" && hold == "hold" => {
            Ok(acquire(principal, scopes, ttl.parse()?)?)
        }
        [session, release, lease_id] if session == "session" && release == "release" => {
            Ok(DaemonRequest::LeaseRelease {
                lease_id: LeaseId::new(lease_id.clone()),
                principal,
            })
        }
        _ => Err("usage: telegram-cli session status | session hold [scopes] [ttl_ms] | session release <lease_id>".into()),
    }
}

fn acquire(principal: String, scopes: &str, ttl_ms: u64) -> Result<DaemonRequest, Box<dyn Error>> {
    let scopes = scopes
        .split(',')
        .map(RiskScope::from_str)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DaemonRequest::LeaseAcquire {
        principal,
        scopes,
        ttl_ms,
    })
}

fn exchange(profile: &str, request: &DaemonRequest) -> Result<DaemonResponse, Box<dyn Error>> {
    let path = socket_path(profile)?;
    validate_socket(&path)?;
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    serde_json::to_writer(&mut stream, request)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response)?;
    Ok(serde_json::from_str(&response)?)
}

fn socket_path(profile: &str) -> Result<PathBuf, Box<dyn Error>> {
    if profile.is_empty()
        || profile.len() > 48
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err("profile name is invalid".into());
    }
    // SAFETY: geteuid has no preconditions and does not access memory.
    let uid = unsafe { libc::geteuid() };
    Ok(PathBuf::from(format!(
        "/tmp/telegramd-{uid}/{profile}.sock"
    )))
}

fn validate_socket(path: &std::path::Path) -> Result<(), Box<dyn Error>> {
    let directory = fs::symlink_metadata(path.parent().ok_or("socket path has no parent")?)?;
    let socket = fs::symlink_metadata(path)?;
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
        return Err("daemon socket is unsafe".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::{DirBuilder, Permissions};
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn session_commands_build_closed_protocol_requests() {
        assert_eq!(
            command(&["status".to_owned()], "cli".to_owned()).unwrap(),
            DaemonRequest::SessionStatus
        );
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
        assert!(
            command(
                &[
                    "session".to_owned(),
                    "hold".to_owned(),
                    "unknown".to_owned(),
                ],
                "cli".to_owned(),
            )
            .is_err()
        );
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
}
