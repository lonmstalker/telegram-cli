//! Private Unix socket namespace для выбранного profile owner.

use std::fmt;
use std::fs::{self, Permissions};
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::ownership::ProfileDatabaseLock;

const DAEMON_SOCKET_SUFFIX: &str = ".sock";
static UMASK_LOCK: Mutex<()> = Mutex::new(());

pub struct DaemonSocket {
    listener: UnixListener,
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl DaemonSocket {
    pub fn bind(ownership: &ProfileDatabaseLock) -> Result<Self, SocketError> {
        let path = socket_path(ownership.profile());
        recover_stale_socket(&path)?;

        let restrictive_umask = RestrictiveUmask::enter();
        let listener =
            UnixListener::bind(&path).map_err(|error| SocketError::Bind(error.kind()))?;
        drop(restrictive_umask);
        fs::set_permissions(&path, Permissions::from_mode(0o600))
            .map_err(|error| SocketError::Permissions(error.kind()))?;
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| SocketError::Metadata(error.kind()))?;
        validate_socket_metadata(&metadata)?;

        Ok(Self {
            listener,
            path,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub fn listener(&self) -> &UnixListener {
        &self.listener
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn socket_path(profile: &str) -> PathBuf {
    // SAFETY: geteuid has no preconditions and does not access memory.
    let user = unsafe { libc::geteuid() };
    PathBuf::from(format!(
        "/tmp/telegramd-{user}-{profile}{DAEMON_SOCKET_SUFFIX}"
    ))
}

impl Drop for DaemonSocket {
    fn drop(&mut self) {
        let Ok(metadata) = fs::symlink_metadata(&self.path) else {
            return;
        };
        if metadata.dev() == self.device && metadata.ino() == self.inode {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn recover_stale_socket(path: &Path) -> Result<(), SocketError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(SocketError::Inspect(error.kind())),
    };
    validate_socket_identity(&metadata)?;
    match UnixStream::connect(path) {
        Ok(_) => Err(SocketError::AlreadyServing),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            if error.kind() != io::ErrorKind::NotFound {
                fs::remove_file(path).map_err(|remove| SocketError::RemoveStale(remove.kind()))?;
            }
            Ok(())
        }
        Err(error) => Err(SocketError::Probe(error.kind())),
    }
}

fn validate_socket_identity(metadata: &fs::Metadata) -> Result<(), SocketError> {
    if !metadata.file_type().is_socket() || metadata.nlink() != 1 {
        return Err(SocketError::UnsafeSocketEntry);
    }
    // SAFETY: geteuid has no preconditions and does not access memory.
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(SocketError::UnsafeSocketEntry);
    }
    Ok(())
}

fn validate_socket_metadata(metadata: &fs::Metadata) -> Result<(), SocketError> {
    validate_socket_identity(metadata)?;
    if metadata.mode() & 0o777 != 0o600 {
        return Err(SocketError::UnsafeSocketMode);
    }
    Ok(())
}

struct RestrictiveUmask {
    previous: libc::mode_t,
    _lock: MutexGuard<'static, ()>,
}

impl RestrictiveUmask {
    fn enter() -> Self {
        let lock = UMASK_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: daemon startup is single-threaded; the process-global change is
        // serialized here and restored by Drop before the listener is exposed.
        let previous = unsafe { libc::umask(0o177) };
        Self {
            previous,
            _lock: lock,
        }
    }
}

impl Drop for RestrictiveUmask {
    fn drop(&mut self) {
        // SAFETY: previous came from umask and restoring it has no pointer arguments.
        unsafe { libc::umask(self.previous) };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketError {
    Inspect(io::ErrorKind),
    UnsafeSocketEntry,
    AlreadyServing,
    Probe(io::ErrorKind),
    RemoveStale(io::ErrorKind),
    Bind(io::ErrorKind),
    Permissions(io::ErrorKind),
    Metadata(io::ErrorKind),
    UnsafeSocketMode,
}

impl fmt::Display for SocketError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inspect(kind) => write!(formatter, "can't inspect profile socket: {kind:?}"),
            Self::UnsafeSocketEntry => {
                formatter.write_str("profile socket path contains an unsafe entry")
            }
            Self::AlreadyServing => formatter.write_str("profile socket is already serving"),
            Self::Probe(kind) => write!(formatter, "can't probe profile socket: {kind:?}"),
            Self::RemoveStale(kind) => {
                write!(formatter, "can't remove stale profile socket: {kind:?}")
            }
            Self::Bind(kind) => write!(formatter, "can't bind profile socket: {kind:?}"),
            Self::Permissions(kind) => {
                write!(formatter, "can't protect profile socket: {kind:?}")
            }
            Self::Metadata(kind) => {
                write!(formatter, "can't inspect bound profile socket: {kind:?}")
            }
            Self::UnsafeSocketMode => formatter.write_str("profile socket mode is not 0600"),
        }
    }
}

impl std::error::Error for SocketError {}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn socket_is_private_and_live_listener_is_never_replaced() {
        let (root, profile) = temporary_scope("private-socket");
        fs::create_dir_all(&root).unwrap();
        let ownership = ProfileDatabaseLock::acquire(profile, &root).unwrap();
        let socket = DaemonSocket::bind(&ownership).unwrap();
        let metadata = fs::symlink_metadata(socket.path()).unwrap();
        assert_eq!(metadata.mode() & 0o777, 0o600);
        assert!(UnixStream::connect(socket.path()).is_ok());
        assert_eq!(
            DaemonSocket::bind(&ownership).err(),
            Some(SocketError::AlreadyServing)
        );
        let path = socket.path().to_owned();
        drop(socket);
        assert!(!path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_socket_is_recovered_but_regular_file_is_preserved() {
        let (root, profile) = temporary_scope("stale-socket");
        fs::create_dir_all(&root).unwrap();
        let ownership = ProfileDatabaseLock::acquire(profile, &root).unwrap();
        let path = socket_path(ownership.profile());
        drop(UnixListener::bind(&path).unwrap());

        drop(DaemonSocket::bind(&ownership).unwrap());
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            DaemonSocket::bind(&ownership).err(),
            Some(SocketError::UnsafeSocketEntry)
        );
        assert!(path.is_file());
        fs::remove_file(path).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_scope(name: &str) -> (PathBuf, String) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let profile = format!("{name}-{}-{nonce:x}", std::process::id());
        (
            std::env::temp_dir().join(format!("telegramd-{profile}")),
            profile,
        )
    }
}
