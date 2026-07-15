//! Защищённые источники database encryption key для pinned TDLib.

use std::ffi::OsStr;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use base64::Engine as _;
use serde_json::json;
use zeroize::Zeroizing;

use crate::authorization::{AuthorizationError, AuthorizationRequest, SensitiveString};

const MAX_DATABASE_KEY_BYTES: u64 = 4_096;

pub enum DatabaseKeySource {
    FileDescriptor(OwnedFd),
    FileSecret(PathBuf),
    Base64FileSecret(PathBuf),
    OsKeychain { service: String, account: String },
}

impl fmt::Debug for DatabaseKeySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileDescriptor(_) => formatter.write_str("FileDescriptor(<redacted>)"),
            Self::FileSecret(_) => formatter.write_str("FileSecret(<redacted>)"),
            Self::Base64FileSecret(_) => formatter.write_str("Base64FileSecret(<redacted>)"),
            Self::OsKeychain { .. } => formatter.write_str("OsKeychain(<redacted>)"),
        }
    }
}

pub struct DatabaseKey(Zeroizing<Vec<u8>>);

impl DatabaseKey {
    pub fn load(source: DatabaseKeySource) -> Result<Self, DatabaseKeyError> {
        let bytes = match source {
            DatabaseKeySource::FileDescriptor(fd) => read_key(File::from(fd))?,
            DatabaseKeySource::FileSecret(path) => read_file_secret(&path)?,
            DatabaseKeySource::Base64FileSecret(path) => read_base64_file_secret(&path)?,
            DatabaseKeySource::OsKeychain { service, account } => {
                read_os_keychain(&service, &account)?
            }
        };
        Self::from_bytes(bytes)
    }

    fn from_bytes(bytes: Zeroizing<Vec<u8>>) -> Result<Self, DatabaseKeyError> {
        if bytes.is_empty() {
            return Err(DatabaseKeyError::Empty);
        }
        Ok(Self(bytes))
    }

    pub(crate) fn tdjson_base64(&self) -> Zeroizing<String> {
        Zeroizing::new(base64::engine::general_purpose::STANDARD.encode(self.0.as_slice()))
    }
}

impl fmt::Debug for DatabaseKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DatabaseKey(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseKeyError {
    PathMustBeAbsolute,
    Open(io::ErrorKind),
    NotRegularFile,
    InsecureFileMode,
    WrongFileOwner,
    Read(io::ErrorKind),
    Empty,
    TooLarge,
    InvalidKeychainReference,
    InvalidBase64,
    KeychainUnavailable,
    KeychainReadFailed,
}

impl fmt::Display for DatabaseKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathMustBeAbsolute => formatter.write_str("database key path must be absolute"),
            Self::Open(kind) => write!(formatter, "can't open database key source: {kind:?}"),
            Self::NotRegularFile => {
                formatter.write_str("database key source is not a regular file")
            }
            Self::InsecureFileMode => {
                formatter.write_str("database key file mode must be exactly 0600")
            }
            Self::WrongFileOwner => {
                formatter.write_str("database key file must be owned by the current user")
            }
            Self::Read(kind) => write!(formatter, "can't read database key source: {kind:?}"),
            Self::Empty => formatter.write_str("database key is missing or empty"),
            Self::TooLarge => formatter.write_str("database key exceeds 4096 bytes"),
            Self::InvalidKeychainReference => {
                formatter.write_str("OS keychain reference is invalid")
            }
            Self::InvalidBase64 => formatter.write_str("database key file is not valid Base64"),
            Self::KeychainUnavailable => formatter.write_str("OS keychain is unavailable"),
            Self::KeychainReadFailed => formatter.write_str("OS keychain lookup failed"),
        }
    }
}

impl std::error::Error for DatabaseKeyError {}

pub struct TdlibParameters {
    pub use_test_dc: bool,
    pub database_directory: PathBuf,
    pub files_directory: PathBuf,
    pub use_file_database: bool,
    pub use_chat_info_database: bool,
    pub use_message_database: bool,
    pub use_secret_chats: bool,
    pub api_id: i32,
    pub api_hash: SensitiveString,
    pub system_language_code: String,
    pub device_model: String,
    pub system_version: String,
    pub application_version: String,
}

impl TdlibParameters {
    pub(crate) fn into_request(
        self,
        key: &DatabaseKey,
    ) -> Result<AuthorizationRequest, AuthorizationError> {
        self.validate()?;
        let encoded_key = key.tdjson_base64();
        let database_directory = self
            .database_directory
            .to_str()
            .ok_or(AuthorizationError::InvalidField("database_directory"))?;
        let files_directory = self
            .files_directory
            .to_str()
            .ok_or(AuthorizationError::InvalidField("files_directory"))?;
        Ok(AuthorizationRequest::new(json!({
            "@type": "setTdlibParameters",
            "use_test_dc": self.use_test_dc,
            "database_directory": database_directory,
            "files_directory": files_directory,
            "database_encryption_key": encoded_key.as_str(),
            "use_file_database": self.use_file_database,
            "use_chat_info_database": self.use_chat_info_database,
            "use_message_database": self.use_message_database,
            "use_secret_chats": self.use_secret_chats,
            "api_id": self.api_id,
            "api_hash": self.api_hash.expose_secret(),
            "system_language_code": self.system_language_code,
            "device_model": self.device_model,
            "system_version": self.system_version,
            "application_version": self.application_version
        })))
    }

    fn validate(&self) -> Result<(), AuthorizationError> {
        if !self.database_directory.is_absolute() {
            return Err(AuthorizationError::InvalidField("database_directory"));
        }
        if !self.files_directory.is_absolute() {
            return Err(AuthorizationError::InvalidField("files_directory"));
        }
        if self.api_id <= 0 {
            return Err(AuthorizationError::InvalidField("api_id"));
        }
        if self.api_hash.expose_secret().is_empty() {
            return Err(AuthorizationError::InvalidField("api_hash"));
        }
        for (value, field) in [
            (&self.system_language_code, "system_language_code"),
            (&self.device_model, "device_model"),
            (&self.system_version, "system_version"),
            (&self.application_version, "application_version"),
        ] {
            if value.is_empty() {
                return Err(AuthorizationError::InvalidField(field));
            }
        }
        Ok(())
    }
}

impl fmt::Debug for TdlibParameters {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TdlibParameters")
            .field("use_test_dc", &self.use_test_dc)
            .field("database_directory", &"<redacted>")
            .field("files_directory", &"<redacted>")
            .field("api_id", &self.api_id)
            .field("api_hash", &"<redacted>")
            .finish_non_exhaustive()
    }
}

fn read_file_secret(path: &Path) -> Result<Zeroizing<Vec<u8>>, DatabaseKeyError> {
    if !path.is_absolute() {
        return Err(DatabaseKeyError::PathMustBeAbsolute);
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| DatabaseKeyError::Open(error.kind()))?;
    let metadata = file
        .metadata()
        .map_err(|error| DatabaseKeyError::Read(error.kind()))?;
    if !metadata.file_type().is_file() {
        return Err(DatabaseKeyError::NotRegularFile);
    }
    if metadata.mode() & 0o777 != 0o600 {
        return Err(DatabaseKeyError::InsecureFileMode);
    }
    // SAFETY: geteuid has no preconditions and does not access memory.
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(DatabaseKeyError::WrongFileOwner);
    }
    read_key(file)
}

fn read_base64_file_secret(path: &Path) -> Result<Zeroizing<Vec<u8>>, DatabaseKeyError> {
    let mut encoded = read_file_secret(path)?;
    if encoded.ends_with(b"\n") {
        encoded.pop();
        if encoded.ends_with(b"\r") {
            encoded.pop();
        }
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_slice())
        .map_err(|_| DatabaseKeyError::InvalidBase64)?;
    Ok(Zeroizing::new(decoded))
}

fn read_key(mut source: impl Read) -> Result<Zeroizing<Vec<u8>>, DatabaseKeyError> {
    let mut bytes = Zeroizing::new(Vec::new());
    source
        .by_ref()
        .take(MAX_DATABASE_KEY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| DatabaseKeyError::Read(error.kind()))?;
    if bytes.len() as u64 > MAX_DATABASE_KEY_BYTES {
        return Err(DatabaseKeyError::TooLarge);
    }
    Ok(bytes)
}

fn read_os_keychain(service: &str, account: &str) -> Result<Zeroizing<Vec<u8>>, DatabaseKeyError> {
    if service.is_empty() || account.is_empty() || service.contains('\0') || account.contains('\0')
    {
        return Err(DatabaseKeyError::InvalidKeychainReference);
    }

    #[cfg(target_os = "macos")]
    let output = run_keychain_command(
        "security",
        ["find-generic-password", "-w", "-s", service, "-a", account],
    )?;
    #[cfg(target_os = "linux")]
    let output = run_keychain_command(
        "secret-tool",
        ["lookup", "service", service, "account", account],
    )?;
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err(DatabaseKeyError::KeychainUnavailable);

    if !output.status.success() {
        return Err(DatabaseKeyError::KeychainReadFailed);
    }
    let mut bytes = Zeroizing::new(output.stdout);
    if bytes.ends_with(b"\n") {
        bytes.pop();
        if bytes.ends_with(b"\r") {
            bytes.pop();
        }
    }
    if bytes.len() as u64 > MAX_DATABASE_KEY_BYTES {
        return Err(DatabaseKeyError::TooLarge);
    }
    Ok(bytes)
}

fn run_keychain_command<I, S>(program: &str, arguments: I) -> Result<Output, DatabaseKeyError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(program)
        .args(arguments)
        .output()
        .map_err(|_| DatabaseKeyError::KeychainUnavailable)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::fd::OwnedFd;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temporary_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "telegram-core-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn descriptor_and_mode_0600_file_load_without_exposing_key() {
        let path = temporary_path("database-key");
        fs::write(&path, b"synthetic-key").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

        let key = DatabaseKey::load(DatabaseKeySource::FileSecret(path.clone())).unwrap();
        assert_eq!(format!("{key:?}"), "DatabaseKey(<redacted>)");
        assert_eq!(key.tdjson_base64().as_str(), "c3ludGhldGljLWtleQ==");

        let fd: OwnedFd = File::open(&path).unwrap().into();
        let descriptor_key = DatabaseKey::load(DatabaseKeySource::FileDescriptor(fd)).unwrap();
        assert_eq!(
            descriptor_key.tdjson_base64().as_str(),
            "c3ludGhldGljLWtleQ=="
        );
        fs::write(&path, b"c3ludGhldGljLWtleQ==\n").unwrap();
        let encoded = DatabaseKey::load(DatabaseKeySource::Base64FileSecret(path.clone())).unwrap();
        assert_eq!(encoded.tdjson_base64().as_str(), "c3ludGhldGljLWtleQ==");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn file_source_rejects_insecure_mode_symlink_and_empty_value() {
        let path = temporary_path("bad-database-key");
        fs::write(&path, b"synthetic-key").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            DatabaseKey::load(DatabaseKeySource::FileSecret(path.clone())).unwrap_err(),
            DatabaseKeyError::InsecureFileMode
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let link = temporary_path("database-key-link");
        symlink(&path, &link).unwrap();
        assert!(matches!(
            DatabaseKey::load(DatabaseKeySource::FileSecret(link.clone())),
            Err(DatabaseKeyError::Open(_))
        ));
        fs::write(&path, b"").unwrap();
        assert_eq!(
            DatabaseKey::load(DatabaseKeySource::FileSecret(path.clone())).unwrap_err(),
            DatabaseKeyError::Empty
        );
        fs::remove_file(link).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn parameters_use_base64_and_wrong_key_blocks_phone_fallback() {
        use crate::authorization::{AuthorizationMachine, AuthorizationStep};

        let key = DatabaseKey::from_bytes(Zeroizing::new(b"synthetic-key".to_vec())).unwrap();
        let parameters = TdlibParameters {
            use_test_dc: false,
            database_directory: PathBuf::from("/tmp/telegram-db"),
            files_directory: PathBuf::from("/tmp/telegram-files"),
            use_file_database: true,
            use_chat_info_database: true,
            use_message_database: true,
            use_secret_chats: false,
            api_id: 1,
            api_hash: SensitiveString::new("synthetic-api-hash"),
            system_language_code: "en".to_owned(),
            device_model: "test".to_owned(),
            system_version: "test".to_owned(),
            application_version: "test".to_owned(),
        };
        let mut machine = AuthorizationMachine::default();
        let generation = match machine
            .observe_state(&json!({"@type": "authorizationStateWaitTdlibParameters"}))
            .unwrap()
        {
            AuthorizationStep::ParametersRequired { generation } => generation,
            other => panic!("expected parameters, got {other:?}"),
        };
        let request = machine
            .submit_parameters(generation, parameters, &key)
            .unwrap();
        assert_eq!(
            request.into_value()["database_encryption_key"],
            "c3ludGhldGljLWtleQ=="
        );

        machine.parameters_failed(generation, 401).unwrap();
        assert_eq!(
            machine
                .observe_state(&json!({"@type": "authorizationStateWaitPhoneNumber"}))
                .unwrap_err(),
            AuthorizationError::DatabaseKeyRejected
        );
        assert!(matches!(
            machine.current_state(),
            Some(crate::authorization::AuthorizationState::WaitTdlibParameters)
        ));
    }
}
