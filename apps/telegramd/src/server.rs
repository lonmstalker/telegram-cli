//! Bounded JSONL lease protocol поверх private profile socket.

use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::{Duration, Instant};

use telegram_protocol::{LeaseErrorCode, LeaseRequest, LeaseResponse};

use crate::lease::LeaseManager;

const MAX_REQUEST_BYTES: u64 = 16 * 1024;
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(5);

pub struct LeaseServer {
    leases: LeaseManager,
}

impl LeaseServer {
    pub fn new(leases: LeaseManager) -> Self {
        Self { leases }
    }

    pub fn poll(&mut self, listener: &UnixListener, now: Instant) -> Result<(), ServerError> {
        self.leases.expire(now);
        loop {
            match self.serve_once(listener) {
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

    fn serve_once(&mut self, listener: &UnixListener) -> Result<(), ServerError> {
        let (stream, _) = listener
            .accept()
            .map_err(|error| ServerError::Accept(error.kind()))?;
        stream
            .set_nonblocking(false)
            .map_err(|error| ServerError::ClientIo(error.kind()))?;
        self.serve_connection(stream)
    }

    fn serve_connection(&mut self, mut stream: UnixStream) -> Result<(), ServerError> {
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
            LeaseResponse::Error {
                code: LeaseErrorCode::InvalidRequest,
            }
        } else {
            bytes.pop();
            if bytes.ends_with(b"\r") {
                bytes.pop();
            }
            match serde_json::from_slice(&bytes) {
                Ok(request) => self.handle(request, Instant::now()),
                Err(_) => LeaseResponse::Error {
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

    fn handle(&mut self, request: LeaseRequest, now: Instant) -> LeaseResponse {
        match request {
            LeaseRequest::LeaseAcquire {
                principal,
                scopes,
                ttl_ms,
            } => match self.leases.acquire(principal, scopes, ttl_ms, now) {
                Ok(lease) => LeaseResponse::LeaseGranted { lease },
                Err(code) => LeaseResponse::Error { code },
            },
            LeaseRequest::LeaseHeartbeat {
                lease_id,
                principal,
            } => match self.leases.heartbeat(&lease_id, &principal, now) {
                Ok(lease) => LeaseResponse::LeaseRenewed { lease },
                Err(code) => LeaseResponse::Error { code },
            },
            LeaseRequest::LeaseRelease {
                lease_id,
                principal,
            } => match self.leases.release(&lease_id, &principal, now) {
                Ok(()) => LeaseResponse::LeaseReleased { lease_id },
                Err(code) => LeaseResponse::Error { code },
            },
        }
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
            LeaseRequest::LeaseAcquire {
                principal: "agent".to_owned(),
                scopes: vec![RiskScope::Read],
                ttl_ms: 1_000,
            },
        );
        let LeaseResponse::LeaseGranted {
            lease: LeaseView { lease_id, .. },
        } = granted
        else {
            panic!("expected granted lease")
        };
        assert!(matches!(
            exchange(
                &mut server,
                &socket,
                LeaseRequest::LeaseHeartbeat {
                    lease_id: lease_id.clone(),
                    principal: "agent".to_owned(),
                }
            ),
            LeaseResponse::LeaseRenewed { .. }
        ));
        assert_eq!(
            exchange(
                &mut server,
                &socket,
                LeaseRequest::LeaseRelease {
                    lease_id: lease_id.clone(),
                    principal: "agent".to_owned(),
                }
            ),
            LeaseResponse::LeaseReleased { lease_id }
        );
        assert_eq!(server.leases.active_count(), 0);

        drop(socket);
        drop(ownership);
        fs::remove_dir_all(root).unwrap();
    }

    fn exchange(
        server: &mut LeaseServer,
        socket: &DaemonSocket,
        request: LeaseRequest,
    ) -> LeaseResponse {
        let mut client = UnixStream::connect(socket.path()).unwrap();
        serde_json::to_writer(&mut client, &request).unwrap();
        client.write_all(b"\n").unwrap();
        server.serve_once(socket.listener()).unwrap();
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
