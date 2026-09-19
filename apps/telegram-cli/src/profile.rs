//! Owner-only profile configuration and lazy singleton daemon startup.

use std::collections::BTreeMap;
use std::env;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use telegram_protocol::{ClientErrorCode, DaemonRequest};
use zeroize::{Zeroize, Zeroizing};

use crate::{CliError, read_tty_secret, read_tty_visible};

const ENV_NAMES: &[&str] = &[
    "TELEGRAM_API_ID",
    "TELEGRAM_API_HASH",
    "TDLIB_DATABASE_DIR",
    "TDLIB_FILES_DIR",
    "TDLIB_DATABASE_KEY_FILE",
    "TDJSON_LIBRARY_PATH",
    "TDLIB_USE_TEST_DC",
    "TELEGRAM_EXPECTED_USER_ID",
    "TELEGRAM_IDLE_TIMEOUT_MS",
    "TELEGRAM_RISK_SCOPES",
    "TELEGRAM_APPROVAL_PUBLIC_KEY_HEX",
];

fn invalid() -> CliError {
    CliError::new(ClientErrorCode::InvalidConfiguration)
}

fn directory(profile: &str) -> Result<PathBuf, CliError> {
    if !telegram_client::valid_name(profile) {
        return Err(CliError::new(ClientErrorCode::InvalidProfile));
    }
    let root = env::var_os("TELEGRAM_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/telegram-cli"))
        })
        .ok_or_else(invalid)?;
    if !root.is_absolute() || root.to_str().is_none() {
        return Err(invalid());
    }
    Ok(root.join(profile))
}

fn private_directory(path: &Path) -> Result<(), CliError> {
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|_| invalid())?;
    let metadata = fs::symlink_metadata(path).map_err(|_| invalid())?;
    if !metadata.is_dir()
        || metadata.uid() != telegram_client::effective_uid()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err(invalid());
    }
    Ok(())
}

fn create_private(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| invalid())?;
    if file.write_all(bytes).and_then(|_| file.sync_all()).is_err() {
        let _ = fs::remove_file(path);
        return Err(invalid());
    }
    Ok(())
}

struct Settings(BTreeMap<String, String>);
impl Drop for Settings {
    fn drop(&mut self) {
        for value in self.0.values_mut() {
            value.zeroize();
        }
    }
}

fn load(path: &Path) -> Result<Settings, CliError> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                CliError::new(ClientErrorCode::ProfileNotConfigured)
            } else {
                invalid()
            }
        })?;
    let metadata = file.metadata().map_err(|_| invalid())?;
    if !metadata.is_file()
        || metadata.uid() != telegram_client::effective_uid()
        || metadata.mode() & 0o777 != 0o600
        || metadata.nlink() != 1
        || metadata.len() > 16_384
    {
        return Err(invalid());
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() > 16_384 {
        return Err(invalid());
    }
    let settings = Settings(serde_json::from_slice(&bytes).map_err(|_| invalid())?);
    validate(&settings.0)?;
    Ok(settings)
}

fn validate(settings: &BTreeMap<String, String>) -> Result<(), CliError> {
    if settings
        .iter()
        .any(|(name, value)| !ENV_NAMES.contains(&name.as_str()) || value.contains('\0'))
    {
        return Err(invalid());
    }
    if settings
        .get("TELEGRAM_API_ID")
        .and_then(|v| v.parse::<i32>().ok())
        .is_none_or(|v| v <= 0)
        || settings
            .get("TELEGRAM_API_HASH")
            .is_none_or(|v| v.len() != 32 || !v.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(invalid());
    }
    for name in [
        "TDLIB_DATABASE_DIR",
        "TDLIB_FILES_DIR",
        "TDLIB_DATABASE_KEY_FILE",
        "TDJSON_LIBRARY_PATH",
    ] {
        if settings
            .get(name)
            .is_none_or(|v| !Path::new(v).is_absolute())
        {
            return Err(invalid());
        }
    }
    Ok(())
}

pub fn setup(profile: &str, import_env: bool) -> Result<(), CliError> {
    let dir = directory(profile)?;
    // Existing profiles are immutable through setup: a rerun cannot replace an account or key.
    if dir.join("profile.json").exists() {
        load(&dir.join("profile.json"))?;
        return Ok(());
    }
    let _owner_tty = crate::open_tty()?;
    crate::install_signal_handlers()?;
    private_directory(dir.parent().ok_or_else(invalid)?)?;
    private_directory(&dir)?;
    let mut settings = Settings(BTreeMap::new());
    if import_env {
        for name in ENV_NAMES {
            if let Ok(value) = env::var(name)
                && !value.is_empty()
            {
                settings.0.insert((*name).to_owned(), value);
            }
        }
        let extended_policy = settings
            .0
            .get("TELEGRAM_RISK_SCOPES")
            .is_some_and(|scopes| scopes != "read")
            || settings.0.contains_key("TELEGRAM_APPROVAL_PUBLIC_KEY_HEX");
        if extended_policy
            && !crate::read_yes_no(
                "Импортировать также расширенные разрешения текущей конфигурации? [y/N]: ",
            )?
        {
            settings.0.remove("TELEGRAM_APPROVAL_PUBLIC_KEY_HEX");
            settings
                .0
                .insert("TELEGRAM_RISK_SCOPES".into(), "read".into());
        }
    } else {
        settings.0.insert(
            "TELEGRAM_API_ID".into(),
            read_tty_secret("API ID (my.telegram.org): ")?.into_inner(),
        );
        settings.0.insert(
            "TELEGRAM_API_HASH".into(),
            read_tty_secret("API hash: ")?.into_inner(),
        );
        let installed = daemon_binary()?
            .parent()
            .and_then(Path::parent)
            .ok_or_else(invalid)?
            .join("lib/telegram-cli")
            .join(if cfg!(target_os = "macos") {
                "libtdjson.dylib"
            } else {
                "libtdjson.so"
            });
        let native = if installed.is_file() {
            installed
        } else {
            PathBuf::from(read_tty_visible("Абсолютный путь к pinned libtdjson: ")?.into_inner())
        };
        let native = fs::canonicalize(native).map_err(|_| invalid())?;
        settings.0.insert(
            "TDJSON_LIBRARY_PATH".into(),
            native.to_str().ok_or_else(invalid)?.to_owned(),
        );
        for (name, child) in [
            ("TDLIB_DATABASE_DIR", "database"),
            ("TDLIB_FILES_DIR", "files"),
        ] {
            let path = dir.join(child);
            private_directory(&path)?;
            settings
                .0
                .insert(name.into(), path.to_str().ok_or_else(invalid)?.to_owned());
        }
        let key = dir.join("database-key");
        settings.0.insert(
            "TDLIB_DATABASE_KEY_FILE".into(),
            key.to_str().ok_or_else(invalid)?.to_owned(),
        );
        validate(&settings.0)?;
        check_native(&settings.0)?;
        ensure_key(&key)?;
    }
    validate(&settings.0)?;
    check_native(&settings.0)?;
    let bytes = Zeroizing::new(serde_json::to_vec(&settings.0).map_err(|_| invalid())?);
    create_private(&dir.join("profile.json"), &bytes)
}

fn ensure_key(path: &Path) -> Result<(), CliError> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    // Resume a setup interrupted after key creation; never replace an existing key.
    if let Ok(mut file) = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
    {
        let metadata = file.metadata().map_err(|_| invalid())?;
        if !metadata.is_file()
            || metadata.uid() != telegram_client::effective_uid()
            || metadata.mode() & 0o777 != 0o600
            || metadata.nlink() != 1
            || metadata.len() != 48
        {
            return Err(invalid());
        }
        let mut key = Zeroizing::new([0_u8; 48]);
        file.read_exact(&mut *key).map_err(|_| invalid())?;
        if key.iter().all(|byte| ALPHABET.contains(byte)) {
            return Ok(());
        }
        return Err(invalid());
    }
    let mut random = Zeroizing::new([0_u8; 48]);
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut *random))
        .map_err(|_| invalid())?;
    for byte in random.iter_mut() {
        *byte = ALPHABET[(*byte & 63) as usize];
    }
    create_private(path, &*random)
}

fn daemon_log(dir: &Path) -> Result<File, CliError> {
    private_directory(dir)?;
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(dir.join("daemon.log"))
        .map_err(|_| invalid())?;
    let metadata = file.metadata().map_err(|_| invalid())?;
    if !metadata.is_file()
        || metadata.uid() != telegram_client::effective_uid()
        || metadata.mode() & 0o777 != 0o600
        || metadata.nlink() != 1
    {
        return Err(invalid());
    }
    if metadata.len() >= 64 * 1024 {
        file.set_len(0).map_err(|_| invalid())?;
    }
    Ok(file)
}

fn check_native(settings: &BTreeMap<String, String>) -> Result<(), CliError> {
    let path = settings.get("TDJSON_LIBRARY_PATH").ok_or_else(invalid)?;
    let status = Command::new(daemon_binary()?)
        .arg("--check-native")
        .arg(path)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| invalid())?;
    if status.success() {
        Ok(())
    } else {
        Err(invalid())
    }
}

pub fn daemon_binary() -> Result<PathBuf, CliError> {
    env::current_exe()
        .and_then(fs::canonicalize)
        .map(|path| path.with_file_name("telegramd"))
        .map_err(|_| CliError::new(ClientErrorCode::DaemonStartFailed))
}

pub fn ensure_started(profile: &str) -> Result<(), CliError> {
    let socket = telegram_client::socket_path(profile).map_err(CliError::new)?;
    if telegram_client::daemon_reachable(profile).map_err(CliError::new)? {
        return Ok(());
    }
    let dir = directory(profile)?;
    let settings = load(&dir.join("profile.json"))?;
    let log = daemon_log(&dir)?;
    let mut command = Command::new(daemon_binary()?);
    // Avoid inheriting unrelated credentials and daemon settings from the agent environment.
    command
        .env_clear()
        .envs(&settings.0)
        .env("TELEGRAM_PROFILE", profile)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log))
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|_| CliError::new(ClientErrorCode::DaemonStartFailed))?;
    drop(settings);
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        if telegram_client::validate_socket(&socket).is_ok() {
            let options = telegram_client::ExchangeOptions::new(
                Duration::from_millis(250),
                telegram_client::ResponseFraming::BoundedLine { max_bytes: 16_384 },
            );
            if telegram_client::exchange_with_options(profile, &DaemonRequest::LoginStatus, options)
                .is_ok()
            {
                // Detached child lives until the daemon's existing idle-close policy expires.
                return Ok(());
            }
        }
        if Instant::now() >= deadline {
            return Err(CliError::new(ClientErrorCode::DaemonStartFailed));
        }
        // A concurrent CLI may win the DB lock; wait for its socket even if our child exits.
        let _ = child.try_wait();
        if crate::RECEIVED_SIGNAL.load(std::sync::atomic::Ordering::Relaxed) != 0 {
            return Err(CliError::new(ClientErrorCode::Cancelled));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub fn doctor(profile: &str) -> serde_json::Value {
    let configured = directory(profile).and_then(|dir| load(&dir.join("profile.json")));
    let config_error = configured.as_ref().err().map(|error| error.code);
    let native_verified = configured
        .as_ref()
        .is_ok_and(|settings| check_native(&settings.0).is_ok());
    let daemon_available = daemon_binary().is_ok_and(|path| path.is_file());
    let socket_available = telegram_client::socket_path(profile)
        .is_ok_and(|path| telegram_client::validate_socket(&path).is_ok());
    serde_json::json!({"profile": profile, "configured": configured.is_ok(),
        "configuration_error": config_error, "native_verified": native_verified, "daemon_installed": daemon_available,
        "socket_present": socket_available, "daemon_log": directory(profile).ok().map(|dir| dir.join("daemon.log")),
        "next_action": if config_error.is_some() { "run_setup_in_owner_terminal" }
            else if !daemon_available { "install_telegramd_beside_cli" }
            else if !native_verified { "restore_pinned_tdjson_artifact" } else { "check_login_status" }})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn setup_reuses_a_valid_key_after_interrupted_profile_creation() {
        let dir = env::temp_dir().join(format!("telegram-key-{}", std::process::id()));
        private_directory(&dir).unwrap();
        let key = dir.join("database-key");
        ensure_key(&key).unwrap();
        let original = fs::read(&key).unwrap();
        ensure_key(&key).unwrap();
        assert_eq!(fs::read(&key).unwrap(), original);
        fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(ensure_key(&key).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn protected_profile_roundtrip_rejects_public_files_and_symlinks() {
        let dir = env::temp_dir().join(format!(
            "telegram-profile-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        private_directory(&dir).unwrap();
        let path = dir.join("profile.json");
        let settings = BTreeMap::from([
            ("TELEGRAM_API_ID".to_owned(), "1".to_owned()),
            ("TELEGRAM_API_HASH".to_owned(), "a".repeat(32)),
            ("TDLIB_DATABASE_DIR".to_owned(), "/tmp/test-db".to_owned()),
            ("TDLIB_FILES_DIR".to_owned(), "/tmp/test-files".to_owned()),
            (
                "TDLIB_DATABASE_KEY_FILE".to_owned(),
                "/tmp/test-key".to_owned(),
            ),
            (
                "TDJSON_LIBRARY_PATH".to_owned(),
                "/tmp/test-native".to_owned(),
            ),
        ]);
        create_private(&path, &serde_json::to_vec(&settings).unwrap()).unwrap();
        assert!(load(&path).is_ok());
        assert!(create_private(&path, b"overwrite").is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load(&path).is_err());
        symlink(&path, dir.join("link")).unwrap();
        assert!(load(&dir.join("link")).is_err());
        fs::remove_dir_all(dir).unwrap();
        assert!(!telegram_client::valid_name(".."));
    }
}
