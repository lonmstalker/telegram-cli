//! Единственная точка владения TDLib database profile.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

pub mod lease;
pub mod ownership;
pub mod server;
pub mod socket;

use lease::LeaseManager;
use ownership::ProfileDatabaseLock;
use server::LeaseServer;
use socket::DaemonSocket;

fn main() -> ExitCode {
    let profile = env::var("TELEGRAM_PROFILE");
    let database_directory = env::var_os("TDLIB_DATABASE_DIR");
    let (profile, database_directory) = match (profile, database_directory) {
        (Err(env::VarError::NotPresent), None) => {
            eprintln!("telegramd: runtime ещё не реализован");
            return ExitCode::FAILURE;
        }
        (Ok(profile), Some(database_directory)) => (profile, PathBuf::from(database_directory)),
        _ => {
            eprintln!("telegramd: profile configuration is incomplete");
            return ExitCode::FAILURE;
        }
    };
    let _ownership = match ProfileDatabaseLock::acquire(profile, database_directory) {
        Ok(ownership) => ownership,
        Err(error) => {
            eprintln!("telegramd: {error}");
            return ExitCode::FAILURE;
        }
    };
    let socket = match DaemonSocket::bind(&_ownership) {
        Ok(socket) => socket,
        Err(error) => {
            eprintln!("telegramd: {error}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("telegramd: lease service ready; TDLib runtime ещё не реализован");
    let mut server = LeaseServer::new(LeaseManager::new());
    if let Err(error) = server.run(socket.listener()) {
        eprintln!("telegramd: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
