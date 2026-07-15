//! Единственная точка владения TDLib database profile.

use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use telegram_core::idempotency::IdempotencyJournal;

pub mod config;
pub mod identity;
pub mod lease;
pub mod lifecycle;
pub mod ownership;
pub mod scheduler;
pub mod server;
pub mod socket;

use config::DaemonConfig;
use lease::LeaseManager;
use lifecycle::Lifecycle;
use ownership::ProfileDatabaseLock;
use server::LeaseServer;
use socket::DaemonSocket;

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
    let _idempotency_journal = IdempotencyJournal::open(
        ownership
            .canonical_database_directory()
            .join(".telegramd-idempotency.jsonl"),
    )?;
    let socket = DaemonSocket::bind(&ownership)?;
    let mut lifecycle = Lifecycle::new(config.idle_timeout());
    lifecycle.start()?;
    eprintln!("telegramd: Starting");

    let database_key = config.load_database_key()?;
    let backend = config.load_native()?;
    let mut runtime = lifecycle::start_runtime(backend)?;
    lifecycle::reach_ready(&mut runtime, &config, &database_key, &ownership)?;
    drop(database_key);
    drop(config);
    lifecycle.ready(std::time::Instant::now())?;
    eprintln!("telegramd: Ready");

    let server = LeaseServer::new(LeaseManager::new());
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
