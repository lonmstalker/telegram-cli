//! Единственная точка владения TDLib database profile.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

pub mod ownership;
pub mod socket;

use ownership::ProfileDatabaseLock;
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
    let _socket = match DaemonSocket::bind(&_ownership) {
        Ok(socket) => socket,
        Err(error) => {
            eprintln!("telegramd: {error}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("telegramd: profile socket bound; service protocol ещё не реализован");
    loop {
        std::thread::park();
    }
}
