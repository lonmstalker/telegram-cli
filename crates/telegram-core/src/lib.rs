//! Владение TDJSON transport, ordered state и Telegram workflows.

#![deny(unsafe_code)]

pub mod authorization;
#[cfg(unix)]
#[allow(unsafe_code)]
pub mod database_key;
pub mod raw_api;
pub mod reducer;
pub mod registry;
pub mod runtime;
pub mod schema;
pub mod transport;

#[allow(unsafe_code)]
mod tdjson_native;

pub use tdjson_native::NativeTdJson;
