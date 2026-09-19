//! Bounded nonblocking framing: a stalled local client must not stall TDLib updates.

use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::{Duration, Instant};
use telegram_protocol::{CommandErrorCode, DaemonRequest, DaemonResponse, LeaseErrorCode};
use zeroize::{Zeroize, Zeroizing};

const MAX_CLIENTS: usize = 16;
const MAX_REQUEST: usize = 16 * 1024;
const MAX_RESPONSE: usize = 16 * 1024 * 1024;
const MAX_BUFFERED: usize = 32 * 1024 * 1024;
const IO_DEADLINE: Duration = Duration::from_secs(5);

struct Client {
    stream: UnixStream,
    bytes: Zeroizing<Vec<u8>>,
    written: Option<usize>,
    deadline: Instant,
}

#[derive(Default)]
pub struct Inbox {
    clients: Vec<Client>,
}

impl Client {
    fn poll_io(&mut self) -> bool {
        if Instant::now() >= self.deadline {
            return false;
        }
        if let Some(written) = self.written {
            return match self.stream.write(&self.bytes[written..]) {
                Ok(0) => false,
                Ok(count) => {
                    self.written = Some(written + count);
                    written + count < self.bytes.len()
                }
                Err(error) => error.kind() == io::ErrorKind::WouldBlock,
            };
        }
        if self.bytes.contains(&b'\n') {
            return true;
        }
        let mut chunk = [0_u8; 4096];
        let keep = match self.stream.read(&mut chunk) {
            Ok(0) if self.bytes.is_empty() => false,
            Ok(0) => {
                self.bytes.zeroize();
                self.bytes.extend_from_slice(b"!\n");
                true
            }
            Ok(count) => {
                self.bytes.extend_from_slice(&chunk[..count]);
                if self.bytes.len() > MAX_REQUEST {
                    self.bytes.zeroize();
                    self.bytes.extend_from_slice(b"!\n");
                }
                true
            }
            Err(error) => error.kind() == io::ErrorKind::WouldBlock,
        };
        chunk.zeroize();
        keep
    }

    fn respond(&mut self, response: &DaemonResponse, limit: usize) {
        let mut output = LimitedOutput {
            bytes: Zeroizing::new(Vec::new()),
            limit,
        };
        if serde_json::to_writer(&mut output, response).is_err() || output.write_all(b"\n").is_err()
        {
            // The operation may already have run. Return a receipt-loss error, never a blind EOF.
            output.bytes.zeroize();
            let error = DaemonResponse::CommandError {
                code: CommandErrorCode::ResponseTooLarge,
            };
            serde_json::to_writer(&mut *output.bytes, &error).expect("small error is serializable");
            output.bytes.push(b'\n');
        }
        self.bytes = output.bytes;
        self.written = Some(0);
        self.deadline = Instant::now() + IO_DEADLINE;
    }
}

impl Inbox {
    pub fn poll(
        &mut self,
        listener: &UnixListener,
        mut handle: impl FnMut(DaemonRequest) -> DaemonResponse,
    ) -> io::Result<()> {
        for _ in self.clients.len()..MAX_CLIENTS {
            match listener.accept() {
                Ok((stream, _)) => {
                    if stream.set_nonblocking(true).is_err() {
                        continue;
                    }
                    self.clients.push(Client {
                        stream,
                        bytes: Zeroizing::new(Vec::new()),
                        written: None,
                        deadline: Instant::now() + IO_DEADLINE,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        self.clients.retain_mut(Client::poll_io);
        let Some(index) = self
            .clients
            .iter()
            .position(|client| client.written.is_none() && client.bytes.contains(&b'\n'))
        else {
            return Ok(());
        };
        let client = &self.clients[index];
        let end = client
            .bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .expect("ready frame");
        let request = serde_json::from_slice(&client.bytes[..end]);
        let started = Instant::now();
        // One workflow per tick lets the lifecycle loop consume TDLib updates.
        let response = match request {
            Ok(request) => handle(request),
            Err(_) => DaemonResponse::Error {
                code: LeaseErrorCode::InvalidRequest,
            },
        };
        // I/O deadlines measure client stalls, excluding time spent inside another workflow.
        let blocked = started.elapsed();
        for client in &mut self.clients {
            client.deadline += blocked;
        }
        let buffered: usize = self.clients.iter().map(|client| client.bytes.len()).sum();
        let limit = MAX_RESPONSE.min(MAX_BUFFERED.saturating_sub(buffered));
        self.clients[index].respond(&response, limit);
        Ok(())
    }
}

struct LimitedOutput {
    bytes: Zeroizing<Vec<u8>>,
    limit: usize,
}
impl Write for LimitedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("response capacity exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workflow_time_does_not_consume_another_clients_io_deadline() {
        let directory =
            std::path::PathBuf::from("/tmp").join(format!("ipc-deadline-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("socket");
        let listener = UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut peers = Vec::new();
        for _ in 0..2 {
            let mut peer = UnixStream::connect(&path).unwrap();
            peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
            serde_json::to_writer(&mut peer, &DaemonRequest::SessionStatus).unwrap();
            peer.write_all(b"\n").unwrap();
            peers.push(peer);
        }
        let mut inbox = Inbox::default();
        inbox
            .poll(&listener, |_| {
                std::thread::sleep(IO_DEADLINE + Duration::from_millis(20));
                DaemonResponse::SessionStatus {
                    metrics: Box::default(),
                }
            })
            .unwrap();
        let mut called = false;
        inbox
            .poll(&listener, |_| {
                called = true;
                DaemonResponse::SessionStatus {
                    metrics: Box::default(),
                }
            })
            .unwrap();
        assert!(
            called,
            "queued request was incorrectly expired by another workflow"
        );
        inbox.poll(&listener, |_| unreachable!()).unwrap();
        for mut peer in peers {
            let mut bytes = Vec::new();
            peer.read_to_end(&mut bytes).unwrap();
            assert!(serde_json::from_slice::<DaemonResponse>(&bytes).is_ok());
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn oversized_receipt_returns_a_typed_error_after_dispatch() {
        let (stream, _peer) = UnixStream::pair().unwrap();
        let mut client = Client {
            stream,
            bytes: Zeroizing::new(Vec::new()),
            written: None,
            deadline: Instant::now(),
        };
        client.respond(
            &DaemonResponse::WorkflowResult {
                workflow: "test".into(),
                result: serde_json::json!({"body":"large"}),
                complete: true,
            },
            1,
        );
        assert_eq!(
            serde_json::from_slice::<DaemonResponse>(&client.bytes).unwrap(),
            DaemonResponse::CommandError {
                code: CommandErrorCode::ResponseTooLarge
            }
        );
    }

    #[test]
    fn incomplete_client_does_not_block_a_complete_request() {
        let path = std::path::PathBuf::from("/tmp").join(format!(
            "tg-ipc-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let listener = UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut slow = UnixStream::connect(&path).unwrap();
        slow.write_all(b"{").unwrap();
        let mut ready = UnixStream::connect(&path).unwrap();
        ready
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        serde_json::to_writer(&mut ready, &DaemonRequest::SessionStatus).unwrap();
        ready.write_all(b"\n").unwrap();
        let mut inbox = Inbox::default();
        let mut calls = 0;
        inbox
            .poll(&listener, |_| {
                calls += 1;
                DaemonResponse::SessionStatus {
                    metrics: Box::default(),
                }
            })
            .unwrap();
        assert_eq!(calls, 1);
        inbox.poll(&listener, |_| unreachable!()).unwrap();
        let mut output = String::new();
        ready.read_to_string(&mut output).unwrap();
        assert!(serde_json::from_str::<DaemonResponse>(&output).is_ok());
        std::fs::remove_file(path).unwrap();
    }
}
