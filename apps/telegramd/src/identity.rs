//! Stable owner-only binding между daemon profile и Telegram user.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use serde_json::Value;

const IDENTITY_FILE: &str = ".telegramd-identity";
const MAX_IDENTITY_BYTES: u64 = 32;

pub fn user_id_from_get_me(response: &Value) -> Result<i64, IdentityError> {
    let object = response
        .as_object()
        .filter(|object| object.get("@type").and_then(Value::as_str) == Some("user"))
        .ok_or(IdentityError::InvalidGetMe)?;
    let id = object
        .get("id")
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        })
        .filter(|id| *id > 0)
        .ok_or(IdentityError::InvalidGetMe)?;
    Ok(id)
}

pub fn verify_or_bind(
    database_directory: &Path,
    actual_user_id: i64,
    configured_user_id: Option<i64>,
) -> Result<(), IdentityError> {
    if configured_user_id.is_some_and(|expected| expected != actual_user_id) {
        return Err(IdentityError::Mismatch);
    }
    let path = database_directory.join(IDENTITY_FILE);
    match open_existing(&path) {
        Ok(mut file) => {
            validate_file(&file)?;
            let stored = read_user_id(&mut file)?;
            if stored != actual_user_id {
                return Err(IdentityError::Mismatch);
            }
            Ok(())
        }
        Err(IdentityError::Open(io::ErrorKind::NotFound)) => {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open(path)
                .map_err(|error| IdentityError::Create(error.kind()))?;
            validate_file(&file)?;
            writeln!(file, "{actual_user_id}")
                .and_then(|_| file.sync_all())
                .map_err(|error| IdentityError::Write(error.kind()))
        }
        Err(error) => Err(error),
    }
}

fn open_existing(path: &Path) -> Result<File, IdentityError> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| IdentityError::Open(error.kind()))
}

fn validate_file(file: &File) -> Result<(), IdentityError> {
    let metadata = file
        .metadata()
        .map_err(|error| IdentityError::Metadata(error.kind()))?;
    if !metadata.file_type().is_file() || metadata.nlink() != 1 {
        return Err(IdentityError::UnsafeFile);
    }
    // SAFETY: geteuid has no preconditions and does not access memory.
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o777 != 0o600 {
        return Err(IdentityError::UnsafeFile);
    }
    Ok(())
}

fn read_user_id(file: &mut File) -> Result<i64, IdentityError> {
    let mut bytes = Vec::new();
    file.take(MAX_IDENTITY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| IdentityError::Read(error.kind()))?;
    if bytes.len() as u64 > MAX_IDENTITY_BYTES {
        return Err(IdentityError::InvalidFile);
    }
    let value = std::str::from_utf8(&bytes)
        .ok()
        .map(str::trim)
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .ok_or(IdentityError::InvalidFile)?;
    Ok(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityError {
    InvalidGetMe,
    Open(io::ErrorKind),
    Create(io::ErrorKind),
    Metadata(io::ErrorKind),
    Read(io::ErrorKind),
    Write(io::ErrorKind),
    UnsafeFile,
    InvalidFile,
    Mismatch,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGetMe => formatter.write_str("TDLib getMe identity is invalid"),
            Self::Open(kind) => write!(formatter, "can't open profile identity: {kind:?}"),
            Self::Create(kind) => write!(formatter, "can't create profile identity: {kind:?}"),
            Self::Metadata(kind) => write!(formatter, "can't inspect profile identity: {kind:?}"),
            Self::Read(kind) => write!(formatter, "can't read profile identity: {kind:?}"),
            Self::Write(kind) => write!(formatter, "can't persist profile identity: {kind:?}"),
            Self::UnsafeFile => formatter.write_str("profile identity file is unsafe"),
            Self::InvalidFile => formatter.write_str("profile identity file is invalid"),
            Self::Mismatch => formatter.write_str("Telegram identity does not match the profile"),
        }
    }
}

impl std::error::Error for IdentityError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn first_get_me_binds_profile_and_later_mismatch_fails_closed() {
        let root = std::env::temp_dir().join(format!(
            "telegramd-identity-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        verify_or_bind(&root, 42, None).unwrap();
        verify_or_bind(&root, 42, Some(42)).unwrap();
        assert_eq!(
            verify_or_bind(&root, 43, None),
            Err(IdentityError::Mismatch)
        );
        fs::remove_dir_all(root).unwrap();
    }
}
