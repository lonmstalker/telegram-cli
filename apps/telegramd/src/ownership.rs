//! Exclusive OS ownership для canonical TDLib database directory.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const OWNER_LOCK_FILE: &str = ".telegramd-owner.lock";

pub struct ProfileDatabaseLock {
    profile: String,
    canonical_database_directory: PathBuf,
    _file: File,
}

impl ProfileDatabaseLock {
    pub fn acquire(
        profile: impl Into<String>,
        database_directory: impl AsRef<Path>,
    ) -> Result<Self, OwnershipError> {
        let profile = profile.into();
        if profile.is_empty() {
            return Err(OwnershipError::EmptyProfile);
        }
        let database_directory = database_directory.as_ref();
        if !database_directory.is_absolute() {
            return Err(OwnershipError::DatabasePathMustBeAbsolute);
        }
        let canonical_database_directory = fs::canonicalize(database_directory)
            .map_err(|error| OwnershipError::Canonicalize(error.kind()))?;
        if !canonical_database_directory.is_dir() {
            return Err(OwnershipError::DatabasePathNotDirectory);
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(canonical_database_directory.join(OWNER_LOCK_FILE))
            .map_err(|error| OwnershipError::Open(error.kind()))?;
        validate_lock_file(&file)?;

        // SAFETY: flock only reads the live file descriptor and has no pointer arguments.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = io::Error::last_os_error();
            return match error.raw_os_error() {
                Some(libc::EAGAIN) => Err(OwnershipError::AlreadyOwned),
                _ => Err(OwnershipError::Lock(error.kind())),
            };
        }
        Ok(Self {
            profile,
            canonical_database_directory,
            _file: file,
        })
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn canonical_database_directory(&self) -> &Path {
        &self.canonical_database_directory
    }
}

fn validate_lock_file(file: &File) -> Result<(), OwnershipError> {
    let metadata = file
        .metadata()
        .map_err(|error| OwnershipError::Metadata(error.kind()))?;
    if !metadata.file_type().is_file() || metadata.nlink() != 1 {
        return Err(OwnershipError::InvalidLockFile);
    }
    // SAFETY: geteuid has no preconditions and does not access memory.
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(OwnershipError::WrongLockFileOwner);
    }
    if metadata.mode() & 0o777 != 0o600 {
        return Err(OwnershipError::InsecureLockFileMode);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipError {
    EmptyProfile,
    DatabasePathMustBeAbsolute,
    Canonicalize(io::ErrorKind),
    DatabasePathNotDirectory,
    Open(io::ErrorKind),
    Metadata(io::ErrorKind),
    InvalidLockFile,
    WrongLockFileOwner,
    InsecureLockFileMode,
    AlreadyOwned,
    Lock(io::ErrorKind),
}

impl fmt::Display for OwnershipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProfile => formatter.write_str("profile name is empty"),
            Self::DatabasePathMustBeAbsolute => {
                formatter.write_str("TDLib database path must be absolute")
            }
            Self::Canonicalize(kind) => {
                write!(
                    formatter,
                    "can't canonicalize TDLib database path: {kind:?}"
                )
            }
            Self::DatabasePathNotDirectory => {
                formatter.write_str("TDLib database path is not a directory")
            }
            Self::Open(kind) => write!(formatter, "can't open database owner lock: {kind:?}"),
            Self::Metadata(kind) => {
                write!(formatter, "can't inspect database owner lock: {kind:?}")
            }
            Self::InvalidLockFile => formatter.write_str("database owner lock is not a safe file"),
            Self::WrongLockFileOwner => {
                formatter.write_str("database owner lock has a different owner")
            }
            Self::InsecureLockFileMode => {
                formatter.write_str("database owner lock mode must be exactly 0600")
            }
            Self::AlreadyOwned => {
                formatter.write_str("TDLib database is already owned by another daemon")
            }
            Self::Lock(kind) => write!(formatter, "can't lock TDLib database: {kind:?}"),
        }
    }
}

impl std::error::Error for OwnershipError {}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn canonical_aliases_allow_exactly_one_owner_until_drop() {
        let root = temporary_directory("canonical-lock");
        let database = root.join("database");
        let alias = root.join("alias");
        fs::create_dir_all(&database).unwrap();
        symlink(&database, &alias).unwrap();

        let owner = ProfileDatabaseLock::acquire("primary", &database).unwrap();
        assert_eq!(owner.profile(), "primary");
        assert_eq!(
            owner.canonical_database_directory(),
            fs::canonicalize(&database).unwrap()
        );
        assert!(matches!(
            ProfileDatabaseLock::acquire("alias-profile", &alias),
            Err(OwnershipError::AlreadyOwned)
        ));

        drop(owner);
        drop(ProfileDatabaseLock::acquire("alias-profile", &alias).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_profile_or_database_path_fails_before_locking() {
        let root = temporary_directory("invalid-lock");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("file");
        fs::write(&file, b"not a directory").unwrap();

        assert_eq!(
            ProfileDatabaseLock::acquire("", &root).err(),
            Some(OwnershipError::EmptyProfile)
        );
        assert_eq!(
            ProfileDatabaseLock::acquire("primary", Path::new("relative")).err(),
            Some(OwnershipError::DatabasePathMustBeAbsolute)
        );
        assert_eq!(
            ProfileDatabaseLock::acquire("primary", &file).err(),
            Some(OwnershipError::DatabasePathNotDirectory)
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_directory(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("telegramd-{name}-{}-{nonce}", std::process::id()))
    }
}
