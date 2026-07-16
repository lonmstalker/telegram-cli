//! Durable operation fingerprints и fail-closed reconciliation state.

use std::collections::HashMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::registry::ValidatedRequest;

const VERSION: u64 = 1;
const MAX_LINE_BYTES: usize = 1024;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct OperationFingerprint([u8; 32]);

impl OperationFingerprint {
    pub(crate) fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    pub fn for_request(request: &ValidatedRequest) -> Self {
        let canonical = serde_json::to_vec(request.as_value())
            .expect("serde_json::Value serialization is infallible");
        Self(hash(b"telegram-cli-operation-v1", &canonical))
    }

    pub fn for_workflow(name: &str, input: &Value) -> Self {
        let canonical = serde_json::to_vec(&(name, input))
            .expect("workflow name and JSON input are serializable");
        Self(hash(b"telegram-cli-workflow-v1", &canonical))
    }

    fn from_hex(value: &str) -> Option<Self> {
        if value.len() != 64 {
            return None;
        }
        let mut bytes = [0; 32];
        for (target, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
            *target = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
        }
        Some(Self(bytes))
    }

    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        let mut encoded = String::with_capacity(64);
        for byte in self.0 {
            use fmt::Write as _;
            write!(encoded, "{byte:02x}").expect("writing to String is infallible");
        }
        encoded
    }
}

impl fmt::Debug for OperationFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("OperationFingerprint")
            .field(&self.to_hex())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Pending,
    Succeeded,
    Failed,
    Uncertain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeginDecision {
    Dispatch,
    AlreadySucceeded,
    ReconcileRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationOutcome {
    Applied,
    NotApplied,
    Unknown,
}

pub struct IdempotencyJournal {
    file: File,
    states: HashMap<OperationFingerprint, OperationState>,
}

impl IdempotencyJournal {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(JournalError::PathNotAbsolute);
        }
        let (file, created) = open_file(path)?;
        validate_file(&file)?;
        if created {
            file.sync_all().map_err(io_error)?;
            File::open(path.parent().ok_or(JournalError::PathNotAbsolute)?)
                .and_then(|directory| directory.sync_all())
                .map_err(io_error)?;
        }

        let (states, valid_bytes) = load(&file)?;
        if valid_bytes < file.metadata().map_err(io_error)?.len() {
            file.set_len(valid_bytes).map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
        }
        let mut journal = Self { file, states };
        let interrupted = journal
            .states
            .iter()
            .filter_map(|(fingerprint, state)| {
                (*state == OperationState::Pending).then_some(*fingerprint)
            })
            .collect::<Vec<_>>();
        for fingerprint in interrupted {
            journal.append(fingerprint, OperationState::Uncertain, None)?;
        }
        Ok(journal)
    }

    pub fn state(&self, fingerprint: OperationFingerprint) -> Option<OperationState> {
        self.states.get(&fingerprint).copied()
    }

    pub fn begin(
        &mut self,
        fingerprint: OperationFingerprint,
    ) -> Result<BeginDecision, JournalError> {
        match self.state(fingerprint) {
            Some(OperationState::Succeeded) => Ok(BeginDecision::AlreadySucceeded),
            Some(OperationState::Pending | OperationState::Uncertain) => {
                Ok(BeginDecision::ReconcileRequired)
            }
            None | Some(OperationState::Failed) => {
                self.append(fingerprint, OperationState::Pending, None)?;
                Ok(BeginDecision::Dispatch)
            }
        }
    }

    pub fn succeeded(&mut self, fingerprint: OperationFingerprint) -> Result<(), JournalError> {
        self.finish(fingerprint, OperationState::Succeeded)
    }

    pub fn failed(&mut self, fingerprint: OperationFingerprint) -> Result<(), JournalError> {
        self.finish(fingerprint, OperationState::Failed)
    }

    pub fn uncertain(&mut self, fingerprint: OperationFingerprint) -> Result<(), JournalError> {
        self.finish(fingerprint, OperationState::Uncertain)
    }

    pub fn reconcile(
        &mut self,
        fingerprint: OperationFingerprint,
        outcome: ReconciliationOutcome,
        canonical_evidence: &[u8],
    ) -> Result<OperationState, JournalError> {
        if self.state(fingerprint) != Some(OperationState::Uncertain) {
            return Err(JournalError::InvalidTransition);
        }
        let state = match outcome {
            ReconciliationOutcome::Applied => OperationState::Succeeded,
            ReconciliationOutcome::NotApplied => OperationState::Failed,
            ReconciliationOutcome::Unknown => OperationState::Uncertain,
        };
        let evidence = encode(hash(
            b"telegram-cli-reconciliation-evidence-v1",
            canonical_evidence,
        ));
        self.append(fingerprint, state, Some(evidence))?;
        Ok(state)
    }

    fn finish(
        &mut self,
        fingerprint: OperationFingerprint,
        state: OperationState,
    ) -> Result<(), JournalError> {
        if self.state(fingerprint) != Some(OperationState::Pending) {
            return Err(JournalError::InvalidTransition);
        }
        self.append(fingerprint, state, None)
    }

    fn append(
        &mut self,
        fingerprint: OperationFingerprint,
        state: OperationState,
        evidence: Option<String>,
    ) -> Result<(), JournalError> {
        if !valid_transition(self.state(fingerprint), state, evidence.is_some()) {
            return Err(JournalError::InvalidTransition);
        }
        let record = Record {
            v: VERSION,
            fingerprint: fingerprint.to_hex(),
            state,
            evidence,
        };
        let mut line = serde_json::to_vec(&record).map_err(|_| JournalError::Corrupt)?;
        line.push(b'\n');
        if line.len() > MAX_LINE_BYTES {
            return Err(JournalError::Corrupt);
        }
        self.file.write_all(&line).map_err(io_error)?;
        self.file.sync_data().map_err(io_error)?;
        self.states.insert(fingerprint, state);
        Ok(())
    }
}

fn open_file(path: &Path) -> Result<(File, bool), JournalError> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .append(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    match options.open(path) {
        Ok(file) => Ok((file, true)),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => OpenOptions::new()
            .read(true)
            .append(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path)
            .map(|file| (file, false))
            .map_err(io_error),
        Err(error) => Err(io_error(error)),
    }
}

fn validate_file(file: &File) -> Result<(), JournalError> {
    let metadata = file.metadata().map_err(io_error)?;
    // SAFETY: geteuid has no preconditions and does not access memory.
    let current_uid = unsafe { libc::geteuid() };
    if !metadata.file_type().is_file()
        || metadata.nlink() != 1
        || metadata.uid() != current_uid
        || metadata.mode() & 0o777 != 0o600
    {
        return Err(JournalError::UnsafeFile);
    }
    Ok(())
}

fn load(file: &File) -> Result<(HashMap<OperationFingerprint, OperationState>, u64), JournalError> {
    let mut reader = BufReader::new(file.try_clone().map_err(io_error)?);
    let mut states = HashMap::new();
    let mut valid_bytes = 0;
    loop {
        let mut line = Vec::new();
        let read = reader.read_until(b'\n', &mut line).map_err(io_error)?;
        if read == 0 {
            break;
        }
        if line.len() > MAX_LINE_BYTES {
            return Err(JournalError::Corrupt);
        }
        if line.last() != Some(&b'\n') {
            break;
        }
        let record: Record = serde_json::from_slice(&line).map_err(|_| JournalError::Corrupt)?;
        if record.v != VERSION {
            return Err(JournalError::Corrupt);
        }
        let fingerprint =
            OperationFingerprint::from_hex(&record.fingerprint).ok_or(JournalError::Corrupt)?;
        let has_evidence = match record.evidence {
            None => false,
            Some(value) if OperationFingerprint::from_hex(&value).is_some() => true,
            Some(_) => return Err(JournalError::Corrupt),
        };
        if !valid_transition(
            states.get(&fingerprint).copied(),
            record.state,
            has_evidence,
        ) {
            return Err(JournalError::Corrupt);
        }
        states.insert(fingerprint, record.state);
        valid_bytes += read as u64;
    }
    Ok((states, valid_bytes))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    v: u64,
    fingerprint: String,
    state: OperationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    evidence: Option<String>,
}

fn valid_transition(
    previous: Option<OperationState>,
    state: OperationState,
    has_evidence: bool,
) -> bool {
    matches!(
        (previous, state, has_evidence),
        (
            None | Some(OperationState::Failed),
            OperationState::Pending,
            false
        ) | (
            Some(OperationState::Pending),
            OperationState::Succeeded | OperationState::Failed | OperationState::Uncertain,
            false
        ) | (
            Some(OperationState::Uncertain),
            OperationState::Succeeded | OperationState::Failed | OperationState::Uncertain,
            true
        )
    )
}

fn hash(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain.len().to_be_bytes());
    hasher.update(domain);
    hasher.update(payload.len().to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}

fn encode(bytes: [u8; 32]) -> String {
    OperationFingerprint(bytes).to_hex()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn io_error(error: io::Error) -> JournalError {
    JournalError::Io(error.kind())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalError {
    PathNotAbsolute,
    Io(io::ErrorKind),
    UnsafeFile,
    Corrupt,
    InvalidTransition,
}

impl fmt::Display for JournalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathNotAbsolute => formatter.write_str("journal path must be absolute"),
            Self::Io(kind) => write!(formatter, "idempotency journal I/O failed: {kind:?}"),
            Self::UnsafeFile => formatter.write_str("idempotency journal file is unsafe"),
            Self::Corrupt => formatter.write_str("idempotency journal is corrupt"),
            Self::InvalidTransition => {
                formatter.write_str("idempotency state transition is invalid")
            }
        }
    }
}

impl std::error::Error for JournalError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use super::*;

    #[test]
    fn fingerprint_is_stable_for_the_same_validated_request() {
        let first =
            ValidatedRequest::from_value(json!({"@type": "getChat", "chat_id": 42})).unwrap();
        let second =
            ValidatedRequest::from_value(json!({"chat_id": 42, "@type": "getChat"})).unwrap();
        let different =
            ValidatedRequest::from_value(json!({"@type": "getChat", "chat_id": 43})).unwrap();

        assert_eq!(
            OperationFingerprint::for_request(&first),
            OperationFingerprint::for_request(&second)
        );
        assert_ne!(
            OperationFingerprint::for_request(&first),
            OperationFingerprint::for_request(&different)
        );
    }

    #[test]
    fn interrupted_dispatch_requires_reconciliation_after_reopen() {
        let root = temporary_directory("recovery");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("journal.jsonl");
        let fingerprint = fingerprint(42);

        let mut journal = IdempotencyJournal::open(&path).unwrap();
        assert_eq!(journal.begin(fingerprint).unwrap(), BeginDecision::Dispatch);
        drop(journal);

        let mut journal = IdempotencyJournal::open(&path).unwrap();
        assert_eq!(journal.state(fingerprint), Some(OperationState::Uncertain));
        assert_eq!(
            journal.begin(fingerprint).unwrap(),
            BeginDecision::ReconcileRequired
        );
        assert_eq!(
            journal
                .reconcile(fingerprint, ReconciliationOutcome::Applied, b"message:sent")
                .unwrap(),
            OperationState::Succeeded
        );
        drop(journal);

        let mut journal = IdempotencyJournal::open(&path).unwrap();
        assert_eq!(
            journal.begin(fingerprint).unwrap(),
            BeginDecision::AlreadySucceeded
        );
        let persisted = fs::read_to_string(&path).unwrap();
        assert!(!persisted.contains("getChat"));
        assert!(!persisted.contains("message:sent"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retry_requires_terminal_or_reconciled_not_applied_proof() {
        let root = temporary_directory("retry");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("journal.jsonl");
        let fingerprint = fingerprint(7);
        let mut journal = IdempotencyJournal::open(&path).unwrap();

        assert_eq!(journal.begin(fingerprint).unwrap(), BeginDecision::Dispatch);
        journal.uncertain(fingerprint).unwrap();
        assert_eq!(
            journal.begin(fingerprint).unwrap(),
            BeginDecision::ReconcileRequired
        );
        journal
            .reconcile(
                fingerprint,
                ReconciliationOutcome::NotApplied,
                b"authoritative absence",
            )
            .unwrap();
        assert_eq!(journal.begin(fingerprint).unwrap(), BeginDecision::Dispatch);
        journal.failed(fingerprint).unwrap();
        assert_eq!(journal.begin(fingerprint).unwrap(), BeginDecision::Dispatch);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn torn_tail_falls_back_to_uncertain_but_complete_corruption_fails() {
        let root = temporary_directory("torn");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("journal.jsonl");
        let fingerprint = fingerprint(9);
        let mut journal = IdempotencyJournal::open(&path).unwrap();
        journal.begin(fingerprint).unwrap();
        drop(journal);
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(br#"{"v":1,"sequence":2"#)
            .unwrap();

        let journal = IdempotencyJournal::open(&path).unwrap();
        assert_eq!(journal.state(fingerprint), Some(OperationState::Uncertain));
        drop(journal);
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"{}\n")
            .unwrap();
        assert!(matches!(
            IdempotencyJournal::open(&path),
            Err(JournalError::Corrupt)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    fn fingerprint(chat_id: i64) -> OperationFingerprint {
        OperationFingerprint::for_request(
            &ValidatedRequest::from_value(json!({"@type": "getChat", "chat_id": chat_id})).unwrap(),
        )
    }

    fn temporary_directory(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "telegram-core-idempotency-{name}-{}-{nonce}",
            std::process::id()
        ))
    }
}
