//! Единственная точка владения TDLib database profile.

use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use telegram_core::approval::ApprovalVerifier;
use telegram_core::idempotency::IdempotencyJournal;

pub mod config;
pub mod identity;
pub mod lease;
pub mod lifecycle;
pub mod ownership;
pub mod scheduler;
pub mod server;
pub mod socket;
pub mod telemetry;

use config::DaemonConfig;
use lease::LeaseManager;
use lifecycle::Lifecycle;
use ownership::ProfileDatabaseLock;
use server::LeaseServer;
use socket::DaemonSocket;
use telemetry::{AuditLog, Telemetry};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("telegramd: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let (profile, database_directory) = profile_config()?;
    let ownership = ProfileDatabaseLock::acquire(profile, database_directory)?;
    let config = DaemonConfig::from_environment(&ownership)?;
    let _approval_verifier = config
        .approval_public_key_hex()
        .map(ApprovalVerifier::from_hex)
        .transpose()?;
    let _idempotency_journal = IdempotencyJournal::open(
        ownership
            .canonical_database_directory()
            .join(".telegramd-idempotency.jsonl"),
    )?;
    let _audit_log = AuditLog::open(
        ownership
            .canonical_database_directory()
            .join(".telegramd-audit.jsonl"),
    )?;
    let socket = DaemonSocket::bind(&ownership)?;
    let mut lifecycle = Lifecycle::new(config.idle_timeout());
    lifecycle.start()?;
    eprintln!("telegramd: Starting");

    let database_key = config.load_database_key()?;
    let backend = config.load_native()?;
    let mut runtime = lifecycle::start_runtime(backend)?;
    let readiness = lifecycle::reach_ready(&mut runtime, &config, &database_key, &ownership)?;
    let risk_scopes = config.risk_scopes().collect::<Vec<_>>();
    let expected_user_id = config.expected_user_id();
    drop(database_key);
    drop(config);
    let mut server = LeaseServer::new(LeaseManager::with_telemetry(
        risk_scopes,
        Telemetry::default(),
    ));
    server.start_events_at(
        runtime
            .state()
            .last_sequence()
            .map(|sequence| sequence.get()),
    );
    server.observe_authorization(&runtime)?;
    if readiness == lifecycle::AuthorizationReadiness::InteractiveRequired {
        lifecycle::serve_until_authorized(
            &mut runtime,
            &socket,
            &mut server,
            &ownership,
            expected_user_id,
        )?;
    }
    lifecycle.ready(std::time::Instant::now())?;
    eprintln!("telegramd: Ready");

    lifecycle::serve_until_idle(runtime, socket, server, &mut lifecycle)?;
    eprintln!("telegramd: Closed");
    Ok(())
}

fn profile_config() -> Result<(String, PathBuf), io::Error> {
    let profile = env::var("TELEGRAM_PROFILE");
    let database_directory = env::var_os("TDLIB_DATABASE_DIR");
    match (profile, database_directory) {
        (Err(env::VarError::NotPresent), None) => {
            Err(io::Error::other("runtime ещё не реализован"))
        }
        (Ok(profile), Some(database_directory)) => Ok((profile, PathBuf::from(database_directory))),
        (Err(env::VarError::NotPresent), Some(database_directory)) => {
            Ok(("default".to_owned(), PathBuf::from(database_directory)))
        }
        _ => Err(io::Error::other("profile configuration is incomplete")),
    }
}
