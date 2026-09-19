//! Единственная точка владения TDLib database profile.

use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use telegram_core::approval::ApprovalVerifier;
use telegram_core::idempotency::IdempotencyJournal;

pub mod authorization;
mod chat_inputs;
pub mod config;
mod discovery;
pub mod identity;
mod ipc;
pub mod lease;
pub mod lifecycle;
pub mod ownership;
pub mod scheduler;
pub mod server;
pub mod socket;
pub mod telemetry;
mod workflow_catalog;

use authorization::AuthorizationCoordinator;
use config::DaemonConfig;
use lease::LeaseManager;
use lifecycle::Lifecycle;
use ownership::ProfileDatabaseLock;
use scheduler::{AccountScheduler, serial_daemon_budgets};
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
    if env::args_os().skip(1).eq(["--discover"]) {
        use std::io::Read;
        let mut bytes = zeroize::Zeroizing::new(Vec::new());
        io::stdin().take(16 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 {
            return Err(io::Error::other("discovery input too large").into());
        }
        let request = serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::other("invalid discovery request"))?;
        serde_json::to_writer(io::stdout().lock(), &discovery::respond(request, None))?;
        return Ok(());
    }
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 2 && args[0] == "--check-native" {
        config::check_native(std::path::Path::new(&args[1]))?;
        return Ok(());
    }
    if env::args_os().len() != 1 {
        return Err(io::Error::other("usage: telegramd [--discover | --check-native PATH]").into());
    }

    let (profile, database_directory) = profile_config()?;
    let ownership = ProfileDatabaseLock::acquire(profile, database_directory)?;
    let config = DaemonConfig::from_environment(&ownership)?;
    let approval_verifier = config
        .approval_public_key_hex()
        .map(ApprovalVerifier::from_hex)
        .transpose()?;
    let idempotency_journal = IdempotencyJournal::open(
        ownership
            .canonical_database_directory()
            .join(".telegramd-idempotency.jsonl"),
    )?;
    let audit_log = AuditLog::open(
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
    let mut authorization = AuthorizationCoordinator::default();
    let readiness = lifecycle::reach_ready(
        &mut runtime,
        &config,
        &database_key,
        &ownership,
        &mut authorization,
    )?;
    let risk_scopes = config.risk_scopes().collect::<Vec<_>>();
    let expected_user_id = config.expected_user_id();
    let files_directory = config.files_directory().to_owned();
    drop(database_key);
    drop(config);
    let telemetry = Telemetry::default();
    let scheduler = AccountScheduler::with_telemetry(serial_daemon_budgets(), telemetry.clone())?;
    let mut server = LeaseServer::new(
        LeaseManager::with_telemetry(risk_scopes, telemetry.clone()),
        scheduler,
        telemetry,
        idempotency_journal,
        audit_log,
        authorization,
    )
    .with_artifact_root(files_directory)
    .with_approval_verifier(approval_verifier);
    server.start_events_at(
        runtime
            .state()
            .last_sequence()
            .map(|sequence| sequence.get()),
    );
    let readiness = if readiness == lifecycle::AuthorizationReadiness::InteractiveRequired {
        lifecycle::serve_until_authorized(
            &mut runtime,
            &socket,
            &mut server,
            &ownership,
            expected_user_id,
        )?
    } else {
        readiness
    };
    if readiness == lifecycle::AuthorizationReadiness::ExternalShutdown {
        eprintln!("telegramd: external authorization shutdown; waiting for close");
        drop(socket);
        lifecycle::finish_external_shutdown(runtime)?;
        eprintln!("telegramd: Closed");
        return Ok(());
    }
    lifecycle.ready(std::time::Instant::now())?;
    eprintln!("telegramd: Ready");

    lifecycle::serve_until_idle(
        runtime,
        socket,
        server,
        &mut lifecycle,
        &ownership,
        expected_user_id,
    )?;
    eprintln!("telegramd: Closed");
    Ok(())
}

fn profile_config() -> Result<(String, PathBuf), io::Error> {
    let profile = env::var("TELEGRAM_PROFILE");
    let database_directory = env::var_os("TDLIB_DATABASE_DIR");
    match (profile, database_directory) {
        (Err(env::VarError::NotPresent), None) => Err(io::Error::other(
            "profile is not configured; run telegram-cli setup",
        )),
        (Ok(profile), Some(database_directory)) => Ok((profile, PathBuf::from(database_directory))),
        (Err(env::VarError::NotPresent), Some(database_directory)) => {
            Ok(("default".to_owned(), PathBuf::from(database_directory)))
        }
        _ => Err(io::Error::other("profile configuration is incomplete")),
    }
}
