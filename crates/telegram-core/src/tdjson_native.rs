//! Узкая Unix FFI-граница для runtime loading exact `tdjson` artifact.

use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::mem;
use std::path::Path;
use std::ptr::NonNull;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use crate::transport::{BackendError, TdJsonBackend};

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type SendFn = unsafe extern "C" fn(*mut c_void, *const c_char);
type ReceiveFn = unsafe extern "C" fn(*mut c_void, c_double) -> *const c_char;
type DestroyFn = unsafe extern "C" fn(*mut c_void);

#[cfg_attr(target_os = "linux", link(name = "dl"))]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}

const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 4;

struct DynamicLibrary {
    handle: NonNull<c_void>,
}

impl DynamicLibrary {
    fn open(path: &Path) -> Result<Self, BackendError> {
        #[cfg(not(unix))]
        compile_error!("NativeTdJson supports the declared macOS/Linux targets only");
        let path = path.canonicalize().map_err(|error| {
            BackendError::new(format!("tdjson library is unavailable: {error}"))
        })?;
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| BackendError::new("tdjson library path contains NUL"))?;
        // SAFETY: path is a valid NUL-terminated string; the handle is checked.
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
        NonNull::new(handle)
            .map(|handle| Self { handle })
            .ok_or_else(last_dynamic_error)
    }

    fn symbol<T: Copy>(&self, name: &'static [u8]) -> Result<T, BackendError> {
        debug_assert_eq!(name.last(), Some(&0));
        // SAFETY: handle is live and name is a static NUL-terminated symbol.
        let address = unsafe { dlsym(self.handle.as_ptr(), name.as_ptr().cast()) };
        let address = NonNull::new(address).ok_or_else(last_dynamic_error)?;
        if mem::size_of::<T>() != mem::size_of::<*mut c_void>() {
            return Err(BackendError::new(
                "tdjson symbol has unexpected pointer size",
            ));
        }
        // SAFETY: the exact TDJSON C ABI function type is fixed by td_json_client.h.
        Ok(unsafe { mem::transmute_copy(&address.as_ptr()) })
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        // SAFETY: this object uniquely owns the live handle.
        let _ = unsafe { dlclose(self.handle.as_ptr()) };
    }
}

fn last_dynamic_error() -> BackendError {
    // SAFETY: dlerror returns either null or a NUL-terminated thread-local string.
    let pointer = unsafe { dlerror() };
    if pointer.is_null() {
        BackendError::new("dynamic loader returned no diagnostic")
    } else {
        // SAFETY: non-null dlerror result is valid until the next loader call.
        let message = unsafe { CStr::from_ptr(pointer) }.to_string_lossy();
        BackendError::new(message.into_owned())
    }
}

/// Direct TDJSON backend. Library path приходит от trusted daemon/packaging
/// configuration; transport requests или secrets не участвуют в loading.
pub struct NativeTdJson {
    client: NonNull<c_void>,
    send: SendFn,
    receive: ReceiveFn,
    destroy: DestroyFn,
    _library: DynamicLibrary,
}

impl NativeTdJson {
    /// # Safety
    ///
    /// `path` должен указывать на проверенный TDJSON artifact с exact ABI
    /// функций из `td_json_client.h`. Загрузка произвольной библиотеки может
    /// нарушить memory-safety до того, как Rust успеет проверить response.
    pub unsafe fn load(path: impl AsRef<Path>) -> Result<Self, BackendError> {
        let library = DynamicLibrary::open(path.as_ref())?;
        let create: CreateFn = library.symbol(b"td_json_client_create\0")?;
        let send = library.symbol(b"td_json_client_send\0")?;
        let receive = library.symbol(b"td_json_client_receive\0")?;
        let destroy = library.symbol(b"td_json_client_destroy\0")?;
        // SAFETY: create was resolved with the exact TDJSON C ABI signature.
        let client = NonNull::new(unsafe { create() })
            .ok_or_else(|| BackendError::new("td_json_client_create returned null"))?;
        Ok(Self {
            client,
            send,
            receive,
            destroy,
            _library: library,
        })
    }
}

// SAFETY: the opaque client is moved into one receive thread before use. The
// public API never exposes the pointer and does not implement Sync.
unsafe impl Send for NativeTdJson {}

impl TdJsonBackend for NativeTdJson {
    fn send(&mut self, request: &str) -> Result<(), BackendError> {
        let request = CString::new(request)
            .map_err(|_| BackendError::new("serialized TDJSON request contains NUL"))?;
        // SAFETY: client and function pointer stay valid while _library is held.
        unsafe { (self.send)(self.client.as_ptr(), request.as_ptr()) };
        Ok(())
    }

    fn receive(&mut self, timeout: Duration) -> Result<Option<String>, BackendError> {
        // SAFETY: client and function pointer stay valid while _library is held.
        let response = unsafe { (self.receive)(self.client.as_ptr(), timeout.as_secs_f64()) };
        if response.is_null() {
            return Ok(None);
        }
        // SAFETY: TDJSON guarantees a NUL-terminated result valid until the next receive.
        let response = unsafe { CStr::from_ptr(response) }
            .to_str()
            .map_err(|_| BackendError::new("TDJSON response is not UTF-8"))?;
        Ok(Some(response.to_owned()))
    }
}

impl Drop for NativeTdJson {
    fn drop(&mut self) {
        // SAFETY: this object uniquely owns the TDJSON client.
        unsafe { (self.destroy)(self.client.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::authorization::{AuthorizationMachine, AuthorizationStep, SensitiveString};
    use crate::database_key::{DatabaseKey, DatabaseKeySource, TdlibParameters};
    use crate::runtime::CoreRuntime;

    use super::*;

    #[test]
    #[ignore = "requires TDJSON_LIBRARY_PATH pointing to a pinned local artifact"]
    fn pinned_native_no_client_call_uses_real_tdjson_transport() {
        let path = env::var_os("TDJSON_LIBRARY_PATH").expect("TDJSON_LIBRARY_PATH");
        let runtime = start_native(&path);
        let response = runtime
            .transport()
            .call(
                json!({"@type":"checkAuthenticationPassword","password":"TDLIB_SECRET_CANARY_DO_NOT_LOG"}),
                Duration::from_secs(5),
            )
            .unwrap();
        assert_eq!(response["@type"], "error");
        runtime.shutdown().unwrap();
    }

    #[test]
    #[ignore = "requires TDJSON_LIBRARY_PATH pointing to a pinned local artifact"]
    fn pinned_native_wrong_or_missing_database_key_is_fail_closed() {
        let library = env::var_os("TDJSON_LIBRARY_PATH").expect("TDJSON_LIBRARY_PATH");
        let root = temporary_path("wrong-key");
        let database = root.join("database");
        let files = root.join("files");
        let key_file = root.join("key");
        fs::create_dir_all(&database).unwrap();
        fs::create_dir_all(&files).unwrap();
        write_secret(&key_file, b"synthetic-correct-key");

        let mut runtime = start_native(&library);
        let correct = parameters_request(&runtime, &key_file, &database, &files);
        assert_call_type(&runtime, correct, "ok");
        wait_authorization(&mut runtime, "authorizationStateWaitPhoneNumber");
        assert_call_type(&runtime, json!({"@type":"close"}), "ok");
        wait_authorization(&mut runtime, "authorizationStateClosed");
        runtime.shutdown().unwrap();
        let before = directory_snapshot(&database);
        assert!(!before.is_empty());

        write_secret(&key_file, b"synthetic-wrong-key");
        let mut runtime = start_native(&library);
        let wrong = parameters_request(&runtime, &key_file, &database, &files);
        let response = runtime
            .transport()
            .call(wrong, Duration::from_secs(5))
            .unwrap();
        assert_eq!(response["@type"], "error");
        assert_eq!(response["code"], 401);
        let deadline = std::time::Instant::now() + Duration::from_millis(100);
        while runtime.next_event_until(deadline).is_ok() {
            assert_ne!(
                runtime.state().authorization().unwrap().value["@type"],
                "authorizationStateWaitPhoneNumber"
            );
        }
        runtime.shutdown().unwrap();
        assert_eq!(directory_snapshot(&database), before);

        fs::remove_file(&key_file).unwrap();
        assert!(DatabaseKey::load(DatabaseKeySource::FileSecret(key_file)).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires protected .env.local loader and pinned TDJSON artifact"]
    fn live_returning_session_reaches_ready_without_login_input() {
        assert_eq!(
            env::var("TELEGRAM_CORE_LIVE_RETURNING").ok().as_deref(),
            Some("1")
        );
        let library = env::var_os("TDJSON_LIBRARY_PATH").expect("TDJSON_LIBRARY_PATH");
        let mut runtime = start_native(&library);
        let key_path = PathBuf::from(required_env("TDLIB_DATABASE_KEY_FILE"));
        let key = DatabaseKey::load(DatabaseKeySource::Base64FileSecret(key_path)).unwrap();
        let request = submit_parameters(
            &runtime,
            &key,
            TdlibParameters {
                use_test_dc: parse_bool_env("TDLIB_USE_TEST_DC"),
                database_directory: PathBuf::from(required_env("TDLIB_DATABASE_DIR")),
                files_directory: PathBuf::from(required_env("TDLIB_FILES_DIR")),
                use_file_database: true,
                use_chat_info_database: true,
                use_message_database: true,
                use_secret_chats: false,
                api_id: required_env("TELEGRAM_API_ID")
                    .parse()
                    .unwrap_or_else(|_| panic!("TELEGRAM_API_ID must be an integer")),
                api_hash: SensitiveString::new(required_env("TELEGRAM_API_HASH")),
                system_language_code: "en".to_owned(),
                device_model: "telegram-core-live-test".to_owned(),
                system_version: "test".to_owned(),
                application_version: "test".to_owned(),
            },
        );
        let response = runtime
            .transport()
            .call(request, Duration::from_secs(10))
            .unwrap();
        assert_eq!(
            response["@type"],
            "ok",
            "setTdlibParameters failed with code {}: {}",
            response["code"],
            response["message"].as_str().unwrap_or("missing message")
        );
        wait_authorization(&mut runtime, "authorizationStateReady");
        assert_call_type(&runtime, json!({"@type":"getMe"}), "user");
        assert_call_type(&runtime, json!({"@type":"close"}), "ok");
        wait_authorization(&mut runtime, "authorizationStateClosed");
        runtime.shutdown().unwrap();
    }

    fn start_native(library: &std::ffi::OsStr) -> CoreRuntime {
        // SAFETY: caller supplies the locally hash-verified pinned artifact.
        let backend = unsafe { NativeTdJson::load(library) }.unwrap();
        CoreRuntime::start(backend, std::time::Instant::now() + Duration::from_secs(5)).unwrap()
    }

    fn parameters_request(
        runtime: &CoreRuntime,
        key_file: &Path,
        database: &Path,
        files: &Path,
    ) -> serde_json::Value {
        let key = DatabaseKey::load(DatabaseKeySource::FileSecret(key_file.to_owned())).unwrap();
        submit_parameters(
            runtime,
            &key,
            TdlibParameters {
                use_test_dc: true,
                database_directory: database.to_owned(),
                files_directory: files.to_owned(),
                use_file_database: true,
                use_chat_info_database: true,
                use_message_database: true,
                use_secret_chats: false,
                api_id: 1,
                api_hash: SensitiveString::new("synthetic-api-hash"),
                system_language_code: "en".to_owned(),
                device_model: "telegram-core-test".to_owned(),
                system_version: "test".to_owned(),
                application_version: "test".to_owned(),
            },
        )
    }

    fn submit_parameters(
        runtime: &CoreRuntime,
        key: &DatabaseKey,
        parameters: TdlibParameters,
    ) -> serde_json::Value {
        let mut machine = AuthorizationMachine::default();
        let generation = match machine
            .observe_state(&runtime.state().authorization().unwrap().value)
            .unwrap()
        {
            AuthorizationStep::ParametersRequired { generation } => generation,
            step => panic!("expected parameters, got {step:?}"),
        };
        machine
            .submit_parameters(generation, parameters, key)
            .unwrap()
            .into_value()
    }

    fn wait_authorization(runtime: &mut CoreRuntime, expected: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while runtime.state().authorization().unwrap().value["@type"] != expected {
            runtime.next_event_until(deadline).unwrap();
        }
    }

    fn assert_call_type(runtime: &CoreRuntime, request: serde_json::Value, expected: &str) {
        let response = runtime
            .transport()
            .call(request, Duration::from_secs(10))
            .unwrap();
        assert_eq!(response["@type"], expected);
    }

    fn temporary_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "telegram-core-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn write_secret(path: &Path, value: &[u8]) {
        fs::write(path, value).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    fn directory_snapshot(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file())
            .map(|file| {
                (
                    PathBuf::from(file.file_name().unwrap()),
                    fs::read(file).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        files.sort_by(|left, right| left.0.cmp(&right.0));
        files
    }

    fn required_env(name: &'static str) -> String {
        env::var(name).unwrap_or_else(|_| panic!("missing {name}"))
    }

    fn parse_bool_env(name: &'static str) -> bool {
        match required_env(name).trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" | "" => false,
            _ => panic!("{name} must be true or false"),
        }
    }
}
