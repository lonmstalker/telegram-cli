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
        let path = path
            .canonicalize()
            .map_err(|error| BackendError::new(format!("tdjson library is unavailable: {error}")))?;
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

    use serde_json::json;

    use crate::transport::TdJsonTransport;

    use super::*;

    #[test]
    #[ignore = "requires TDJSON_LIBRARY_PATH pointing to a pinned local artifact"]
    fn pinned_native_no_client_call_uses_real_tdjson_transport() {
        let path = env::var_os("TDJSON_LIBRARY_PATH").expect("TDJSON_LIBRARY_PATH");
        // SAFETY: test command points at the locally hash-verified pinned artifact.
        let backend = unsafe { NativeTdJson::load(path) }.unwrap();
        let (transport, _events) = TdJsonTransport::start(backend).unwrap();
        let response = transport
            .call(
                json!({"@type": "getOption", "name": "version"}),
                Duration::from_secs(5),
            )
            .unwrap();
        assert_eq!(response["@type"], "optionValueString");
        assert_eq!(response["value"], "1.8.66");
        transport.shutdown().unwrap();
    }
}
