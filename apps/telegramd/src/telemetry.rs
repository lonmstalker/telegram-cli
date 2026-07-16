//! Fixed-shape operational metrics и payload-free audit.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;
use telegram_core::registry::{self, CapabilityDisposition, RiskClass, ValidatedRequest};
use telegram_protocol::OperationalMetrics;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationOutcome {
    Succeeded,
    Failed,
    Uncertain,
    Denied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    Fresh,
    Cached,
    Stale,
    Partial,
}

#[derive(Clone, Default)]
pub struct Telemetry(Arc<Mutex<OperationalMetrics>>);

impl Telemetry {
    pub fn record_request(&self, latency: Duration, outcome: OperationOutcome) {
        let mut metrics = self.lock();
        let latency = milliseconds(latency);
        metrics.requests = metrics.requests.saturating_add(1);
        let counter = match outcome {
            OperationOutcome::Succeeded => &mut metrics.succeeded,
            OperationOutcome::Failed => &mut metrics.failed,
            OperationOutcome::Uncertain => &mut metrics.uncertain,
            OperationOutcome::Denied => &mut metrics.denied,
        };
        *counter = counter.saturating_add(1);
        metrics.request_latency_ms_total = metrics.request_latency_ms_total.saturating_add(latency);
        metrics.request_latency_ms_max = metrics.request_latency_ms_max.max(latency);
    }

    pub fn observe_queue(&self, depth: usize) {
        let mut metrics = self.lock();
        metrics.queue_depth = depth;
        metrics.queue_depth_max = metrics.queue_depth_max.max(depth);
    }

    pub fn record_queue_rejection(&self) {
        let mut metrics = self.lock();
        metrics.queue_rejections = metrics.queue_rejections.saturating_add(1);
    }

    pub fn record_retry(&self) {
        let mut metrics = self.lock();
        metrics.retries = metrics.retries.saturating_add(1);
    }

    pub fn record_flood(&self, delay: Duration) {
        let mut metrics = self.lock();
        metrics.flood_waits = metrics.flood_waits.saturating_add(1);
        metrics.flood_delay_ms_total = metrics
            .flood_delay_ms_total
            .saturating_add(milliseconds(delay));
    }

    pub fn observe_update_lag(&self, lag: Duration) {
        let mut metrics = self.lock();
        metrics.update_lag_events = metrics.update_lag_events.saturating_add(1);
        metrics.update_lag_ms_max = metrics.update_lag_ms_max.max(milliseconds(lag));
    }

    pub fn observe_freshness(&self, freshness: Freshness) {
        let mut metrics = self.lock();
        let counter = match freshness {
            Freshness::Fresh => &mut metrics.fresh_results,
            Freshness::Cached => &mut metrics.cached_results,
            Freshness::Stale => &mut metrics.stale_results,
            Freshness::Partial => &mut metrics.partial_results,
        };
        *counter = counter.saturating_add(1);
    }

    pub fn observe_leases(&self, active: usize) {
        let mut metrics = self.lock();
        metrics.active_leases = active;
        metrics.active_leases_max = metrics.active_leases_max.max(active);
    }

    pub fn snapshot(&self) -> OperationalMetrics {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, OperationalMetrics> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(Debug)]
pub struct AuditEvent {
    observed_at_unix_ms: u64,
    method: &'static str,
    risk: RiskClass,
    outcome: OperationOutcome,
    latency_ms: u64,
    queue_ms: u64,
    retries: u64,
    reconciled: bool,
}

impl AuditEvent {
    pub fn operation(
        request: &ValidatedRequest,
        outcome: OperationOutcome,
        latency: Duration,
        queued: Duration,
        retries: u64,
        reconciled: bool,
        observed_at: SystemTime,
    ) -> Result<Self, AuditError> {
        let capability =
            registry::capability(request.descriptor().name).ok_or(AuditError::MethodNotReviewed)?;
        let CapabilityDisposition::Reviewed { risk, .. } = capability.disposition else {
            return Err(AuditError::MethodNotReviewed);
        };
        let observed_at_unix_ms = observed_at
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AuditError::InvalidClock)?
            .as_millis()
            .try_into()
            .map_err(|_| AuditError::InvalidClock)?;
        Ok(Self {
            observed_at_unix_ms,
            method: request.descriptor().name,
            risk,
            outcome,
            latency_ms: milliseconds(latency),
            queue_ms: milliseconds(queued),
            retries,
            reconciled,
        })
    }

    fn json(&self) -> serde_json::Value {
        json!({
            "v": 1,
            "event": "operation",
            "observed_at_unix_ms": self.observed_at_unix_ms,
            "method": self.method,
            "risk": risk_name(self.risk),
            "outcome": outcome_name(self.outcome),
            "latency_ms": self.latency_ms,
            "queue_ms": self.queue_ms,
            "retries": self.retries,
            "reconciled": self.reconciled,
        })
    }
}

pub struct AuditLog(File);

impl AuditLog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AuditError> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(AuditError::PathNotAbsolute);
        }
        let (file, created) = open_file(path)?;
        validate_file(&file)?;
        if created {
            file.sync_all().map_err(io_error)?;
            File::open(path.parent().ok_or(AuditError::PathNotAbsolute)?)
                .and_then(|directory| directory.sync_all())
                .map_err(io_error)?;
        }
        Ok(Self(file))
    }

    pub fn append(&mut self, event: &AuditEvent) -> Result<(), AuditError> {
        serde_json::to_writer(&mut self.0, &event.json()).map_err(|_| AuditError::Serialize)?;
        self.0.write_all(b"\n").map_err(io_error)?;
        self.0.sync_data().map_err(io_error)
    }
}

fn open_file(path: &Path) -> Result<(File, bool), AuditError> {
    let mut options = OpenOptions::new();
    options
        .append(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    match options.open(path) {
        Ok(file) => Ok((file, true)),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => OpenOptions::new()
            .append(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path)
            .map(|file| (file, false))
            .map_err(io_error),
        Err(error) => Err(io_error(error)),
    }
}

fn validate_file(file: &File) -> Result<(), AuditError> {
    let metadata = file.metadata().map_err(io_error)?;
    // SAFETY: geteuid has no preconditions and doesn't access memory.
    let current_uid = unsafe { libc::geteuid() };
    if !metadata.file_type().is_file()
        || metadata.nlink() != 1
        || metadata.uid() != current_uid
        || metadata.mode() & 0o777 != 0o600
    {
        return Err(AuditError::UnsafeFile);
    }
    Ok(())
}

fn milliseconds(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

fn risk_name(risk: RiskClass) -> &'static str {
    match risk {
        RiskClass::Read => "read",
        RiskClass::Presence => "presence",
        RiskClass::Send => "send",
        RiskClass::ReversibleMutation => "reversible_mutation",
        RiskClass::Admin => "admin",
        RiskClass::Destructive => "destructive",
        RiskClass::Financial => "financial",
        RiskClass::AuthSecurity => "auth_security",
    }
}

fn outcome_name(outcome: OperationOutcome) -> &'static str {
    match outcome {
        OperationOutcome::Succeeded => "succeeded",
        OperationOutcome::Failed => "failed",
        OperationOutcome::Uncertain => "uncertain",
        OperationOutcome::Denied => "denied",
    }
}

fn io_error(error: io::Error) -> AuditError {
    AuditError::Io(error.kind())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditError {
    PathNotAbsolute,
    Io(io::ErrorKind),
    UnsafeFile,
    MethodNotReviewed,
    InvalidClock,
    Serialize,
}

impl fmt::Display for AuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathNotAbsolute => formatter.write_str("audit path must be absolute"),
            Self::Io(kind) => write!(formatter, "audit I/O failed: {kind:?}"),
            Self::UnsafeFile => formatter.write_str("audit file is unsafe"),
            Self::MethodNotReviewed => formatter.write_str("audit method is not reviewed"),
            Self::InvalidClock => formatter.write_str("audit clock is invalid"),
            Self::Serialize => formatter.write_str("audit serialization failed"),
        }
    }
}

impl std::error::Error for AuditError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use super::*;

    #[test]
    fn metrics_cover_required_operational_dimensions() {
        let telemetry = Telemetry::default();
        telemetry.record_request(Duration::from_millis(7), OperationOutcome::Succeeded);
        telemetry.observe_queue(3);
        telemetry.record_queue_rejection();
        telemetry.record_retry();
        telemetry.record_flood(Duration::from_millis(11));
        telemetry.observe_update_lag(Duration::from_millis(13));
        telemetry.observe_freshness(Freshness::Fresh);
        telemetry.observe_freshness(Freshness::Cached);
        telemetry.observe_freshness(Freshness::Stale);
        telemetry.observe_freshness(Freshness::Partial);
        telemetry.observe_leases(2);

        assert_eq!(
            telemetry.snapshot(),
            OperationalMetrics {
                requests: 1,
                succeeded: 1,
                failed: 0,
                uncertain: 0,
                denied: 0,
                request_latency_ms_total: 7,
                request_latency_ms_max: 7,
                queue_depth: 3,
                queue_depth_max: 3,
                queue_rejections: 1,
                retries: 1,
                flood_waits: 1,
                flood_delay_ms_total: 11,
                update_lag_events: 1,
                update_lag_ms_max: 13,
                fresh_results: 1,
                cached_results: 1,
                stale_results: 1,
                partial_results: 1,
                active_leases: 2,
                active_leases_max: 2,
            }
        );
    }

    #[test]
    fn audit_schema_cannot_persist_request_payload_or_identifiers() {
        const CANARY: &str = "TDLIB_TELEMETRY_SECRET_CANARY";
        let root = temporary_directory();
        fs::create_dir_all(&root).unwrap();
        let path = root.join("audit.jsonl");
        let request = ValidatedRequest::from_value(json!({
            "@type": "setChatDescription",
            "chat_id": 424242,
            "description": CANARY,
        }))
        .unwrap();
        let event = AuditEvent::operation(
            &request,
            OperationOutcome::Succeeded,
            Duration::from_millis(4),
            Duration::from_millis(2),
            0,
            true,
            SystemTime::now(),
        )
        .unwrap();
        let mut audit = AuditLog::open(&path).unwrap();
        audit.append(&event).unwrap();
        drop(audit);

        let output = fs::read_to_string(&path).unwrap();
        assert!(output.contains("setChatDescription"));
        assert!(!output.contains(CANARY));
        assert!(!output.contains("424242"));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_directory() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "telegramd-telemetry-{}-{nonce}",
            std::process::id()
        ))
    }
}
