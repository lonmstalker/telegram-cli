//! Protected runtime configuration for the daemon-owned TDLib client.

use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};
use telegram_core::NativeTdJson;
use telegram_core::authorization::SensitiveString;
use telegram_core::database_key::{
    DatabaseKey, DatabaseKeyError, DatabaseKeySource, TdlibParameters,
};

use crate::ownership::ProfileDatabaseLock;

#[cfg(target_os = "macos")]
const NATIVE_PROVENANCE: &str =
    include_str!("../../../vendor/tdlib/native-builds/aarch64-apple-darwin.json");
#[cfg(target_os = "linux")]
const NATIVE_PROVENANCE: &str =
    include_str!("../../../vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json");
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
compile_error!("telegramd supports the declared macOS/Linux targets only");

const DEFAULT_IDLE_TIMEOUT_MS: u64 = 30_000;

pub struct DaemonConfig {
    database_directory: PathBuf,
    files_directory: PathBuf,
    database_key_file: PathBuf,
    native_library: PathBuf,
    native_sha256: String,
    native_bytes: u64,
    use_test_dc: bool,
    api_id: i32,
    api_hash: SensitiveString,
    expected_user_id: Option<i64>,
    idle_timeout: Duration,
}

impl DaemonConfig {
    pub fn from_environment(ownership: &ProfileDatabaseLock) -> Result<Self, ConfigError> {
        let provenance: Value =
            serde_json::from_str(NATIVE_PROVENANCE).map_err(|_| ConfigError::InvalidNativePin)?;
        let artifact = provenance
            .get("artifact")
            .and_then(Value::as_object)
            .ok_or(ConfigError::InvalidNativePin)?;
        let native_sha256 = required_json_string(artifact.get("sha256"))?.to_owned();
        let native_bytes = artifact
            .get("bytes")
            .and_then(Value::as_u64)
            .ok_or(ConfigError::InvalidNativePin)?;
        let cache_path = required_json_string(artifact.get("cache_path"))?;
        let native_library = env::var_os("TDJSON_LIBRARY_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace_root().join(cache_path));

        let files_directory = required_path("TDLIB_FILES_DIR")?;
        if !files_directory.is_absolute() {
            return Err(ConfigError::PathMustBeAbsolute("TDLIB_FILES_DIR"));
        }
        let database_key_file = required_path("TDLIB_DATABASE_KEY_FILE")?;
        if !database_key_file.is_absolute() {
            return Err(ConfigError::PathMustBeAbsolute("TDLIB_DATABASE_KEY_FILE"));
        }
        let api_id = required_unicode("TELEGRAM_API_ID")?
            .parse::<i32>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or(ConfigError::InvalidValue("TELEGRAM_API_ID"))?;
        let api_hash = SensitiveString::new(required_unicode("TELEGRAM_API_HASH")?);
        let use_test_dc = parse_bool("TDLIB_USE_TEST_DC", false)?;
        let expected_user_id = optional_positive_i64("TELEGRAM_EXPECTED_USER_ID")?;
        let idle_timeout_ms =
            optional_u64("TELEGRAM_IDLE_TIMEOUT_MS")?.unwrap_or(DEFAULT_IDLE_TIMEOUT_MS);
        if idle_timeout_ms == 0 {
            return Err(ConfigError::InvalidValue("TELEGRAM_IDLE_TIMEOUT_MS"));
        }

        Ok(Self {
            database_directory: ownership.canonical_database_directory().to_owned(),
            files_directory,
            database_key_file,
            native_library,
            native_sha256,
            native_bytes,
            use_test_dc,
            api_id,
            api_hash,
            expected_user_id,
            idle_timeout: Duration::from_millis(idle_timeout_ms),
        })
    }

    pub fn load_native(&self) -> Result<NativeTdJson, ConfigError> {
        verify_native_artifact(&self.native_library, self.native_bytes, &self.native_sha256)?;
        // SAFETY: the exact file bytes were checked against target-specific pinned
        // provenance immediately before resolving the fixed TDJSON C ABI symbols.
        unsafe { NativeTdJson::load(&self.native_library) }.map_err(|_| ConfigError::NativeLoad)
    }

    pub fn load_database_key(&self) -> Result<DatabaseKey, ConfigError> {
        DatabaseKey::load(DatabaseKeySource::Base64FileSecret(
            self.database_key_file.clone(),
        ))
        .map_err(ConfigError::DatabaseKey)
    }

    pub fn tdlib_parameters(&self) -> TdlibParameters {
        TdlibParameters {
            use_test_dc: self.use_test_dc,
            database_directory: self.database_directory.clone(),
            files_directory: self.files_directory.clone(),
            use_file_database: true,
            use_chat_info_database: true,
            use_message_database: true,
            use_secret_chats: true,
            api_id: self.api_id,
            api_hash: self.api_hash.clone(),
            system_language_code: "ru".to_owned(),
            device_model: "telegramd".to_owned(),
            system_version: env::consts::OS.to_owned(),
            application_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }

    pub fn expected_user_id(&self) -> Option<i64> {
        self.expected_user_id
    }

    pub fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn verify_native_artifact(
    path: &Path,
    expected_bytes: u64,
    expected_hash: &str,
) -> Result<(), ConfigError> {
    if !path.is_absolute() {
        return Err(ConfigError::PathMustBeAbsolute("TDJSON_LIBRARY_PATH"));
    }
    let mut file = File::open(path).map_err(|error| ConfigError::NativeRead(error.kind()))?;
    let metadata = file
        .metadata()
        .map_err(|error| ConfigError::NativeRead(error.kind()))?;
    if !metadata.is_file() || metadata.len() != expected_bytes {
        return Err(ConfigError::NativeMismatch);
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| ConfigError::NativeRead(error.kind()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if format!("{:x}", hasher.finalize()) != expected_hash {
        return Err(ConfigError::NativeMismatch);
    }
    Ok(())
}

fn required_json_string(value: Option<&Value>) -> Result<&str, ConfigError> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(ConfigError::InvalidNativePin)
}

fn required_unicode(name: &'static str) -> Result<String, ConfigError> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(ConfigError::Missing(name))
}

fn required_path(name: &'static str) -> Result<PathBuf, ConfigError> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(ConfigError::Missing(name))
}

fn optional_u64(name: &'static str) -> Result<Option<u64>, ConfigError> {
    match env::var(name) {
        Err(env::VarError::NotPresent) => Ok(None),
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => value
            .parse()
            .map(Some)
            .map_err(|_| ConfigError::InvalidValue(name)),
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError::InvalidValue(name)),
    }
}

fn optional_positive_i64(name: &'static str) -> Result<Option<i64>, ConfigError> {
    let value = optional_u64(name)?;
    value
        .map(|value| {
            i64::try_from(value)
                .ok()
                .filter(|value| *value > 0)
                .ok_or(ConfigError::InvalidValue(name))
        })
        .transpose()
}

fn parse_bool(name: &'static str, default: bool) -> Result<bool, ConfigError> {
    match env::var(name) {
        Err(env::VarError::NotPresent) => Ok(default),
        Ok(value) => match value.as_str() {
            "1" | "true" | "yes" => Ok(true),
            "0" | "false" | "no" | "" => Ok(false),
            _ => Err(ConfigError::InvalidValue(name)),
        },
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError::InvalidValue(name)),
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Missing(&'static str),
    InvalidValue(&'static str),
    PathMustBeAbsolute(&'static str),
    InvalidNativePin,
    NativeRead(io::ErrorKind),
    NativeMismatch,
    NativeLoad,
    DatabaseKey(DatabaseKeyError),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(name) => write!(formatter, "required configuration {name} is missing"),
            Self::InvalidValue(name) => write!(formatter, "configuration {name} is invalid"),
            Self::PathMustBeAbsolute(name) => {
                write!(formatter, "configuration {name} must be an absolute path")
            }
            Self::InvalidNativePin => formatter.write_str("pinned native provenance is invalid"),
            Self::NativeRead(kind) => {
                write!(formatter, "can't read pinned TDJSON artifact: {kind:?}")
            }
            Self::NativeMismatch => formatter.write_str("TDJSON artifact does not match the pin"),
            Self::NativeLoad => formatter.write_str("pinned TDJSON artifact failed to load"),
            Self::DatabaseKey(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ConfigError {}
